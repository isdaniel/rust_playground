/// Leader Election with Sticky Primary Failover
///
/// Three-layer defense against leadership flip-flop:
///
/// Layer 1 — Startup Grace Period (Application-level):
///   On startup, the election loop waits `startup_grace_secs` before allowing
///   promotion.  During this period it consumes messages from `__exporter_leader`
///   to populate `last_known_leader`, giving the current active leader time to
///   publish at least 2 claims.
///
/// Layer 2 — Peer Health Check (Application-level):
///   Before promoting, query the peer's `/status` HTTP endpoint.  If the peer
///   responds with `role: "ACTIVE"`, defer promotion (peer is alive and leading).
///   Uses a 3-second HTTP timeout — won't delay failover when peer is truly dead.
///
/// Layer 3 — Cooperative-Sticky Partition Assignment (Kafka-level):
///   `partition.assignment.strategy = cooperative-sticky` preserves existing
///   partition assignments during rebalance.  When the old primary recovers
///   and rejoins the consumer group, the new primary keeps its partition.
///
/// Layer 4 — Leader Claim Fencing (Application-level):
///   The Active instance periodically writes "leader claim" messages to
///   `__exporter_leader` with its `instance_id`.  Before promoting, any
///   instance checks for recent claims.  If another instance has a fresh
///   claim (within `failover_timeout_secs`), promotion is deferred.
///
/// Layer 5 — Demotion Grace (Application-level):
///   Don't demote immediately when partition count drops to 0 (can happen
///   briefly during cooperative rebalance).  Require 3 consecutive checks
///   (~6 seconds) with 0 partitions before demoting.
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

// ── Peer health check ──────────────────────────────────────────────────────

/// Check whether the peer is currently Active by querying its /status endpoint.
/// Returns `true` if the peer responds with `role: "ACTIVE"`, `false` otherwise
/// (unreachable, timeout, non-ACTIVE role, parse error).
async fn peer_is_active(peer_endpoint: &str) -> bool {
    let url = format!("http://{}/status", peer_endpoint);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(_) => return false,
    };

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(body) = resp.text().await {
                // Parse the JSON and check the role field
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(role) = json.get("role").and_then(|v| v.as_str()) {
                        return role == "ACTIVE";
                    }
                }
            }
            false
        }
        _ => false,
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
    let peer_endpoint = state.config.peer_endpoint.clone();
    let startup_grace = Duration::from_secs(state.config.startup_grace_secs);

    // ── Consumer: cooperative-sticky assignment ──
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "exporter-leader")
        .set("bootstrap.servers", &brokers)
        .set("session.timeout.ms", "10000")
        .set("heartbeat.interval.ms", "3000")
        .set("max.poll.interval.ms", "30000")
        .set("partition.assignment.strategy", "cooperative-sticky")
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "earliest")
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
        startup_grace_secs = startup_grace.as_secs(),
        "Joined leader election group (cooperative-sticky), waiting for partition assignment..."
    );

    let mut last_claim_time = std::time::Instant::now()
        .checked_sub(claim_interval)
        .unwrap_or_else(std::time::Instant::now);

    // ── Layer 1: Startup grace period ──
    let startup_deadline = tokio::time::Instant::now() + startup_grace;
    let mut startup_complete = false;

    // ── Layer 5: Demotion grace counter ──
    let mut zero_partition_count: u32 = 0;
    const DEMOTION_THRESHOLD: u32 = 3;

    loop {
        // Check if startup grace period has elapsed
        if !startup_complete && tokio::time::Instant::now() >= startup_deadline {
            startup_complete = true;
            info!(
                instance = %our_id,
                "Startup grace period complete, promotion decisions now active"
            );
        }

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

                // During startup grace, consume messages but skip promotion checks
                if !startup_complete {
                    debug!(
                        instance = %our_id,
                        "Startup grace period: consuming claims but deferring promotion"
                    );
                    continue;
                }

                // Reset demotion grace counter — we have a partition (received a message)
                zero_partition_count = 0;

                // We hold the partition → check if we should promote
                let mut role = state.ha_role.lock().await;
                if *role == HaRole::Standby {
                    let claim = state.get_leader_claim().await;
                    let now = Utc::now();
                    if should_promote(&our_id, claim.as_ref(), now, failover_timeout) {
                        // Layer 2: Peer health check before promoting
                        if let Some(ref endpoint) = peer_endpoint {
                            if peer_is_active(endpoint).await {
                                info!(
                                    instance = %our_id,
                                    peer = %endpoint,
                                    "Peer is ACTIVE, deferring promotion"
                                );
                                continue;
                            }
                        }
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
                // During startup grace, skip promotion/demotion checks
                if !startup_complete {
                    debug!(
                        instance = %our_id,
                        "Startup grace period: waiting for claims..."
                    );
                    continue;
                }

                // Timeout: no message. Check partition assignment.
                let assignment = consumer.assignment();
                match assignment {
                    Ok(tpl) => {
                        let count = tpl.count();
                        let mut role = state.ha_role.lock().await;
                        if count > 0 && *role == HaRole::Standby {
                            // Reset demotion grace counter
                            zero_partition_count = 0;

                            let claim = state.get_leader_claim().await;
                            let now = Utc::now();
                            if should_promote(&our_id, claim.as_ref(), now, failover_timeout) {
                                // Layer 2: Peer health check before promoting
                                if let Some(ref endpoint) = peer_endpoint {
                                    if peer_is_active(endpoint).await {
                                        info!(
                                            instance = %our_id,
                                            peer = %endpoint,
                                            "Peer is ACTIVE, deferring promotion"
                                        );
                                        continue;
                                    }
                                }
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
                            // Layer 5: Demotion grace — require consecutive 0-partition checks
                            zero_partition_count += 1;
                            if zero_partition_count >= DEMOTION_THRESHOLD {
                                warn!(
                                    instance = %our_id,
                                    consecutive_zero_checks = zero_partition_count,
                                    "Lost all partitions for {} consecutive checks -- demoting to STANDBY",
                                    DEMOTION_THRESHOLD
                                );
                                *role = HaRole::Standby;
                                zero_partition_count = 0;
                            } else {
                                debug!(
                                    instance = %our_id,
                                    consecutive_zero_checks = zero_partition_count,
                                    threshold = DEMOTION_THRESHOLD,
                                    "Zero partitions detected, demotion grace {}/{}",
                                    zero_partition_count, DEMOTION_THRESHOLD
                                );
                            }
                        } else if count > 0 {
                            // Active with partitions — reset demotion counter
                            zero_partition_count = 0;
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
