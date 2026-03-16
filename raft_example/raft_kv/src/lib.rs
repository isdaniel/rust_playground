//! `raft_kv` -- A distributed key/value store built on `raft_core`.
//!
//! This crate provides the K/V-specific command types, a `KvStateMachine`
//! implementing `raft_core::state_machine::StateMachine`, and three
//! binaries (`raft-server`, `raft-client`, `raft-test-client`).

pub mod kv;
