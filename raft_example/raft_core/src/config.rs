use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Unique identifier for a Raft node in the cluster.
pub type NodeId = u64;

/// Configuration for a single Raft node and its view of the cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftConfig {
    /// This node's unique identifier.
    pub id: NodeId,
    /// All peers in the cluster (must include this node itself).
    pub peers: Vec<PeerConfig>,
    /// Election timeout range in milliseconds.
    /// A random value in [min, max] is chosen each election cycle.
    pub election_timeout_min_ms: u64,
    pub election_timeout_max_ms: u64,
    /// Heartbeat interval in milliseconds (leader sends AppendEntries).
    pub heartbeat_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerConfig {
    pub id: NodeId,
    pub addr: SocketAddr,
}

impl RaftConfig {
    /// Returns the socket address for this node.
    pub fn self_addr(&self) -> SocketAddr {
        self.peers
            .iter()
            .find(|p| p.id == self.id)
            .expect("config must contain this node's own id in peers")
            .addr
    }

    /// Returns peer configs excluding this node.
    pub fn other_peers(&self) -> Vec<&PeerConfig> {
        self.peers.iter().filter(|p| p.id != self.id).collect()
    }

    /// Number of nodes required for a majority (quorum).
    pub fn quorum(&self) -> usize {
        self.peers.len() / 2 + 1
    }
}
