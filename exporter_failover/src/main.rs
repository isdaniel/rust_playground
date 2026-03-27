// ============================================================================
// Exporter Service with Active/Standby Failover
//
// Architecture:
//   - Leader Election via Kafka Consumer Group (single-partition topic)
//   - Realtime Consumer: Kafka -> Cloud streaming with adaptive micro-batch
//   - Backfill Engine: dormant until WAN recovery, rate-limited replay
//   - Health Monitor: connection state machine (Connected/Disconnected/Backfilling)
//   - HTTP API: health, status, and metrics endpoints
// ============================================================================

mod api;
mod backfill;
mod config;
mod health;
mod leader;
mod models;
mod realtime;
mod state;

use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("exporter_failover=info".parse().unwrap())
                .add_directive("rdkafka=warn".parse().unwrap()),
        )
        .with_target(true)
        .init();

    // Load config: try file first, fallback to env
    let config = match std::env::args().nth(1) {
        Some(path) => {
            info!(path = %path, "Loading config from file");
            config::ExporterConfig::from_file(&path)?
        }
        None => {
            info!("Loading config from environment variables");
            config::ExporterConfig::from_env()?
        }
    };

    info!(
        fab = %config.fab_id,
        instance = %config.instance_id,
        brokers = ?config.kafka_brokers,
        "Exporter service starting..."
    );

    // Create shared state
    let state = state::SharedState::new(config);

    // ── 1. Start HTTP server (runs on both Active and Standby) ──
    let state_http = Arc::clone(&state);
    let http_handle = tokio::spawn(async move {
        api::run_http_server(state_http).await;
    });

    // ── 2. Start Leader Election ──
    let state_leader = Arc::clone(&state);
    let leader_handle = tokio::spawn(async move {
        leader::leader_election_loop(state_leader).await;
    });

    // ── 3. Wait until we become Active ──
    info!("Waiting for leader election (starting as STANDBY)...");
    leader::wait_until_active(Arc::clone(&state)).await;
    info!("Promoted to ACTIVE -- starting worker tasks");

    // ── 4. Start worker tasks (only when Active) ──
    let state_rt = Arc::clone(&state);
    let realtime_handle = tokio::spawn(async move {
        realtime::realtime_consumer(state_rt).await;
    });

    let state_bf = Arc::clone(&state);
    let backfill_handle = tokio::spawn(async move {
        backfill::backfill_engine(state_bf).await;
    });

    let state_hm = Arc::clone(&state);
    let health_handle = tokio::spawn(async move {
        health::health_monitor(state_hm).await;
    });

    // ── 5. Wait for any task to exit (should not happen) ──
    tokio::select! {
        _ = http_handle => error!("HTTP server exited unexpectedly"),
        _ = leader_handle => error!("Leader election loop exited unexpectedly"),
        _ = realtime_handle => error!("Realtime consumer exited unexpectedly"),
        _ = backfill_handle => error!("Backfill engine exited unexpectedly"),
        _ = health_handle => error!("Health monitor exited unexpectedly"),
    }

    Ok(())
}
