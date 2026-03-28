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
use tracing::{error, info, warn};

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

    // ── 3. Lifecycle loop: wait for promotion, run workers, handle demotion ──
    //
    // When demoted, worker tasks exit cleanly (realtime consumer drops its Kafka
    // consumer, releasing rt-metrics partitions). On re-promotion, workers are
    // restarted fresh with new consumers.
    let state_lifecycle = Arc::clone(&state);
    let lifecycle_handle = tokio::spawn(async move {
        loop {
            // Wait until we become Active
            info!("Waiting for leader election (starting as STANDBY)...");
            leader::wait_until_active(Arc::clone(&state_lifecycle)).await;
            info!("Promoted to ACTIVE -- starting worker tasks");

            // Start worker tasks (only when Active)
            let state_rt = Arc::clone(&state_lifecycle);
            let realtime_handle = tokio::spawn(async move {
                realtime::realtime_consumer(state_rt).await;
            });

            let state_bf = Arc::clone(&state_lifecycle);
            let backfill_handle = tokio::spawn(async move {
                backfill::backfill_engine(state_bf).await;
            });

            let state_hm = Arc::clone(&state_lifecycle);
            let health_handle = tokio::spawn(async move {
                health::health_monitor(state_hm).await;
            });

            // Wait for any worker to exit (realtime exits on demotion)
            // or for demotion to be detected
            tokio::select! {
                _ = realtime_handle => {
                    warn!("Realtime consumer exited (likely demoted)");
                }
                _ = backfill_handle => {
                    warn!("Backfill engine exited");
                }
                _ = health_handle => {
                    warn!("Health monitor exited");
                }
            }

            // Check if we were demoted
            let role = state_lifecycle.get_role().await;
            if role == state::HaRole::Standby {
                info!("Demoted to STANDBY -- workers stopped, waiting for re-promotion");
                // Loop back to wait_until_active
            } else {
                error!("Worker task exited while still Active -- restarting workers");
            }
        }
    });

    // ── 4. Wait for critical tasks ──
    tokio::select! {
        _ = http_handle => error!("HTTP server exited unexpectedly"),
        _ = leader_handle => error!("Leader election loop exited unexpectedly"),
        _ = lifecycle_handle => error!("Lifecycle loop exited unexpectedly"),
    }

    Ok(())
}
