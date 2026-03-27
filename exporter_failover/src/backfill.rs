/// Backfill Engine: dormant normally, wakes up after WAN recovery.
///
/// Key behaviors:
/// - Uses an independent consumer group ("backfill") so it doesn't affect realtime offsets
/// - Token bucket rate limiter: only uses configured % of bandwidth
/// - Priority sort: alarm > key > raw
/// - Automatically stops when caught up, transitions back to CONNECTED
use std::sync::Arc;
use std::time::Duration;

use rdkafka::config::ClientConfig;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message;
use tracing::{error, info, warn};

use crate::models::MetricRecord;
use crate::state::{ConnectionState, SharedState};

pub async fn backfill_engine(state: Arc<SharedState>) {
    let brokers = state.config.kafka_brokers.join(",");

    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "backfill")
        .set("bootstrap.servers", &brokers)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest")
        .set("session.timeout.ms", "10000")
        .set("fetch.min.bytes", "65536")
        .set("fetch.wait.max.ms", "1000")
        .create()
        .expect("Failed to create backfill consumer");

    consumer
        .subscribe(&["metrics.alarm", "metrics.key", "metrics.raw"])
        .expect("Failed to subscribe for backfill");

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    loop {
        // Sleep until Health Monitor wakes us up
        info!("Backfill engine dormant, waiting for signal...");
        state.backfill_notify.notified().await;
        info!("Backfill engine woken up! Starting recovery...");

        // Token bucket rate limiter
        // Calculate bytes per second based on bandwidth cap
        let bytes_per_sec: u64 =
            (1_000_000_000u64 * state.config.backfill_bandwidth_cap_pct as u64) / (100 * 8);
        let mut tokens: u64 = bytes_per_sec; // start with 1 second of tokens
        let mut last_refill = tokio::time::Instant::now();
        let refill_interval = Duration::from_millis(100);

        let mut total_sent: u64 = 0;
        let mut batch_count: u64 = 0;

        loop {
            // Check if still in backfill state
            let conn_state = state.get_connection_state();
            if conn_state != ConnectionState::Backfilling {
                info!(state = %conn_state, "Connection state changed, stopping backfill");
                break;
            }

            // Refill tokens
            let now = tokio::time::Instant::now();
            let elapsed = now.duration_since(last_refill);
            if elapsed >= refill_interval {
                tokens += bytes_per_sec * elapsed.as_millis() as u64 / 1000;
                tokens = tokens.min(bytes_per_sec * 2); // cap at 2 seconds
                last_refill = now;
            }

            // Pull batch from Kafka
            let mut records: Vec<MetricRecord> = Vec::with_capacity(1000);

            for _ in 0..1000 {
                match tokio::time::timeout(Duration::from_millis(10), consumer.recv()).await {
                    Ok(Ok(msg)) => {
                        if let Some(payload) = msg.payload() {
                            if let Ok(record) = serde_json::from_slice::<MetricRecord>(payload) {
                                records.push(record);
                            }
                        }
                    }
                    _ => break,
                }
            }

            if records.is_empty() {
                // No more data to backfill -> done
                info!(
                    batches = batch_count,
                    bytes = total_sent,
                    "Backfill complete!"
                );
                let _ = state.connection_tx.send(ConnectionState::Connected);
                break;
            }

            // Sort by priority: alarm > key > raw
            records.sort_by_key(|r| match r.metric_id.as_str() {
                id if id.starts_with("alarm") => 0,
                id if id.starts_with("key") => 1,
                _ => 2,
            });

            let payload = serde_json::to_vec(&records).unwrap();
            let payload_len = payload.len() as u64;

            // Rate limit: wait for sufficient tokens
            while tokens < payload_len {
                tokio::time::sleep(refill_interval).await;
                let now = tokio::time::Instant::now();
                let elapsed = now.duration_since(last_refill);
                tokens += bytes_per_sec * elapsed.as_millis() as u64 / 1000;
                tokens = tokens.min(bytes_per_sec * 2);
                last_refill = now;
            }
            tokens -= payload_len;

            // Send to cloud
            let url = format!("{}/ingest/backfill", state.config.aws_endpoint);
            match http_client.post(&url).body(payload).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Err(e) = consumer.commit_consumer_state(CommitMode::Async) {
                        warn!("Backfill: failed to commit: {}", e);
                    }
                    total_sent += payload_len;
                    batch_count += 1;

                    if batch_count % 10 == 0 {
                        info!(
                            batches = batch_count,
                            mb_sent = total_sent as f64 / 1_048_576.0,
                            "Backfill progress"
                        );
                    }
                }
                Ok(resp) => {
                    error!(status = %resp.status(), "Backfill: cloud returned error");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
                Err(e) => {
                    error!(error = %e, "Backfill send failed");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }
}
