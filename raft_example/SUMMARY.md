# Raft Consensus Algorithm -- Rust Implementation

A Raft consensus protocol implementation in Rust, structured as a reusable library (`raft_core`) with a distributed key/value store application (`raft_kv`) built on top.

## Project Structure

```
raft_example/
+-- Cargo.toml                  # Workspace root
+-- Dockerfile                  # Multi-stage build for all binaries
+-- docker-compose.yml          # 3-node cluster for manual testing
+-- docker-compose.test.yml     # 3-node cluster + automated test runner
+-- config/
|   +-- node1.json              # Node configuration files
|   +-- node2.json
|   +-- node3.json
+-- raft_core/                  # Reusable Raft consensus library
|   +-- Cargo.toml
|   +-- src/
|       +-- lib.rs              # Crate root: re-exports all modules
|       +-- config.rs           # NodeId, RaftConfig, PeerConfig
|       +-- error.rs            # RaftError, Result type alias
|       +-- log.rs              # LogEntry (opaque command payload)
|       +-- node.rs             # RaftNode<S: StateMachine> -- core consensus engine
|       +-- rpc.rs              # Wire protocol: AppendEntries, RequestVote, Client RPCs
|       +-- state_machine.rs    # StateMachine trait (apply + query)
|       +-- storage.rs          # PersistentState, file-backed Storage
|       +-- transport.rs        # TCP transport: length-prefixed JSON framing
+-- raft_kv/                    # K/V store application
|   +-- Cargo.toml
|   +-- src/
|       +-- lib.rs              # Crate root
|       +-- kv.rs               # KvStateMachine, KvCommand, KvQuery, KvResult
|       +-- bin/
|           +-- server.rs       # raft-server: starts a Raft node
|           +-- client.rs       # raft-client: interactive CLI
|           +-- test_client.rs  # raft-test-client: scripted testing CLI
+-- tests/
    +-- run_all_tests.sh        # Master test runner
    +-- helpers.sh              # Shared test utilities
    +-- test_01_normal_workflow.sh
    +-- test_02_leader_crash.sh
    +-- test_03_consensus.sh
```

## Architecture

### Two-Crate Workspace Design

| Crate | Purpose | Dependencies |
|---|---|---|
| **`raft_core`** | Protocol-agnostic Raft consensus library | tokio, serde, serde_json, rand, tracing |
| **`raft_kv`** | K/V store application using raft_core | raft_core, tokio, serde, serde_json, tracing, tracing-subscriber |

The core library defines a **`StateMachine` trait** with two methods:
- `apply(&mut self, command: &Option<Vec<u8>>) -> Option<Vec<u8>>` -- apply a committed log entry
- `query(&self, query: &[u8]) -> Option<Vec<u8>>` -- read-only query (not replicated)

Log entries carry **opaque `Vec<u8>` payloads** (or `None` for protocol-level no-ops), keeping `raft_core` independent of any application-specific command format.

### Raft Protocol Implementation

