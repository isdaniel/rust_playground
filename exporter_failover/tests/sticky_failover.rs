/// Integration tests for sticky primary failover.
///
/// These tests exercise multi-step failover scenarios using the pure
/// `should_promote` logic and `SharedState` claim tracking — no Kafka
/// broker required.
use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};

use exporter_failover::config::ExporterConfig;
use exporter_failover::leader::should_promote;
use exporter_failover::state::{LeaderClaimState, SharedState};

fn test_config(instance_id: &str) -> ExporterConfig {
    ExporterConfig {
        kafka_brokers: vec!["localhost:9092".to_string()],
        aws_endpoint: "http://localhost:8080".to_string(),
        fab_id: "TEST".to_string(),
        instance_id: instance_id.to_string(),
        normal_batch_size: 100,
        normal_flush_secs: 5,
        slow_batch_size: 500,
        slow_flush_secs: 15,
        backfill_bandwidth_cap_pct: 30,
        heartbeat_interval_secs: 5,
        failover_timeout_secs: 15,
        leader_claim_interval_secs: 3,
        http_port: 9090,
    }
}

fn claim(id: &str, ts: chrono::DateTime<Utc>) -> LeaderClaimState {
    LeaderClaimState {
        instance_id: id.to_string(),
        timestamp: ts,
    }
}

const TIMEOUT: Duration = Duration::from_secs(15);

// ── Scenario 1: Normal startup — no prior leader ────────────────────────────

#[test]
fn scenario_normal_startup_no_prior_leader() {
    // First instance starts, no leader claim exists → promote immediately.
    let now = Utc::now();
    assert!(
        should_promote("server-a", None, now, TIMEOUT),
        "First startup with no leader should promote"
    );
}

// ── Scenario 2: Standby defers to active leader ─────────────────────────────

#[test]
fn scenario_standby_defers_to_active_fresh_claim() {
    // server-a is Active and writing claims every 3s.
    // server-b gets the partition somehow — it should see server-a's fresh claim
    // and defer promotion.
    let now = Utc::now();
    let fresh_claim = claim("server-a", now - ChronoDuration::seconds(2));

    assert!(
        !should_promote("server-b", Some(&fresh_claim), now, TIMEOUT),
        "Standby should defer when active leader has a fresh claim"
    );
}

// ── Scenario 3: Failover after crash — claim expired ────────────────────────

#[test]
fn scenario_failover_after_crash() {
    // server-a was Active, last claim was 20s ago (crashed ~18s ago).
    // server-b gets the partition after Kafka rebalance.
    // Claim is expired → promote.
    let now = Utc::now();
    let expired_claim = claim("server-a", now - ChronoDuration::seconds(20));

    assert!(
        should_promote("server-b", Some(&expired_claim), now, TIMEOUT),
        "Should promote after leader's claim expires (crash detected)"
    );
}

// ── Scenario 4: THE KEY TEST — sticky primary ───────────────────────────────

#[test]
fn scenario_sticky_primary_old_server_comes_back() {
    // Timeline:
    //   T=0:   server-a is Active, writing claims every 3s
    //   T=5:   server-a crashes
    //   T=20:  server-b detects expired claim, promotes to Active
    //   T=21:  server-b starts writing claims every 3s
    //   T=30:  server-a recovers, gets partition (cooperative-sticky may prevent
    //          this, but we test the application-level defense)
    //
    // At T=30, server-a sees server-b's fresh claim (age ~1-2s) → defers → stays Standby.

    let t_base = Utc::now();

    // Step 1: server-a is Active with fresh claim
    let t0 = t_base;
    let claim_a = claim("server-a", t0);
    assert!(
        should_promote("server-a", Some(&claim_a), t0, TIMEOUT),
        "server-a should be Active (self claim)"
    );

    // Step 2: server-a crashes. server-b sees claim is expired after 20s
    let t20 = t_base + ChronoDuration::seconds(20);
    assert!(
        should_promote("server-b", Some(&claim_a), t20, TIMEOUT),
        "server-b should promote after server-a's claim expires"
    );

    // Step 3: server-b is now Active and writing claims
    let t29 = t_base + ChronoDuration::seconds(29);
    let claim_b = claim("server-b", t29);

    // Step 4: server-a comes back at T=30, sees server-b's fresh claim (1s old)
    let t30 = t_base + ChronoDuration::seconds(30);
    assert!(
        !should_promote("server-a", Some(&claim_b), t30, TIMEOUT),
        "STICKY: old primary should stay Standby when new primary has fresh claim"
    );

    // Step 5: server-b continues as Active (self claim)
    assert!(
        should_promote("server-b", Some(&claim_b), t30, TIMEOUT),
        "New primary should remain Active (self claim)"
    );
}

// ── Scenario 5: Double failure — eventual promotion ─────────────────────────

