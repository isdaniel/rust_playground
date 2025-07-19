// Additional utility functions and test helpers
use redis::{AsyncCommands, Commands, RedisResult};
use std::time::Instant;

pub struct RedisTestHelper;

impl RedisTestHelper {
    /// Clean up test keys from Redis
    pub fn cleanup_sync(conn: &mut redis::Connection, pattern: &str) -> RedisResult<()> {
        let keys: Vec<String> = redis::cmd("KEYS").arg(pattern).query(conn)?;
        if !keys.is_empty() {
            let _: () = conn.del(&keys)?;
        }
        Ok(())
    }

    /// Clean up test keys from Redis (async version)
    pub async fn cleanup_async(
        conn: &mut redis::aio::ConnectionManager,
        pattern: &str,
    ) -> RedisResult<()> {
        let keys: Vec<String> = redis::cmd("KEYS").arg(pattern).query_async(conn).await?;
        if !keys.is_empty() {
            let _: () = conn.del(&keys).await?;
        }
        Ok(())
    }

    /// Get Redis server info
    pub fn get_server_info(conn: &mut redis::Connection) -> RedisResult<String> {
        redis::cmd("INFO").arg("server").query(conn)
    }

    /// Measure operation latency
    pub async fn measure_latency<F, T>(operation: F) -> (T, std::time::Duration)
    where
        F: std::future::Future<Output = T>,
    {
        let start = Instant::now();
        let result = operation.await;
        let duration = start.elapsed();
        (result, duration)
    }
}

/// Test data structures for different scenarios
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct UserProfile {
    pub id: u64,
    pub username: String,
    pub email: String,
    pub created_at: u64,
    pub last_login: Option<u64>,
    pub preferences: std::collections::HashMap<String, String>,
}

impl UserProfile {
    pub fn new(id: u64) -> Self {
        let mut preferences = std::collections::HashMap::new();
        preferences.insert("theme".to_string(), "dark".to_string());
        preferences.insert("language".to_string(), "en".to_string());

        Self {
            id,
            username: format!("user_{}", id),
            email: format!("user_{}@example.com", id),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_login: None,
            preferences,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct CacheEntry {
    pub key: String,
    pub value: String,
    pub ttl: Option<u64>,
    pub created_at: u64,
}

impl CacheEntry {
    pub fn new(key: String, value: String, ttl: Option<u64>) -> Self {
        Self {
            key,
            value,
            ttl,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

/// Stress test functions
pub mod stress_tests {
    use super::*;
    use redis::AsyncCommands;
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    /// Run a stress test with controlled concurrency
    pub async fn concurrent_stress_test(
        operations: usize,
        max_concurrent: usize,
    ) -> anyhow::Result<()> {
        let client = redis::Client::open("redis://127.0.0.1/")?;
        let manager = redis::aio::ConnectionManager::new(client).await?;
        let semaphore = Arc::new(Semaphore::new(max_concurrent));

        let tasks: Vec<_> = (0..operations)
            .map(|i| {
                let sem = semaphore.clone();
                let mut conn = manager.clone();
                tokio::spawn(async move {
                    let _permit = sem.acquire().await.unwrap();

                    let user = UserProfile::new(i as u64);
                    let key = format!("stress_user_{}", i);
                    let value = serde_json::to_string(&user).unwrap();

                    // SET with expiration
                    let _: () = conn.set_ex(&key, &value, 60).await.unwrap();

                    // GET
                    let _: String = conn.get(&key).await.unwrap();

                    // UPDATE (simulate user login)
                    let mut updated_user = user;
                    updated_user.last_login = Some(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    );
                    let updated_value = serde_json::to_string(&updated_user).unwrap();
                    let _: () = conn.set_ex(&key, &updated_value, 60).await.unwrap();

                    // DELETE
                    let _: () = conn.del(&key).await.unwrap();
                })
            })
            .collect();

        futures::future::join_all(tasks).await;
        Ok(())
    }

    /// Memory usage stress test
    pub async fn memory_stress_test(
        large_value_size: usize,
        num_keys: usize,
    ) -> anyhow::Result<()> {
        let client = redis::Client::open("redis://127.0.0.1/")?;
        let manager = redis::aio::ConnectionManager::new(client).await?;
        let mut conn = manager.clone();

        // Generate large value
        let large_value = "x".repeat(large_value_size);

        println!("Storing {} keys with {}KB values each...", num_keys, large_value_size / 1024);

        for i in 0..num_keys {
            let key = format!("large_key_{}", i);
            let _: () = conn.set_ex(&key, &large_value, 300).await?; // 5 min TTL
        }

        // Verify we can read them back
        for i in 0..num_keys {
            let key = format!("large_key_{}", i);
            let _: String = conn.get(&key).await?;
        }

        // Cleanup
        for i in 0..num_keys {
            let key = format!("large_key_{}", i);
            let _: () = conn.del(&key).await?;
        }

        Ok(())
    }
}
