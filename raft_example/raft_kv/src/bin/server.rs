use std::path::PathBuf;

use raft_core::config::RaftConfig;
use raft_core::node::RaftNode;
use raft_core::storage::Storage;
use raft_core::transport;
use raft_kv::kv::KvStateMachine;
use tokio::sync::mpsc;
use tracing::info;

/// Usage:
///   raft-server <config.json>
///
/// The config file describes this node and the full cluster.
/// Example config (3-node cluster, this is node 1):
/// ```json
/// {
///   "id": 1,
///   "peers": [
///     { "id": 1, "addr": "127.0.0.1:9001" },
///     { "id": 2, "addr": "127.0.0.1:9002" },
///     { "id": 3, "addr": "127.0.0.1:9003" }
///   ],
///   "election_timeout_min_ms": 1500,
///   "election_timeout_max_ms": 3000,
///   "heartbeat_interval_ms": 500
/// }
/// ```
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: raft-server <config.json>");
        std::process::exit(1);
    }

    let config_path = &args[1];
    let config_str = std::fs::read_to_string(config_path).unwrap_or_else(|e| {
        eprintln!("failed to read config file '{}': {}", config_path, e);
        std::process::exit(1);
    });

    let config: RaftConfig = serde_json::from_str(&config_str).unwrap_or_else(|e| {
        eprintln!("invalid config JSON: {}", e);
        std::process::exit(1);
    });

    let listen_addr = config.self_addr();
    info!("starting raft node {} on {}", config.id, listen_addr);

    // Storage directory.
    let data_dir = PathBuf::from("data");
    let storage = Storage::new(&data_dir, config.id).expect("failed to create storage");

    // Application-specific state machine.
    let state_machine = KvStateMachine::new();

    // Channel for incoming RPCs: transport -> node.
    let (rpc_tx, rpc_rx) = mpsc::channel(256);

    // Start TCP listener.
    tokio::spawn(async move {
        if let Err(e) = transport::start_listener(listen_addr, rpc_tx).await {
            eprintln!("listener error: {}", e);
        }
    });

    // Create and run the Raft node.
    let node =
        RaftNode::new(config, storage, state_machine, rpc_rx).expect("failed to create raft node");
    if let Err(e) = node.run().await {
        eprintln!("raft node error: {}", e);
    }
}
