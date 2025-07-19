mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand};
use futures::future::join_all;
use rayon::prelude::*;
use redis::{AsyncCommands, Commands};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use utils::stress_tests;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "redis-perf")]
#[command(about = "Redis Async vs Sync Performance Comparison")]
struct Cli {
    #[command(subcommand)]
    command: BenchCommands,
}

#[derive(Subcommand)]
enum BenchCommands {
    /// Run sync Redis operations
    Sync {
        #[arg(short, long, default_value = "1000")]
        operations: usize,
        #[arg(short, long, default_value = "1")]
        threads: usize,
    },
    /// Run async Redis operations
    Async {
        #[arg(short, long, default_value = "1000")]
        operations: usize,
        #[arg(short, long, default_value = "10")]
        concurrent: usize,
    },
    /// Compare both approaches
    Compare {
        #[arg(short, long, default_value = "1000")]
        operations: usize,
    },
    /// Run comprehensive benchmark
    Benchmark,
    /// Run stress tests
    Stress {
        #[arg(short, long, default_value = "5000")]
        operations: usize,
        #[arg(short, long, default_value = "50")]
        max_concurrent: usize,
    },
    /// Test large data handling
    Memory {
        #[arg(short, long, default_value = "1024")]
        value_size_kb: usize,
        #[arg(short, long, default_value = "100")]
        num_keys: usize,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct TestData {
    id: String,
    name: String,
    value: i32,
    timestamp: u64,
}

impl TestData {
    fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: format!("test_{}", rand::random::<u32>()),
            value: rand::random::<i32>() % 1000,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

#[derive(Debug)]
struct BenchmarkResult {
    operation_type: String,
    total_operations: usize,
    total_time: Duration,
    ops_per_second: f64,
    avg_latency_ms: f64,
}

impl BenchmarkResult {
    fn new(operation_type: String, total_operations: usize, total_time: Duration) -> Self {
        let ops_per_second = total_operations as f64 / total_time.as_secs_f64();
        let avg_latency_ms = total_time.as_millis() as f64 / total_operations as f64;

        Self {
            operation_type,
            total_operations,
            total_time,
            ops_per_second,
            avg_latency_ms,
        }
    }

    fn print(&self) {
        println!("=== {} Results ===", self.operation_type);
        println!("Total Operations: {}", self.total_operations);
        println!("Total Time: {:.2?}", self.total_time);
        println!("Operations/Second: {:.2}", self.ops_per_second);
        println!("Average Latency: {:.2} ms", self.avg_latency_ms);
        println!();
    }
}

// Synchronous Redis operations
fn sync_redis_operations(operations: usize, threads: usize) -> Result<BenchmarkResult> {
    println!("Running {} sync operations with {} threads...", operations, threads);

    let start = Instant::now();

    if threads == 1 {
        // Single-threaded sync operations
        let client = redis::Client::open("redis://127.0.0.1/")?;
        let mut conn = client.get_connection()?;

        for i in 0..operations {
            let data = TestData::new();
            let key = format!("sync_key_{}", i);
            let value = serde_json::to_string(&data)?;

            // SET operation
            let _: () = conn.set(&key, &value)?;

            // GET operation
            let _: String = conn.get(&key)?;

            // DELETE operation
            let _: () = conn.del(&key)?;
        }
    } else {
        // Multi-threaded sync operations using rayon
        let ops_per_thread = operations / threads;
        let results: Result<Vec<_>, anyhow::Error> = (0..threads)
            .into_par_iter()
            .map(|thread_id| -> Result<(), anyhow::Error> {
                let client = redis::Client::open("redis://127.0.0.1/")?;
                let mut conn = client.get_connection()?;

                for i in 0..ops_per_thread {
                    let data = TestData::new();
                    let key = format!("sync_key_{}_{}", thread_id, i);
                    let value = serde_json::to_string(&data)?;

                    let _: () = conn.set(&key, &value)?;
                    let _: String = conn.get(&key)?;
                    let _: () = conn.del(&key)?;
                }

                Ok(())
            })
            .collect();

        results?;
    }

    let duration = start.elapsed();
    Ok(BenchmarkResult::new("Sync".to_string(), operations, duration))
}

// Asynchronous Redis operations
async fn async_redis_operations(operations: usize, concurrent: usize) -> Result<BenchmarkResult> {
    println!("Running {} async operations with {} concurrent tasks...", operations, concurrent);

    let start = Instant::now();

    let client = redis::Client::open("redis://127.0.0.1/")?;
    let manager = redis::aio::ConnectionManager::new(client).await?;

    let ops_per_task = operations / concurrent;
    let mut tasks = Vec::new();

    for task_id in 0..concurrent {
        let mut conn = manager.clone();

        let task = tokio::spawn(async move {
            for i in 0..ops_per_task {
                let data = TestData::new();
                let key = format!("async_key_{}_{}", task_id, i);
                let value = serde_json::to_string(&data).unwrap();

                // SET operation
                let _: () = conn.set(&key, &value).await.unwrap();

                // GET operation
                let _: String = conn.get(&key).await.unwrap();

                // DELETE operation
                let _: () = conn.del(&key).await.unwrap();
            }
        });

        tasks.push(task);
    }

    // Wait for all tasks to complete
    join_all(tasks).await;

    let duration = start.elapsed();
    Ok(BenchmarkResult::new("Async".to_string(), operations, duration))
}

// Pipeline operations for better performance
async fn async_pipeline_operations(operations: usize) -> Result<BenchmarkResult> {
    println!("Running {} async pipeline operations...", operations);

    let start = Instant::now();

    let client = redis::Client::open("redis://127.0.0.1/")?;
    let manager = redis::aio::ConnectionManager::new(client).await?;
    let mut conn = manager.clone();

    // Use pipeline for better performance
    let batch_size = 100;
    for batch in 0..(operations / batch_size) {
        let mut pipe = redis::pipe();

        // Prepare batch operations
        for i in 0..batch_size {
            let data = TestData::new();
            let key = format!("pipeline_key_{}_{}", batch, i);
            let value = serde_json::to_string(&data)?;

            pipe.set(&key, &value).ignore();
        }

        // Execute pipeline
        let _: () = pipe.query_async(&mut conn).await?;

        // Read back the values
        let mut pipe = redis::pipe();
        for i in 0..batch_size {
            let key = format!("pipeline_key_{}_{}", batch, i);
            pipe.get(&key);
        }
        let _: Vec<String> = pipe.query_async(&mut conn).await?;

        // Delete the keys
        let mut pipe = redis::pipe();
        for i in 0..batch_size {
            let key = format!("pipeline_key_{}_{}", batch, i);
            pipe.del(&key).ignore();
        }
        let _: () = pipe.query_async(&mut conn).await?;
    }

    let duration = start.elapsed();
    Ok(BenchmarkResult::new("Async Pipeline".to_string(), operations, duration))
}

// Test Redis connection
async fn test_redis_connection() -> Result<()> {
    println!("Testing Redis connection...");

    // Test sync connection
    let client = redis::Client::open("redis://127.0.0.1/")?;
    let mut conn = client.get_connection()?;
    let _: () = conn.set("test_key", "test_value")?;
    let result: String = conn.get("test_key")?;
    assert_eq!(result, "test_value");
    let _: () = conn.del("test_key")?;

    // Test async connection
    let manager = redis::aio::ConnectionManager::new(client).await?;
    let mut async_conn = manager.clone();
    let _: () = async_conn.set("test_async_key", "test_async_value").await?;
    let async_result: String = async_conn.get("test_async_key").await?;
    assert_eq!(async_result, "test_async_value");
    let _: () = async_conn.del("test_async_key").await?;

    println!("✅ Redis connection test passed!");
    Ok(())
}

// Run comprehensive benchmark
async fn run_benchmark() -> Result<()> {
    println!("🚀 Starting comprehensive Redis performance benchmark...\n");

    test_redis_connection().await?;

    let operations = 1000;
    let mut results = Vec::new();

    // Sync single-threaded
    let sync_single = sync_redis_operations(operations, 1)?;
    sync_single.print();
    results.push(sync_single);

    // Sync multi-threaded
    let sync_multi = sync_redis_operations(operations, 4)?;
    sync_multi.print();
    results.push(sync_multi);

    // Async concurrent
    let async_concurrent = async_redis_operations(operations, 10).await?;
    async_concurrent.print();
    results.push(async_concurrent);

    // Async pipeline
    let async_pipeline = async_pipeline_operations(operations).await?;
    async_pipeline.print();
    results.push(async_pipeline);

    // Summary comparison
    println!("=== Performance Summary ===");
    results.sort_by(|a, b| b.ops_per_second.partial_cmp(&a.ops_per_second).unwrap());

    for (i, result) in results.iter().enumerate() {
        println!(
            "{}. {}: {:.2} ops/sec ({:.2} ms avg latency)",
            i + 1,
            result.operation_type,
            result.ops_per_second,
            result.avg_latency_ms
        );
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        BenchCommands::Sync { operations, threads } => {
            let result = sync_redis_operations(operations, threads)?;
            result.print();
        }
        BenchCommands::Async { operations, concurrent } => {
            let result = async_redis_operations(operations, concurrent).await?;
            result.print();
        }
        BenchCommands::Compare { operations } => {
            println!("Comparing sync vs async Redis operations...\n");

            let sync_result = sync_redis_operations(operations, 1)?;
            sync_result.print();

            let async_result = async_redis_operations(operations, 10).await?;
            async_result.print();

            let speedup = async_result.ops_per_second / sync_result.ops_per_second;
            println!("Async is {:.2}x faster than sync", speedup);
        }
        BenchCommands::Benchmark => {
            run_benchmark().await?;
        }
        BenchCommands::Stress { operations, max_concurrent } => {
            println!("🔥 Running stress test with {} operations and max {} concurrent...", operations, max_concurrent);
            let start = Instant::now();
            stress_tests::concurrent_stress_test(operations, max_concurrent).await?;
            let duration = start.elapsed();
            println!("✅ Stress test completed in {:.2?}", duration);
            println!("Average: {:.2} ops/sec", operations as f64 / duration.as_secs_f64());
        }
        BenchCommands::Memory { value_size_kb, num_keys } => {
            println!("🧠 Running memory stress test with {}KB values and {} keys...", value_size_kb, num_keys);
            let start = Instant::now();
            stress_tests::memory_stress_test(value_size_kb * 1024, num_keys).await?;
            let duration = start.elapsed();
            println!("✅ Memory test completed in {:.2?}", duration);
            let total_data_mb = (value_size_kb * num_keys) as f64 / 1024.0;
            println!("Processed {:.2} MB of data", total_data_mb);
        }
    }

    Ok(())
}