The implementation follows the [Raft paper](https://raft.github.io/raft.pdf) (Ongaro & Ousterhout), specifically Figure 2.

| Feature | Status | Key Location |
|---|---|---|
| Leader Election | Implemented | `raft_core/src/node.rs` -- `start_election`, `handle_request_vote` |
| Log Replication | Implemented | `raft_core/src/node.rs` -- `send_append_entries_to_all`, `handle_append_entries` |
| Safety (log up-to-date check) | Implemented | `raft_core/src/node.rs` -- `is_log_up_to_date` (Section 5.4.1) |
| Commit Index Advancement | Implemented | `raft_core/src/node.rs` -- `advance_commit_index` |
| No-op on New Leader | Implemented | `raft_core/src/node.rs` -- `become_leader` (Section 5.4.2) |
| Persistent State | Implemented | `raft_core/src/storage.rs` -- term, votedFor, log persisted to JSON |
| Fast Log Back-up | Implemented | `raft_core/src/node.rs` -- uses `last_log_index` hint on conflict |
| Randomized Election Timeout | Implemented | `raft_core/src/node.rs` -- `new_election_deadline` |
| Client Redirect | Implemented | `raft_core/src/node.rs` -- returns `NotLeader` with leader address |
| Log Compaction / Snapshots | Not implemented | -- |
| Cluster Membership Changes | Not implemented | -- |
| Linearizable Reads | Not implemented | Reads served from leader state without read-index protocol |
| Pre-vote Protocol | Not implemented | -- |

### Wire Protocol

All communication uses **length-prefixed JSON over TCP**:
- 4-byte big-endian length header
- JSON payload (max 16 MB)
- One request per TCP connection (simple, no multiplexing)

### K/V Application Layer

The `raft_kv` crate provides:

- **`KvCommand`** -- `Set { key, value }` and `Delete { key }` (serialized to/from the opaque log payload)
- **`KvQuery`** -- `Get { key }` (read-only, not replicated)
- **`KvStateMachine`** -- in-memory `HashMap<String, String>` implementing `StateMachine`
- **`KvResult`** -- `Value(Option<String>)` response wrapper

Three binaries:
- **`raft-server`** -- starts a Raft node with the K/V state machine
- **`raft-client`** -- interactive REPL with auto-redirect to leader
- **`raft-test-client`** -- non-interactive CLI for scripted tests (exit codes: 0=success, 1=error, 2=not-leader)

## Running

### Local Development

```bash
cargo build --release
```

### Docker Cluster (Manual)

```bash
docker compose up --build
```

Starts a 3-node cluster:
- Node 1: `localhost:9001`
- Node 2: `localhost:9002`
- Node 3: `localhost:9003`

Connect with the interactive client:
```bash
cargo run --release --bin raft-client -- 127.0.0.1:9001
```

### Docker Integration Tests

```bash
docker compose -f docker-compose.test.yml up --build --abort-on-container-exit
```

## Test Results

All **52 tests pass** across 3 test suites:

### Test 01: Normal Workflow (14 tests)
- Leader election verification
- Basic Set/Get operations
- Key updates
- Multiple key storage and retrieval
- Delete operations
- Non-existent key queries
- Follower redirect (client auto-follows `NotLeader` to actual leader)
- Bulk write (10 keys) and read consistency

### Test 02: Leader Crash & Data Consistency (18 tests)
- Pre-crash data writes and verification
- Leader crash simulation (Docker container pause)
- New leader election from remaining nodes
- Pre-crash data survives on new leader (no data loss)
- New writes succeed while old leader is down
- Pre-crash key updates on new leader
- Old leader recovery and data convergence
- Post-recovery cluster operational verification

### Test 03: Raft Consensus Algorithm (20 tests)
- **Minority partition**: 1 follower isolated, cluster continues with 2/3 majority
- **No write quorum**: both followers isolated, leader alone correctly rejects writes (timeout)
- **Rapid leadership changes**: crash leader #1, elect #2, crash #2, elect #3 -- data survives
- **Final convergence**: all historical data intact after all partitions healed
- **Concurrent writes**: 20 parallel writes under stable leadership, all committed
- **Delete and re-create**: key lifecycle (set -> delete -> set) works correctly

### Summary

| Suite | Passed | Failed |
|---|---|---|
| Test 01: Normal Workflow | 14 | 0 |
| Test 02: Leader Crash & Data Consistency | 18 | 0 |
| Test 03: Raft Consensus Algorithm | 20 | 0 |
| **Total** | **52** | **0** |

## Key Design Decisions

1. **Generic StateMachine trait** -- `raft_core` is decoupled from K/V specifics. To build a different replicated application, implement `StateMachine` and use `RaftNode<YourStateMachine>`.

2. **Opaque command bytes** -- Log entries carry `Option<Vec<u8>>` (`None` = no-op). This keeps the Raft protocol layer completely unaware of application semantics.

3. **One-connection-per-RPC** -- Simple TCP model without connection pooling. Adequate for educational purposes and moderate throughput.

4. **JSON persistence** -- Persistent state is stored as JSON files. Not optimal for production but easy to debug and inspect.

5. **Container pause for crash simulation** -- Docker `pause`/`unpause` freezes all processes in a container, faithfully simulating a complete node crash without losing filesystem state.

## Extending

To build a new application on `raft_core`:

```rust
use raft_core::state_machine::StateMachine;

struct MyStateMachine { /* ... */ }

impl StateMachine for MyStateMachine {
    fn apply(&mut self, command: &Option<Vec<u8>>) -> Option<Vec<u8>> {
        // Deserialize and apply your command
    }

    fn query(&self, query: &[u8]) -> Option<Vec<u8>> {
        // Handle read-only queries
    }
}

// Then create a RaftNode<MyStateMachine> and run it.
```
