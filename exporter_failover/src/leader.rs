/// Leader Election with Sticky Primary Failover
///
/// Two-layer defense against leadership flip-flop:
///
/// Layer 1 — Cooperative-Sticky Partition Assignment (Kafka-level):
///   `partition.assignment.strategy = cooperative-sticky` preserves existing
///   partition assignments during rebalance.  When the old primary recovers
///   and rejoins the consumer group, the new primary keeps its partition.
///
/// Layer 2 — Leader Claim Fencing (Application-level):
///   The Active instance periodically writes "leader claim" messages to
///   `__exporter_leader` with its `instance_id`.  Before promoting, any
///   instance checks for recent claims.  If another instance has a fresh
///   claim (within `failover_timeout_secs`), promotion is deferred.
///
/// Together these ensure that after failover, the new primary keeps its role
/// even when the old primary recovers.
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use tracing::{debug, info, warn};

use crate::state::{HaRole, LeaderClaim, LeaderClaimState, SharedState};

// ── Pure logic: should we promote to Active? ────────────────────────────────

/// Decide whether this instance should promote to Active.
///
/// Rules (evaluated in order):
///   1. No known leader            → true  (first startup / clean slate)
///   2. Known leader is us         → true  (we are already the leader)
///   3. Known leader is someone else AND claim is fresh (within timeout)
///                                 → false (defer — they are still alive)
///   4. Known leader is someone else AND claim is expired
///                                 → true  (they are dead — take over)
pub fn should_promote(
    our_id: &str,
    known_leader: Option<&LeaderClaimState>,
    now: DateTime<Utc>,
    failover_timeout: Duration,
) -> bool {
    match known_leader {
        None => true,
        Some(claim) => {
            if claim.instance_id == our_id {
                return true;
            }
            let claim_age = now
                .signed_duration_since(claim.timestamp)
                .to_std()
                .unwrap_or(Duration::ZERO);
            claim_age > failover_timeout
        }
    }
}

// ── Message parsing ─────────────────────────────────────────────────────────

/// Process a message from the `__exporter_leader` topic.
///
/// Two kinds of messages live on this topic:
///   * **Producer heartbeats** — key = `"leader"`, no structured payload.
///     These are sent by the `producer` binary to keep the topic alive
///     (triggering partition assignment).  We ignore the payload.
///   * **Leader claims** — key = instance_id, JSON payload with
///     `{ "type": "leader_claim", "instance_id": "...", "ts": "..." }`.
///     We parse these and update shared state.
pub async fn process_leader_message(
    key: Option<&[u8]>,
    payload: Option<&[u8]>,
    state: &SharedState,
) {
    let key_str = match key {
        Some(k) => match std::str::from_utf8(k) {
            Ok(s) => s,
            Err(_) => return,
        },
        None => return,
    };

    // Skip producer heartbeats (key = "leader")
    if key_str == "leader" {
        return;
    }

    // Try to parse leader claim from payload
    if let Some(payload_bytes) = payload {
        if let Ok(claim) = serde_json::from_slice::<LeaderClaim>(payload_bytes) {
            if claim.claim_type == "leader_claim" {
                debug!(
                    from = %claim.instance_id,
                    ts = %claim.ts,
                    "Received leader claim"
                );
                state
                    .update_leader_claim(&claim.instance_id, claim.ts)
                    .await;
            }
        }
    }
}

// ── Claim producer ──────────────────────────────────────────────────────────

/// Build a leader claim JSON payload for this instance.
fn build_claim_payload(instance_id: &str) -> String {
    let claim = LeaderClaim {
        claim_type: "leader_claim".to_string(),
        instance_id: instance_id.to_string(),
        ts: Utc::now(),
    };
    serde_json::to_string(&claim).expect("LeaderClaim serialization cannot fail")
}

