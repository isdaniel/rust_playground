//! `raft_core` -- A reusable Raft consensus protocol library.
//!
//! This crate provides the core Raft algorithm: leader election, log
//! replication, persistence and a TCP-based transport layer.  The state
//! machine is abstracted behind the [`StateMachine`] trait so the same
//! engine can drive any replicated application (key/value store, queue,
//! configuration store, etc.).

pub mod config;
pub mod error;
pub mod log;
pub mod node;
pub mod rpc;
pub mod state_machine;
pub mod storage;
pub mod transport;
