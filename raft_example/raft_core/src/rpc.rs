use crate::config::NodeId;
use crate::log::LogEntry;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// RPC message envelope -- every message on the wire is one of these.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RpcMessage {
    AppendEntriesRequest(AppendEntriesRequest),
    AppendEntriesResponse(AppendEntriesResponse),
    RequestVoteRequest(RequestVoteRequest),
    RequestVoteResponse(RequestVoteResponse),
    /// Client request forwarded through the cluster.
    ClientRequest(ClientRequest),
    ClientResponse(ClientResponse),
}

// ---------------------------------------------------------------------------
// AppendEntries RPC  (Raft paper Figure 2)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesRequest {
    /// Leader's term.
    pub term: u64,
    /// So follower can redirect clients.
    pub leader_id: NodeId,
    /// Index of log entry immediately preceding new ones.
    pub prev_log_index: u64,
    /// Term of prev_log_index entry.
    pub prev_log_term: u64,
    /// Log entries to store (empty for heartbeat).
    pub entries: Vec<LogEntry>,
    /// Leader's commit_index.
    pub leader_commit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    /// Current term, for leader to update itself.
    pub term: u64,
    /// True if follower contained entry matching prev_log_index/prev_log_term.
    pub success: bool,
    /// The responder's id.
    pub from: NodeId,
    /// Hint: the last log index on this follower (for fast back-up on conflict).
    pub last_log_index: u64,
}

// ---------------------------------------------------------------------------
// RequestVote RPC  (Raft paper Figure 2)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestVoteRequest {
    /// Candidate's term.
    pub term: u64,
    /// Candidate requesting vote.
    pub candidate_id: NodeId,
    /// Index of candidate's last log entry.
    pub last_log_index: u64,
    /// Term of candidate's last log entry.
    pub last_log_term: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestVoteResponse {
    /// Current term, for candidate to update itself.
    pub term: u64,
    /// True means candidate received vote.
    pub vote_granted: bool,
    /// The responder's id.
    pub from: NodeId,
}

// ---------------------------------------------------------------------------
// Client interaction -- payloads are opaque bytes so the Raft core
// is independent of any particular application.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRequest {
    pub request_id: u64,
    pub command: ClientCommand,
}

/// A client command.  `Query` is read-only and served from the leader
/// without replication.  `Mutate` is written to the replicated log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientCommand {
    /// Read-only query (served directly from leader state).
    Query { payload: Vec<u8> },
    /// Write command to be replicated via the Raft log.
    Mutate { payload: Vec<u8> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientResponse {
    pub request_id: u64,
    pub result: ClientResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientResult {
    /// Success with an optional value payload.
    Ok { value: Option<Vec<u8>> },
    /// An error occurred.
    Error { message: String },
    /// Not the leader -- try this node instead.
    NotLeader { leader_addr: Option<String> },
}
