/// Realtime Consumer: consumes metrics from Kafka and sends them to the cloud
/// in adaptive micro-batches.
///
/// Key behaviors:
/// - Infinite loop, always running while Active
/// - Pauses when WAN is disconnected (Kafka buffers automatically)
/// - At-least-once delivery: commit offset only after cloud confirms receipt
/// - Adaptive batch size based on connection state
use std::sync::Arc;
use std::time::Duration;

use rdkafka::config::ClientConfig;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message;
use tracing::{error, info, warn};

use crate::leader::check_still_active;
use crate::models::{MetricRecord, MetricsBatch};
use crate::state::{ConnectionState, SharedState};

pub async fn realtime_consumer(state: Arc<SharedState>) {
    let brokers = state.config.kafka_brokers.join(",");

    // Both Active and Standby use the same group.id "rt-metrics".
    // Kafka ensures a partition is consumed by only one consumer in the group.
    // On failover, Kafka rebalances partitions to the Standby.
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "rt-metrics")
        .set("bootstrap.servers", &brokers)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest")
        .set("session.timeout.ms", "10000")
        .set("max.poll.interval.ms", "300000")
        .set("fetch.min.bytes", "1024")
        .set("fetch.wait.max.ms", "500")
        .create()
        .expect("Failed to create realtime consumer");

    consumer
        .subscribe(&["metrics.alarm", "metrics.key", "metrics.raw"])
        .expect("Failed to subscribe to metrics topics");

    info!(instance = %state.config.instance_id, "Realtime consumer started");

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    let mut batch: Vec<MetricRecord> = Vec::with_capacity(state.config.normal_batch_size);
    let mut last_flush = tokio::time::Instant::now();
    let mut was_disconnected = false;

    loop {
        // Check if we are still Active
        if !check_still_active(&state).await {
            info!(
                instance = %state.config.instance_id,
                "No longer Active -- realtime consumer exiting for clean partition handover"
            );
            // Drop the consumer by returning. This leaves the rt-metrics consumer
            // group, triggering a rebalance so the new Active can pick up partitions.
            return;
        }

        // IMPORTANT: Always poll Kafka to keep the consumer's fetch pipeline alive. Skipping recv() during WAN disconnect causes the internal fetch state to go stale, preventing message delivery after recovery.
        match tokio::time::timeout(Duration::from_millis(100), consumer.recv()).await {
            Ok(Ok(msg)) => {
                if let Some(payload) = msg.payload() {
                    if let Ok(record) = serde_json::from_slice::<MetricRecord>(payload) {
                        batch.push(record);
                    }
                }
            }
            Ok(Err(e)) => {
                warn!("Kafka recv error: {}", e);
            }
            Err(_) => {
                // Poll timeout, check if we should flush
            }
        }

        // Check connection state AFTER polling Kafka
        let conn_state = state.get_connection_state();
        if conn_state == ConnectionState::Disconnected {
            // Log once on transition, not every iteration
            if !was_disconnected {
                info!("WAN disconnected, draining Kafka messages (backfill will replay on recovery)");
                was_disconnected = true;
            }
            // Discard accumulated records — the backfill engine will replay
            // them from its independent consumer group on WAN recovery.
            // We don't commit offsets, so a process restart would also replay.
            batch.clear();
            last_flush = tokio::time::Instant::now();
            continue;
        }

        if was_disconnected {
            info!("WAN recovered, realtime consumer resuming normal operation");
            was_disconnected = false;
        }

        // Adaptive micro-batch: decide when to flush
        let (target_size, target_duration) = get_batch_params(&state);
        let elapsed = last_flush.elapsed();

        let should_flush =
            batch.len() >= target_size || elapsed >= Duration::from_secs(target_duration);

        if should_flush && !batch.is_empty() {
            let payload = MetricsBatch {
                fab_id: state.config.fab_id.clone(),
                batch_id: uuid::Uuid::new_v4().to_string(),
                records: std::mem::take(&mut batch),
                compressed: false,
            };

            let json = serde_json::to_vec(&payload).unwrap();
            let url = format!("{}/ingest/metrics", state.config.aws_endpoint);

            match http_client.post(&url).body(json).send().await {
                Ok(resp) if resp.status().is_success() => {
                    // Successfully delivered. Commit Kafka offset.
                    if let Err(e) = consumer.commit_consumer_state(CommitMode::Async) {
                        warn!("Failed to commit offset: {}", e);
                    }
                    info!(
                        batch_size = payload.records.len(),
                        batch_id = %payload.batch_id,
                        "Batch sent successfully"
                    );
                    last_flush = tokio::time::Instant::now();
                }
                Ok(resp) => {
                    error!(status = %resp.status(), "Cloud returned error, will retry");
                    batch = payload.records;
                }
                Err(e) => {
                    error!(error = %e, "Failed to send batch to cloud");
                    batch = payload.records;
                }
            }
        }
    }
}

/// Get adaptive batch parameters based on current connection state.
fn get_batch_params(state: &SharedState) -> (usize, u64) {
    let conn_state = state.get_connection_state();
    match conn_state {
        ConnectionState::Connected => {
            (state.config.normal_batch_size, state.config.normal_flush_secs)
        }
        ConnectionState::Backfilling => {
            // During backfill, use larger batches to leave bandwidth for backfill
            (state.config.slow_batch_size, state.config.slow_flush_secs)
        }
        ConnectionState::Disconnected => {
            (state.config.normal_batch_size, state.config.normal_flush_secs)
        }
    }
}
