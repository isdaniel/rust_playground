use crate::error::{RaftError, Result};
use crate::log::LogEntry;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Persistent state that must survive crashes (Raft paper Figure 2).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersistentState {
    pub current_term: u64,
    pub voted_for: Option<u64>,
    pub log: Vec<LogEntry>,
}

/// File-backed storage for the persistent Raft state.
pub struct Storage {
    path: PathBuf,
}

impl Storage {
    pub fn new(data_dir: &Path, node_id: u64) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let path = data_dir.join(format!("node_{}.json", node_id));
        Ok(Self { path })
    }

    /// Load state from disk, or return default if no file exists.
    pub fn load(&self) -> Result<PersistentState> {
        if !self.path.exists() {
            return Ok(PersistentState::default());
        }
        let data = std::fs::read_to_string(&self.path)?;
        let state: PersistentState = serde_json::from_str(&data)?;
        Ok(state)
    }

    /// Persist the current state to disk (atomic-ish: write-then-rename).
    pub fn save(&self, state: &PersistentState) -> Result<()> {
        let tmp = self.path.with_extension("tmp");
        let data = serde_json::to_string(state)?;
        std::fs::write(&tmp, data)?;
        std::fs::rename(&tmp, &self.path).map_err(|e| {
            RaftError::Internal(format!("failed to rename state file: {}", e))
        })?;
        Ok(())
    }
}
