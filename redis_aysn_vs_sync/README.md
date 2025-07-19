# Redis Async vs Sync Performance Comparison

This Rust project demonstrates and benchmarks the performance differences between synchronous and asynchronous Redis operations, including concurrent execution patterns.

## ✅ **Status: Fixed and Working!**

All compilation errors have been resolved:
- ✅ Fixed naming conflict between `Commands` enum and Redis `Commands` trait
- ✅ Fixed type inference issues in error handling
- ✅ Fixed missing `Clone` derive in examples
- ✅ Cleaned up unused imports

## 🚀 Features

- **Synchronous Redis Operations**: Single-threaded and multi-threaded using Rayon
- **Asynchronous Redis Operations**: Concurrent tasks using Tokio
- **Pipeline Operations**: Batched async operations for improved throughput
- **Performance Benchmarking**: Detailed metrics and comparisons
- **Real-world Test Data**: JSON serialization/deserialization of structured data
- **Conceptual Demo**: Understand async concepts without requiring Redis

## Quick Start (Windows)

### ✨ **Easiest: Test Everything (Automated)**
```powershell
# Run automated test (includes conceptual demo + Redis tests if available)
.\test_project.ps1
```

### Option 1: Run Without Redis (Conceptual Demo)
If you want to understand the concepts without setting up Redis:
```powershell
# Run the conceptual demonstration
cargo run --example concepts
```
This will show you the performance differences between sync and async approaches using mock operations.

### Option 2: Quick Redis Benchmark
```powershell
# Build and run (assumes Redis is running on localhost:6379)
cargo build --release
cargo run --release -- benchmark
```

### Option 3: Using PowerShell Script (Full Redis Benchmark)
```powershell
# Run the comprehensive test suite
.\run_tests.ps1
```

### Option 4: Using Docker (Full Redis Benchmark)
```powershell
# Start Redis with Docker
docker-compose up -d redis

# Run tests
cargo run --release -- benchmark

# Stop Redis
docker-compose down
```

## Usage

### Build the project
```bash
cargo build --release
```

### Run different benchmark modes

#### 1. Synchronous Operations
```bash
# Single-threaded sync (1000 operations)
cargo run -- sync --operations 1000 --threads 1

# Multi-threaded sync (1000 operations, 4 threads)
cargo run -- sync --operations 1000 --threads 4
```

#### 2. Asynchronous Operations
```bash
# Async with 10 concurrent tasks (1000 operations)
cargo run -- async --operations 1000 --concurrent 10

# Async with 50 concurrent tasks
cargo run -- async --operations 1000 --concurrent 50
```

#### 3. Quick Comparison
```bash
# Compare sync vs async performance
cargo run -- compare --operations 1000
```

#### 4. Comprehensive Benchmark
```bash
# Run all tests and show performance summary
cargo run -- benchmark
```

## Example Output

```
🚀 Starting comprehensive Redis performance benchmark...

Testing Redis connection...
✅ Redis connection test passed!

Running 1000 sync operations with 1 threads...
=== Sync Results ===
Total Operations: 1000
Total Time: 2.34s
Operations/Second: 427.35
Average Latency: 2.34 ms

Running 1000 sync operations with 4 threads...
=== Sync Multi-threaded Results ===
Total Operations: 1000
Total Time: 0.89s
Operations/Second: 1123.60
Average Latency: 0.89 ms

Running 1000 async operations with 10 concurrent tasks...
=== Async Results ===
Total Operations: 1000
Total Time: 0.45s
Operations/Second: 2222.22
Average Latency: 0.45 ms

Running 1000 async pipeline operations...
=== Async Pipeline Results ===
Total Operations: 1000
Total Time: 0.12s
Operations/Second: 8333.33
Average Latency: 0.12 ms

=== Performance Summary ===
1. Async Pipeline: 8333.33 ops/sec (0.12 ms avg latency)
2. Async: 2222.22 ops/sec (0.45 ms avg latency)
3. Sync Multi-threaded: 1123.60 ops/sec (0.89 ms avg latency)
4. Sync: 427.35 ops/sec (2.34 ms avg latency)
```

## Key Performance Insights

### 1. **Async Pipeline** (Fastest)
- **Best for**: High-throughput batch operations
- **Performance**: ~8000+ ops/sec
- **Use Case**: Bulk data loading, batch processing

### 2. **Async Concurrent** (Very Fast)
- **Best for**: I/O intensive applications with many concurrent operations
- **Performance**: ~2000+ ops/sec
- **Use Case**: Web applications, real-time systems

### 3. **Sync Multi-threaded** (Good)
- **Best for**: CPU-bound operations with some parallelism
- **Performance**: ~1000+ ops/sec
- **Use Case**: Traditional multi-threaded applications

### 4. **Sync Single-threaded** (Baseline)
- **Best for**: Simple, sequential operations
- **Performance**: ~400+ ops/sec
- **Use Case**: Scripts, simple tools

## Project Structure

```
src/
├── main.rs           # Main application with all benchmark implementations
├── ...
Cargo.toml           # Dependencies and project configuration
README.md           # This file
```

## Dependencies

- `redis`: Redis client with async support
- `tokio`: Async runtime
- `rayon`: Data parallelism for sync operations
- `serde/serde_json`: Data serialization
- `clap`: Command-line interface
- `uuid`: Unique identifiers for test data
- `anyhow`: Error handling

## Understanding the Results

### Operations per Second (ops/sec)
Higher is better. This measures throughput.

### Average Latency (ms)
Lower is better. This measures individual operation speed.

### When to Use Each Approach

**Use Async When:**
- High concurrency requirements
- I/O bound operations
- Building web services or real-time applications
- Need to handle many simultaneous connections

**Use Sync When:**
- Simple, sequential operations
- CPU-bound tasks
- Legacy codebases
- Easier debugging and reasoning

**Use Pipelines When:**
- Batch operations
- High-throughput data processing
- Minimizing network round trips

## Performance Tips

1. **Connection Pooling**: Use `ConnectionManager` for async operations
2. **Pipelining**: Batch operations when possible
3. **Proper Concurrency**: Don't over-parallelize (test optimal concurrent task count)
4. **Network Latency**: Results will vary based on network conditions
5. **Redis Configuration**: Tune Redis settings for your workload

## Testing Different Scenarios

Try these variations to see how performance changes:
```bash
# Test with different operation counts
cargo run -- benchmark  # Uses 1000 operations
cargo run -- async --operations 5000 --concurrent 20

# Test with different concurrency levels
cargo run -- async --operations 1000 --concurrent 5
cargo run -- async --operations 1000 --concurrent 25
cargo run -- async --operations 1000 --concurrent 100
```
