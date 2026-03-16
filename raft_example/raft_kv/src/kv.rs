use raft_core::state_machine::StateMachine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Application-level K/V commands
// ---------------------------------------------------------------------------

/// Application-level commands for the K/V state machine.
/// These are serialized to/from the opaque `Vec<u8>` log payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KvCommand {
    Set { key: String, value: String },
    Delete { key: String },
}

/// Application-level query (read-only, not replicated).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KvQuery {
    Get { key: String },
}

/// Result value returned from the state machine after apply/query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KvResult {
    Value(Option<String>),
}

// ---------------------------------------------------------------------------
// KvStateMachine
// ---------------------------------------------------------------------------

/// A simple in-memory key/value store implementing the Raft `StateMachine` trait.
pub struct KvStateMachine {
    store: HashMap<String, String>,
}

impl KvStateMachine {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }
}

impl Default for KvStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl StateMachine for KvStateMachine {
    fn apply(&mut self, command: &Option<Vec<u8>>) -> Option<Vec<u8>> {
        let payload = match command {
            Some(p) => p,
            None => return None, // Noop
        };

        let cmd: KvCommand = match serde_json::from_slice(payload) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("failed to deserialize KvCommand: {}", e);
                return None;
            }
        };

        let result = match cmd {
            KvCommand::Set { key, value } => {
                self.store.insert(key, value.clone());
                KvResult::Value(Some(value))
            }
            KvCommand::Delete { key } => {
                let old = self.store.remove(&key);
                KvResult::Value(old)
            }
        };

        serde_json::to_vec(&result).ok()
    }

    fn query(&self, query: &[u8]) -> Option<Vec<u8>> {
        let q: KvQuery = match serde_json::from_slice(query) {
            Ok(q) => q,
            Err(e) => {
                tracing::warn!("failed to deserialize KvQuery: {}", e);
                return None;
            }
        };

        match q {
            KvQuery::Get { key } => {
                let value = self.store.get(&key).cloned();
                let result = KvResult::Value(value);
                serde_json::to_vec(&result).ok()
            }
        }
    }
}
