# Exporter Service with Active/Standby Failover

A Rust implementation of an industrial metrics exporter daemon with **Active/Standby high-availability failover**, built on Kafka consumer group rebalancing.

## Architecture Overview

```
                    ┌─────────────────────┐
                    │    Kafka Cluster     │
                    │  (metrics topics +   │
                    │   __exporter_leader) │
                    └──────┬──────┬───────┘
                           │      │
              ┌────────────┘      └────────────┐
              │                                 │
              ▼                                 ▼
    ┌──────────────────┐             ┌──────────────────┐
    │ Exporter Active  │             │ Exporter Standby │
    │ (Server-A)       │             │ (Server-B)       │
    │                  │             │                  │
    │ ┌──────────────┐ │             │ ┌──────────────┐ │
    │ │ Leader       │ │             │ │ Leader       │ │
    │ │ Election     │ │             │ │ Election     │ │
    │ ├──────────────┤ │             │ ├──────────────┤ │
    │ │ Realtime     │ │             │ │ (waiting for │ │
    │ │ Consumer     │ │             │ │  partition)  │ │
    │ ├──────────────┤ │             │ │              │ │
    │ │ Backfill     │ │             │ └──────────────┘ │
    │ │ Engine       │ │             │                  │
    │ ├──────────────┤ │             │ HTTP /health     │
    │ │ Health       │ │             │ HTTP /status     │
    │ │ Monitor      │ │             │                  │
    │ ├──────────────┤ │             │                  │
    │ │ HTTP API     │ │             │                  │
    │ └──────────────┘ │             └──────────────────┘
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐
    │  Cloud Endpoint  │
    │  (AWS / Mock)    │
    └──────────────────┘
```

## Core Design Principles

### 1. Leader Election via Kafka Consumer Group

The most critical design choice: we **do not** use external coordination services (ZooKeeper, etcd, Redis). Instead, we leverage Kafka's built-in consumer group protocol:

- Both exporter instances join the consumer group `"exporter-leader"`
- They subscribe to a **single-partition** topic `__exporter_leader`
- Kafka guarantees that only **one consumer** in a group can own a given partition
- The consumer that receives the partition assignment = **Active**
- If Active crashes, Kafka triggers a **rebalance** and assigns the partition to Standby

**Why this works:**
- No additional infrastructure needed (Kafka already exists)
- Automatic failover in ~10-15 seconds (session timeout + rebalance time)
- Battle-tested protocol (Kafka consumer group rebalance)
- Offset tracking is free

### 2. Connection State Machine

```
                     WAN down (3 failures)
    ┌───────────┐ ──────────────────────> ┌──────────────┐
    │ CONNECTED │                         │ DISCONNECTED │
    │           │ <────────────────────── │              │
    └───────────┘     backfill complete   └──────┬───────┘
         ▲                                       │
         │              WAN recovered            │
         │                                       ▼
         │                                ┌──────────────┐
         └─────────────────────────────── │ BACKFILLING  │
                   caught up              └──────────────┘
```

- **CONNECTED**: Normal operation. Realtime consumer sends micro-batches to cloud.
- **DISCONNECTED**: WAN down. Consumers pause. Kafka buffers data automatically.
- **BACKFILLING**: WAN recovered. Realtime resumes + backfill engine replays missed data at 30% bandwidth cap.

### 3. At-Least-Once Delivery

- Kafka offset is committed **only after** the cloud endpoint confirms receipt
- If the exporter crashes between send and commit, messages will be re-sent
- Cloud endpoint deduplicates using `(equipment_id, timestamp, metric_id)` as the key

### 4. Adaptive Micro-Batching

| State | Batch Size | Flush Interval | Purpose |
|-------|-----------|----------------|---------|
| CONNECTED | 100 records | 5 seconds | Low latency |
| BACKFILLING | 500 records | 15 seconds | Leave bandwidth for backfill |
| DISCONNECTED | (paused) | (paused) | Kafka buffers |


## Module Details

### `leader.rs` - Leader Election

```rust
// Key Kafka consumer config for leader election:
.set("group.id", "exporter-leader")          // Both instances use same group
.set("session.timeout.ms", "10000")          // 10s until Kafka detects failure
.set("heartbeat.interval.ms", "3000")        // 3s heartbeat frequency
.set("partition.assignment.strategy", "range") // Deterministic assignment
```

**Failover timeline:**
1. `T+0s`: Active exporter crashes
2. `T+3s`: Kafka notices missed heartbeat
3. `T+10s`: Session timeout expires, Kafka triggers rebalance
4. `T+12s`: Standby receives partition assignment
5. `T+12s`: Standby promotes to Active, starts worker tasks

### `health.rs` - Health Monitor

Periodically pings the cloud endpoint (HTTP HEAD request). Three consecutive failures trigger the DISCONNECTED state. Recovery triggers BACKFILLING and wakes up the backfill engine.

### `realtime.rs` - Realtime Consumer

- Uses consumer group `"rt-metrics"` (same group ID on both instances)
- On failover, Kafka rebalance assigns partitions to the new Active
- Resumes from the **last committed offset** (no data loss)
- Subscribes to `metrics.alarm`, `metrics.key`, `metrics.raw`

### `backfill.rs` - Backfill Engine

- Independent consumer group `"backfill"` (does not affect realtime offsets)
- **Token bucket rate limiter**: caps at 30% of 1 Gbps = 37.5 MB/s
- Priority sorting: alarm data first, then key, then raw
- Auto-completes when no more data to replay

