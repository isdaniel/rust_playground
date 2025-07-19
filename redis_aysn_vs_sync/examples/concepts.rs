use std::time::{Duration, Instant};
use tokio::time::sleep;

/// This example demonstrates the conceptual differences between sync and async
/// operations without requiring Redis to be installed.
///
/// Run with: cargo run --example concepts

#[derive(Debug, Clone)]
struct MockOperation {
    id: usize,
    duration_ms: u64,
}

impl MockOperation {
    fn new(id: usize, duration_ms: u64) -> Self {
        Self { id, duration_ms }
    }

    // Simulate a synchronous blocking operation
    fn execute_sync(&self) -> String {
        std::thread::sleep(Duration::from_millis(self.duration_ms));
        format!("Sync operation {} completed in {}ms", self.id, self.duration_ms)
    }

    // Simulate an asynchronous non-blocking operation
    async fn execute_async(&self) -> String {
        sleep(Duration::from_millis(self.duration_ms)).await;
        format!("Async operation {} completed in {}ms", self.id, self.duration_ms)
    }
}

// Synchronous approach - operations run sequentially
fn sync_operations(operations: Vec<MockOperation>) -> Duration {
    let start = Instant::now();

    println!("🔄 Running {} operations synchronously (sequential)...", operations.len());

    for op in operations {
        let result = op.execute_sync();
        println!("  ✅ {}", result);
    }

    start.elapsed()
}

// Synchronous with threads - operations run in parallel
fn sync_threaded_operations(operations: Vec<MockOperation>) -> Duration {
    use std::sync::Arc;
    use std::thread;

    let start = Instant::now();

    println!("🔄 Running {} operations with threads (parallel)...", operations.len());

    let operations = Arc::new(operations);
    let mut handles = vec![];

    for i in 0..operations.len() {
        let ops = Arc::clone(&operations);
        let handle = thread::spawn(move || {
            let result = ops[i].execute_sync();
            println!("  ✅ {}", result);
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    start.elapsed()
}

// Asynchronous approach - operations run concurrently
async fn async_operations(operations: Vec<MockOperation>) -> Duration {
    let start = Instant::now();

    println!("🔄 Running {} operations asynchronously (concurrent)...", operations.len());

    let mut tasks = vec![];

    for op in operations {
        let task = tokio::spawn(async move {
            let result = op.execute_async().await;
            println!("  ✅ {}", result);
        });
        tasks.push(task);
    }

    // Wait for all tasks to complete
    futures::future::join_all(tasks).await;

    start.elapsed()
}

// Demonstrate different concurrency patterns
async fn demonstrate_patterns() {
    println!("🚀 Demonstrating Sync vs Async Patterns\n");

    // Create mock operations that simulate I/O delays
    let operations = vec![
        MockOperation::new(1, 100),  // 100ms operation
        MockOperation::new(2, 150),  // 150ms operation
        MockOperation::new(3, 200),  // 200ms operation
        MockOperation::new(4, 120),  // 120ms operation
        MockOperation::new(5, 80),   // 80ms operation
    ];

    let expected_sequential_time = operations.iter().map(|op| op.duration_ms).sum::<u64>();
    let expected_concurrent_time = operations.iter().map(|op| op.duration_ms).max().unwrap_or(0);

    println!("Expected times:");
    println!("  Sequential: ~{}ms", expected_sequential_time);
    println!("  Concurrent: ~{}ms", expected_concurrent_time);
    println!();

    // Test 1: Synchronous (sequential)
    let sync_time = sync_operations(operations.clone());
    println!("Sync Sequential Time: {:.2?}\n", sync_time);

    // Test 2: Synchronous with threads (parallel)
    let sync_threaded_time = sync_threaded_operations(operations.clone());
    println!("Sync Threaded Time: {:.2?}\n", sync_threaded_time);

    // Test 3: Asynchronous (concurrent)
    let async_time = async_operations(operations.clone()).await;
    println!("Async Concurrent Time: {:.2?}\n", async_time);

    // Performance summary
    println!("📊 Performance Summary:");
    println!("  Sync Sequential: {:.2?} (baseline)", sync_time);
    println!("  Sync Threaded:  {:.2?} ({:.1}x faster)",
             sync_threaded_time,
             sync_time.as_millis() as f64 / sync_threaded_time.as_millis() as f64);
    println!("  Async Concurrent: {:.2?} ({:.1}x faster)",
             async_time,
             sync_time.as_millis() as f64 / async_time.as_millis() as f64);

    println!("\n💡 Key Insights:");
    println!("  • Sync sequential: Operations block each other");
    println!("  • Sync threaded: Uses OS threads, good for CPU-bound work");
    println!("  • Async concurrent: Uses cooperative multitasking, ideal for I/O");
    println!("  • Async has less overhead than threads for I/O-bound operations");
}

// Demonstrate resource usage differences
async fn demonstrate_resource_usage() {
    println!("\n🔍 Resource Usage Comparison\n");

    // Simulate many concurrent operations
    let operation_count = 1000;
    let operations: Vec<_> = (0..operation_count)
        .map(|i| MockOperation::new(i, 10)) // 10ms each
        .collect();

    println!("Creating {} operations (10ms each)...", operation_count);

    // Async approach: Uses very few OS threads
    let start = Instant::now();
    let mut tasks = vec![];

    for op in operations {
        let task = tokio::spawn(async move {
            op.execute_async().await
        });
        tasks.push(task);
    }

    let _results: Vec<_> = futures::future::join_all(tasks).await;
    let async_time = start.elapsed();

    println!("✅ {} async operations completed in {:.2?}", operation_count, async_time);
    println!("   Resource usage: ~1-2 OS threads (Tokio runtime)");

    // Note: We don't run 1000 OS threads as it would be too resource intensive
    println!("\n💡 If we used 1000 OS threads:");
    println!("   • Each thread uses ~8MB of stack space");
    println!("   • Total memory: ~8GB just for stacks!");
    println!("   • Context switching overhead would be significant");
    println!("   • This is why async is preferred for high-concurrency I/O");
}

#[tokio::main]
async fn main() {
    demonstrate_patterns().await;
    demonstrate_resource_usage().await;

    println!("\n🎯 When to use each approach:");
    println!("  📈 Use Async for:");
    println!("     • High-concurrency applications");
    println!("     • I/O-bound operations (network, file, database)");
    println!("     • Web servers, APIs, real-time systems");
    println!("     • When you need to handle thousands of concurrent operations");

    println!("  🧵 Use Sync + Threads for:");
    println!("     • CPU-bound operations");
    println!("     • When you need true parallelism");
    println!("     • Legacy codebases");
    println!("     • Simple scripts and tools");

    println!("  📊 Use Sync Sequential for:");
    println!("     • Simple, linear workflows");
    println!("     • When operations must happen in order");
    println!("     • Easy debugging and reasoning");
    println!("     • Scripts and batch processing");
}
