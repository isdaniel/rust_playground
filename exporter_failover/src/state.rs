use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{watch, Mutex, Notify};

use crate::config::ExporterConfig;

/// Connection state machine: Connected -> Disconnected -> Backfilling -> Connected
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Backfilling,
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionState::Connected => write!(f, "CONNECTED"),
            ConnectionState::Disconnected => write!(f, "DISCONNECTED"),
            ConnectionState::Backfilling => write!(f, "BACKFILLING"),
        }
    }
}

/// HA Role: Active processes data, Standby monitors and waits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HaRole {
    Active,
    Standby,
}

impl std::fmt::Display for HaRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HaRole::Active => write!(f, "ACTIVE"),
            HaRole::Standby => write!(f, "STANDBY"),
        }
    }
}

/// JSON payload written to `__exporter_leader` for leader claim fencing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderClaim {
    #[serde(rename = "type")]
    pub claim_type: String,
    pub instance_id: String,
    pub ts: DateTime<Utc>,
}

/// In-memory record of the most recent leader claim.
#[derive(Debug, Clone)]
pub struct LeaderClaimState {
    pub instance_id: String,
    pub timestamp: DateTime<Utc>,
}

/// Shared state across all async tasks.
pub struct SharedState {
    pub connection_tx: watch::Sender<ConnectionState>,
    pub connection_rx: watch::Receiver<ConnectionState>,
    pub ha_role: Mutex<HaRole>,
    pub backfill_notify: Notify,
    pub config: ExporterConfig,
    /// Tracks the most recent leader claim seen from any instance.
    pub last_known_leader: Mutex<Option<LeaderClaimState>>,
}

impl SharedState {
    pub fn new(config: ExporterConfig) -> Arc<Self> {
        let (conn_tx, conn_rx) = watch::channel(ConnectionState::Connected);
        Arc::new(SharedState {
            connection_tx: conn_tx,
            connection_rx: conn_rx,
            ha_role: Mutex::new(HaRole::Standby), // Start as Standby
            backfill_notify: Notify::new(),
            config,
            last_known_leader: Mutex::new(None),
        })
    }

    pub async fn get_role(&self) -> HaRole {
        *self.ha_role.lock().await
    }

    pub fn get_connection_state(&self) -> ConnectionState {
        *self.connection_rx.borrow()
    }

    /// Update the last known leader claim. Only updates if the new claim is newer
    /// than the existing one (or if there is no existing claim).
    pub async fn update_leader_claim(&self, instance_id: &str, ts: DateTime<Utc>) {
        let mut claim = self.last_known_leader.lock().await;
        let should_update = match &*claim {
            None => true,
            Some(existing) => ts > existing.timestamp,
        };
        if should_update {
            *claim = Some(LeaderClaimState {
                instance_id: instance_id.to_string(),
                timestamp: ts,
            });
        }
    }

    /// Get the current leader claim, if any.
    pub async fn get_leader_claim(&self) -> Option<LeaderClaimState> {
        self.last_known_leader.lock().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn test_config() -> ExporterConfig {
        ExporterConfig {
            kafka_brokers: vec!["localhost:9092".to_string()],
            aws_endpoint: "http://localhost:8080".to_string(),
            fab_id: "TEST".to_string(),
            instance_id: "test-1".to_string(),
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

    #[tokio::test]
    async fn test_initial_state_has_no_leader_claim() {
        let state = SharedState::new(test_config());
        assert!(state.get_leader_claim().await.is_none());
    }

    #[tokio::test]
    async fn test_update_leader_claim_sets_claim() {
        let state = SharedState::new(test_config());
        let now = Utc::now();
        state.update_leader_claim("server-a", now).await;

        let claim = state.get_leader_claim().await.unwrap();
        assert_eq!(claim.instance_id, "server-a");
        assert_eq!(claim.timestamp, now);
    }

    #[tokio::test]
    async fn test_newer_claim_replaces_older() {
        let state = SharedState::new(test_config());
        let t1 = Utc::now();
        let t2 = t1 + chrono::Duration::seconds(5);

        state.update_leader_claim("server-a", t1).await;
        state.update_leader_claim("server-b", t2).await;

        let claim = state.get_leader_claim().await.unwrap();
        assert_eq!(claim.instance_id, "server-b");
        assert_eq!(claim.timestamp, t2);
    }

    #[tokio::test]
    async fn test_older_claim_does_not_replace_newer() {
        let state = SharedState::new(test_config());
        let t1 = Utc::now();
        let t2 = t1 - chrono::Duration::seconds(5);

        state.update_leader_claim("server-a", t1).await;
        state.update_leader_claim("server-b", t2).await;

        let claim = state.get_leader_claim().await.unwrap();
        assert_eq!(claim.instance_id, "server-a");
        assert_eq!(claim.timestamp, t1);
    }

    #[tokio::test]
    async fn test_initial_role_is_standby() {
        let state = SharedState::new(test_config());
        assert_eq!(state.get_role().await, HaRole::Standby);
    }

    #[tokio::test]
    async fn test_leader_claim_serde_roundtrip() {
        let claim = LeaderClaim {
            claim_type: "leader_claim".to_string(),
            instance_id: "server-a".to_string(),
            ts: Utc::now(),
        };
        let json = serde_json::to_string(&claim).unwrap();
        let parsed: LeaderClaim = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.instance_id, "server-a");
        assert_eq!(parsed.claim_type, "leader_claim");
    }
}