### `api.rs` - HTTP API

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET/HEAD | Returns 200 OK (for load balancers) |
| `/status` | GET | JSON: instance_id, fab_id, role, connection_state |
| `/metrics` | GET | Prometheus-format metrics |

## Running the Demo

### Prerequisites

- Docker and Docker Compose v2
- ~4 GB free RAM (for Kafka + 2 exporters)
- Ports 8080, 9091, 9092, 9094 available

### Quick Start

```bash
cd exporter_failover

# Build and start all services
docker compose up --build -d

# Watch logs
docker compose logs -f exporter-active exporter-standby

# Check status
curl http://localhost:9091/status   # Active exporter
curl http://localhost:9092/status   # Standby exporter
curl http://localhost:8080/admin/stats  # Mock AWS stats
```

### Automated Failover Demo

```bash
chmod +x scripts/demo-failover.sh
./scripts/demo-failover.sh
```

This script automatically:
1. Starts all services
2. Verifies Active is processing metrics
3. Kills the Active exporter
4. Verifies Standby takes over within ~15 seconds
5. Restarts the original Active (becomes Standby)
6. Simulates WAN disconnect/reconnect

### Manual Failover Test

```bash
# Step 1: Check who is Active
curl -s http://localhost:9091/status | python -m json.tool
curl -s http://localhost:9092/status | python -m json.tool

# Step 2: Kill the Active exporter
docker compose stop exporter-active

# Step 3: Wait ~15 seconds for Kafka rebalance

# Step 4: Verify Standby is now Active
curl -s http://localhost:9092/status | python -m json.tool

# Step 5: Restart original (becomes Standby)
docker compose start exporter-active
```

### WAN Disconnect Simulation

```bash
# Simulate WAN outage
curl -X POST http://localhost:8080/admin/disconnect

# Check exporter state (should transition to DISCONNECTED)
sleep 20
curl -s http://localhost:9091/status | python -m json.tool

# Restore WAN
curl -X POST http://localhost:8080/admin/connect

# Check backfill progress
sleep 10
curl -s http://localhost:9091/status | python -m json.tool
curl -s http://localhost:8080/admin/stats | python -m json.tool
```

### Cleanup

```bash
docker compose down -v
```

## Failover Mechanism Detail

### Why Kafka Consumer Group?

| Approach | Pros | Cons |
|----------|------|------|
| **Kafka Consumer Group** | No extra infra, automatic, battle-tested | Failover time ~10-15s |
| ZooKeeper | Sub-second failover | Extra infra to maintain |
| etcd / Consul | Rich feature set | Extra infra, complexity |
| File lock on NAS | Simple | Requires shared storage |
| Heartbeat-only | Customizable | Split-brain risk |

We chose Kafka Consumer Group because:
1. **Kafka already exists** in the architecture (for metric buffering)
2. **Zero additional infrastructure** to deploy and maintain
3. **Automatic rebalance** handles both planned and unplanned failovers
4. **Offset tracking** gives us at-least-once delivery for free

### Split-Brain Prevention

Kafka's consumer group protocol inherently prevents split-brain:
- Only ONE consumer in a group can own a partition
- If two consumers both think they are Active, the one without the partition will fail to receive messages from `__exporter_leader`
- The `assignment()` check in the leader election loop detects and corrects stale state

### Data Durability Guarantees

```
┌─────────────────────────────────────────────────────────────┐
│  Defense 1:  Kafka Retention (24h, 6TB per cluster)         │
│  Defense 2:  Consumer Group Offset (committed after ACK)    │
│  Defense 3:  Cloud-side Dedup (equipment_id + timestamp)    │
│                                                             │
│  Worst case: a few uncommitted messages are re-sent         │
│  Result: at-least-once delivery (never data loss)           │
└─────────────────────────────────────────────────────────────┘
```

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `KAFKA_BROKERS` | `kafka:9092` | Comma-separated Kafka broker addresses |
| `AWS_ENDPOINT` | `http://mock-aws:8080` | Cloud endpoint URL |
| `FAB_ID` | `TW-1` | Factory identifier |
| `INSTANCE_ID` | `exporter-1` | Unique instance identifier |
| `PEER_ENDPOINT` | `exporter-standby:9090` | Peer exporter address |
| `HTTP_PORT` | `9090` | HTTP API listen port |
| `RUST_LOG` | - | Log level filter (e.g., `info`, `debug`) |

### YAML Configuration (Alternative)

```yaml
kafka_brokers:
  - "kafka-1:9092"
  - "kafka-2:9092"
  - "kafka-3:9092"
aws_endpoint: "https://nlb-xxxx.vpc.amazonaws.com"
fab_id: "TW-1"
instance_id: "exporter-a"
normal_batch_size: 5000
normal_flush_secs: 5
slow_batch_size: 50000
slow_flush_secs: 30
backfill_bandwidth_cap_pct: 30
peer_endpoint: "server-b:8080"
heartbeat_interval_secs: 5
failover_timeout_secs: 30
http_port: 9090
```

## Key Kafka Topics

| Topic | Partitions | Purpose |
|-------|-----------|---------|
| `__exporter_leader` | **1** | Leader election (must be exactly 1 partition) |
| `metrics.alarm` | 3 | High-priority alarm data (P0) |
| `metrics.key` | 3 | Key performance indicators (P1) |
| `metrics.raw` | 6 | Raw sensor data (P2) |