#[test]
fn scenario_double_failure_eventual_promotion() {
    // Both servers crash.  A third instance (or one of them restarting much later)
    // should eventually promote once ALL claims expire.
    let now = Utc::now();

    // server-a claimed at T-60, server-b claimed at T-30.
    // Check from server-c's perspective at T=now:
    // The most recent claim (server-b, 30s ago) is expired → promote.
    let old_claim = claim("server-b", now - ChronoDuration::seconds(30));
    assert!(
        should_promote("server-c", Some(&old_claim), now, TIMEOUT),
        "Should promote when all known claims are expired"
    );
}

// ── Scenario 6: Gradual claim expiry over time ──────────────────────────────

#[test]
fn scenario_gradual_claim_expiry() {
    // Simulate server-b watching server-a's claim age over time.
    let t0 = Utc::now();
    let claim_a = claim("server-a", t0);

    // At T+5s: claim is 5s old → fresh → defer
    let t5 = t0 + ChronoDuration::seconds(5);
    assert!(
        !should_promote("server-b", Some(&claim_a), t5, TIMEOUT),
        "At T+5s claim should still be fresh"
    );

    // At T+10s: claim is 10s old → fresh → defer
    let t10 = t0 + ChronoDuration::seconds(10);
    assert!(
        !should_promote("server-b", Some(&claim_a), t10, TIMEOUT),
        "At T+10s claim should still be fresh"
    );

    // At T+14s: claim is 14s old → fresh → defer
    let t14 = t0 + ChronoDuration::seconds(14);
    assert!(
        !should_promote("server-b", Some(&claim_a), t14, TIMEOUT),
        "At T+14s claim should still be fresh (just under timeout)"
    );

    // At T+15s: claim is exactly 15s old → still fresh (strictly greater)
    let t15 = t0 + ChronoDuration::seconds(15);
    assert!(
        !should_promote("server-b", Some(&claim_a), t15, TIMEOUT),
        "At T+15s (exact boundary) should not yet promote"
    );

    // At T+16s: claim is 16s old → expired → promote
    let t16 = t0 + ChronoDuration::seconds(16);
    assert!(
        should_promote("server-b", Some(&claim_a), t16, TIMEOUT),
        "At T+16s claim should be expired → promote"
    );
}

// ── Scenario 7: SharedState claim tracking across updates ───────────────────

#[tokio::test]
async fn scenario_shared_state_claim_tracking() {
    let state = SharedState::new(test_config("server-b"));

    // Initially no claim
    assert!(state.get_leader_claim().await.is_none());

    // server-a writes a claim
    let t0 = Utc::now();
    state.update_leader_claim("server-a", t0).await;

    let c1 = state.get_leader_claim().await.unwrap();
    assert_eq!(c1.instance_id, "server-a");

    // server-b writes a newer claim (after promoting)
    let t1 = t0 + ChronoDuration::seconds(20);
    state.update_leader_claim("server-b", t1).await;

    let c2 = state.get_leader_claim().await.unwrap();
    assert_eq!(c2.instance_id, "server-b");
    assert_eq!(c2.timestamp, t1);

    // An older claim from server-a should NOT replace server-b's newer claim
    let t_old = t0 + ChronoDuration::seconds(5);
    state.update_leader_claim("server-a", t_old).await;

    let c3 = state.get_leader_claim().await.unwrap();
    assert_eq!(
        c3.instance_id, "server-b",
        "Older claim should not overwrite newer claim"
    );
}

// ── Scenario 8: Rapid leader transitions ────────────────────────────────────

#[test]
fn scenario_rapid_leader_transitions() {
    // Simulate rapid failover: A → B → A (if B also crashes quickly).
    let t0 = Utc::now();

    // server-a is Active
    let claim_a = claim("server-a", t0);
    assert!(should_promote("server-a", Some(&claim_a), t0, TIMEOUT));

    // server-a crashes. After timeout, server-b promotes.
    let t16 = t0 + ChronoDuration::seconds(16);
    assert!(should_promote("server-b", Some(&claim_a), t16, TIMEOUT));

    // server-b is Active but crashes almost immediately (claims at T+17)
    let claim_b = claim("server-b", t0 + ChronoDuration::seconds(17));

    // server-a comes back at T+18, sees server-b's very fresh claim → defer
    let t18 = t0 + ChronoDuration::seconds(18);
    assert!(
        !should_promote("server-a", Some(&claim_b), t18, TIMEOUT),
        "server-a should defer to server-b's fresh claim"
    );

    // server-b's claim expires at T+33 (17+16)
    let t33 = t0 + ChronoDuration::seconds(33);
    assert!(
        should_promote("server-a", Some(&claim_b), t33, TIMEOUT),
        "server-a should promote after server-b's claim expires"
    );
}
