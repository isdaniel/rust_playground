use std::fmt;

pub type Result<T> = std::result::Result<T, RaftError>;

#[derive(Debug)]
pub enum RaftError {
    /// The node is not the leader; includes the leader id if known.
    NotLeader(Option<u64>),
    /// IO / network error.
    Io(std::io::Error),
    /// Serialization error.
    Serde(serde_json::Error),
    /// Timeout waiting for a response.
    Timeout,
    /// Generic internal error.
    Internal(String),
}

impl fmt::Display for RaftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RaftError::NotLeader(id) => write!(f, "not leader, leader may be {:?}", id),
            RaftError::Io(e) => write!(f, "io error: {}", e),
            RaftError::Serde(e) => write!(f, "serde error: {}", e),
            RaftError::Timeout => write!(f, "timeout"),
            RaftError::Internal(msg) => write!(f, "internal: {}", msg),
        }
    }
}

impl std::error::Error for RaftError {}

impl From<std::io::Error> for RaftError {
    fn from(e: std::io::Error) -> Self {
        RaftError::Io(e)
    }
}

impl From<serde_json::Error> for RaftError {
    fn from(e: serde_json::Error) -> Self {
        RaftError::Serde(e)
    }
}
