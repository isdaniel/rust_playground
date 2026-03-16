use serde::{Deserialize, Serialize};

/// A single entry in the Raft log.
///
/// The `command` field carries an opaque byte payload that is interpreted
/// by the application-level [`StateMachine`](crate::state_machine::StateMachine).
/// A `None` command represents the protocol-level **no-op** entry that
/// a new leader appends at the start of its term (Raft paper Section 5.4.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// The term when the entry was received by the leader.
    pub term: u64,
    /// The index in the log (1-based, as per the Raft paper).
    pub index: u64,
    /// The application command payload.
    /// `None` = no-op (protocol level); `Some(bytes)` = application command.
    pub command: Option<Vec<u8>>,
}