/// Write a single leader claim to `__exporter_leader`.
async fn publish_leader_claim(producer: &FutureProducer, instance_id: &str) {
    let payload = build_claim_payload(instance_id);
    let record = FutureRecord::to("__exporter_leader")
        .key(instance_id)
        .payload(&payload);

    match producer.send(record, Duration::from_secs(5)).await {
        Ok(_) => {
            debug!(instance = %instance_id, "Published leader claim");
        }
        Err((e, _)) => {
            warn!(instance = %instance_id, error = %e, "Failed to publish leader claim");
        }
    }
}

// ── Main election loop ──────────────────────────────────────────────────────

/// Run the leader election loop.
/// This function never returns under normal operation.
pub async fn leader_election_loop(state: Arc<SharedState>) {
    let brokers = state.config.kafka_brokers.join(",");
    let our_id = state.config.instance_id.clone();
    let failover_timeout =
        Duration::from_secs(state.config.failover_timeout_secs);
    let claim_interval =
        Duration::from_secs(state.config.leader_claim_interval_secs);

    // ── Consumer: cooperative-sticky assignment ──
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "exporter-leader")
        .set("bootstrap.servers", &brokers)
        .set("session.timeout.ms", "10000")
        .set("heartbeat.interval.ms", "3000")
        .set("max.poll.interval.ms", "30000")
        .set("partition.assignment.strategy", "cooperative-sticky")
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "latest")
        .create()
        .expect("Failed to create leader election consumer");

    consumer
        .subscribe(&["__exporter_leader"])
        .expect("Failed to subscribe to leader topic");

    // ── Producer: for writing leader claims ──
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Failed to create leader claim producer");

    info!(
        instance = %our_id,
        "Joined leader election group (cooperative-sticky), waiting for partition assignment..."
    );

    let mut last_claim_time = std::time::Instant::now()
        .checked_sub(claim_interval)
        .unwrap_or_else(std::time::Instant::now);

    loop {
        // Poll for messages with a 2-second timeout.
        match tokio::time::timeout(Duration::from_secs(2), consumer.recv()).await {
            Ok(Ok(msg)) => {
                // Process the message (may be a heartbeat or a leader claim)
                process_leader_message(
                    msg.key(),
                    msg.payload(),
                    &state,
                )
                .await;

                // We hold the partition → check if we should promote
                let mut role = state.ha_role.lock().await;
                if *role == HaRole::Standby {
                    let claim = state.get_leader_claim().await;
                    let now = Utc::now();
                    if should_promote(&our_id, claim.as_ref(), now, failover_timeout) {
                        info!(
                            instance = %our_id,
                            offset = msg.offset(),
                            "Partition assigned + promotion approved -- promoting to ACTIVE"
                        );
                        *role = HaRole::Active;
                    } else {
                        debug!(
                            instance = %our_id,
                            "Partition assigned but another leader has fresh claim -- deferring"
                        );
                    }
                }
            }
            Ok(Err(e)) => {
                warn!("Leader election poll error: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Err(_) => {
                // Timeout: no message. Check partition assignment.
                let assignment = consumer.assignment();
                match assignment {
                    Ok(tpl) => {
                        let count = tpl.count();
                        let mut role = state.ha_role.lock().await;
                        if count > 0 && *role == HaRole::Standby {
                            let claim = state.get_leader_claim().await;
                            let now = Utc::now();
                            if should_promote(&our_id, claim.as_ref(), now, failover_timeout) {
                                info!(
                                    instance = %our_id,
                                    partitions = count,
                                    "Partition(s) detected + promotion approved -- promoting to ACTIVE"
                                );
                                *role = HaRole::Active;
                            } else {
                                debug!(
                                    instance = %our_id,
                                    "Partition(s) detected but another leader has fresh claim -- deferring"
                                );
                            }
                        } else if count == 0 && *role == HaRole::Active {
                            warn!(
                                instance = %our_id,
                                "Lost all partitions -- demoting to STANDBY"
                            );
                            *role = HaRole::Standby;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to check assignment: {}", e);
                    }
                }
            }
        }

        // ── While Active: periodically publish leader claims ──
        let current_role = state.get_role().await;
        if current_role == HaRole::Active && last_claim_time.elapsed() >= claim_interval {
            publish_leader_claim(&producer, &our_id).await;
            // Also update our own shared state so we see our own claim
            state.update_leader_claim(&our_id, Utc::now()).await;
            last_claim_time = std::time::Instant::now();
        }
    }
}

/// Wait until this instance becomes Active.
pub async fn wait_until_active(state: Arc<SharedState>) {
    loop {
        {
            let role = state.ha_role.lock().await;
            if *role == HaRole::Active {
                return;
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Continuously check if we are still Active. If demoted, log and return false.
/// This is used by worker tasks to gracefully stop when losing leadership.
pub async fn check_still_active(state: &SharedState) -> bool {
    let role = state.ha_role.lock().await;
    *role == HaRole::Active
}

// ── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;

    const TIMEOUT: Duration = Duration::from_secs(15);

    fn claim(id: &str, ts: DateTime<Utc>) -> LeaderClaimState {
        LeaderClaimState {
            instance_id: id.to_string(),
            timestamp: ts,
        }
    }

    #[test]
    fn test_no_known_leader_promotes() {
        let now = Utc::now();
        assert!(should_promote("server-a", None, now, TIMEOUT));
    }

    #[test]
    fn test_self_claim_promotes() {
        let now = Utc::now();
        let c = claim("server-a", now - ChronoDuration::seconds(2));
        assert!(should_promote("server-a", Some(&c), now, TIMEOUT));
    }

    #[test]
    fn test_fresh_other_claim_defers() {
        let now = Utc::now();
        // Claim is 5s old, timeout is 15s → fresh → defer
        let c = claim("server-b", now - ChronoDuration::seconds(5));
        assert!(!should_promote("server-a", Some(&c), now, TIMEOUT));
    }

    #[test]
    fn test_expired_other_claim_promotes() {
        let now = Utc::now();
        // Claim is 20s old, timeout is 15s → expired → promote
        let c = claim("server-b", now - ChronoDuration::seconds(20));
        assert!(should_promote("server-a", Some(&c), now, TIMEOUT));
    }

    #[test]
    fn test_boundary_exact_timeout_does_not_promote() {
        let now = Utc::now();
        // Claim is exactly 15s old = timeout → NOT expired (need strictly >)
        let c = claim("server-b", now - ChronoDuration::seconds(15));
        assert!(!should_promote("server-a", Some(&c), now, TIMEOUT));
    }

    #[test]
    fn test_boundary_just_past_timeout_promotes() {
        let now = Utc::now();
        // Claim is 16s old, timeout is 15s → expired → promote
        let c = claim("server-b", now - ChronoDuration::seconds(16));
        assert!(should_promote("server-a", Some(&c), now, TIMEOUT));
    }

    #[test]
    fn test_very_old_claim_promotes() {
        let now = Utc::now();
        // Claim is 10 minutes old → definitely expired
        let c = claim("server-b", now - ChronoDuration::seconds(600));
        assert!(should_promote("server-a", Some(&c), now, TIMEOUT));
    }

    #[test]
    fn test_future_claim_defers() {
        // Edge case: claim timestamp is in the future (clock skew)
        let now = Utc::now();
        let c = claim("server-b", now + ChronoDuration::seconds(5));
        assert!(!should_promote("server-a", Some(&c), now, TIMEOUT));
    }

    #[test]
    fn test_self_claim_even_if_old_promotes() {
        let now = Utc::now();
        // Even a very old self-claim should promote (we are the leader)
        let c = claim("server-a", now - ChronoDuration::seconds(600));
        assert!(should_promote("server-a", Some(&c), now, TIMEOUT));
    }
}
