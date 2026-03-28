use serde::Deserialize;

/// Exporter configuration, loaded from config.yaml or environment variables.
#[derive(Debug, Clone, Deserialize)]
pub struct ExporterConfig {
    pub kafka_brokers: Vec<String>,
    pub aws_endpoint: String,
    pub fab_id: String,
    pub instance_id: String,

    // Batch settings
    pub normal_batch_size: usize,
    pub normal_flush_secs: u64,
    pub slow_batch_size: usize,
    pub slow_flush_secs: u64,

    // Backfill settings
    pub backfill_bandwidth_cap_pct: u8,

    // HA settings
    pub heartbeat_interval_secs: u64,
    pub failover_timeout_secs: u64,
    pub leader_claim_interval_secs: u64,

    // HTTP server port for health/metrics
    pub http_port: u16,

    // Peer endpoint for health check before promotion (e.g., "exporter-standby:9090")
    #[serde(default)]
    pub peer_endpoint: Option<String>,

    // Grace period (seconds) at startup before allowing promotion.
    // Default: leader_claim_interval_secs * 2 + 2
    #[serde(default)]
    pub startup_grace_secs: u64,
}

impl ExporterConfig {
    /// Load config from a YAML file path.
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: ExporterConfig = serde_yaml::from_str(&contents)?;
        Ok(config)
    }

    /// Load config from environment variables (for Docker).
    pub fn from_env() -> anyhow::Result<Self> {
        let brokers = std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "kafka:9092".into());
        let aws_endpoint =
            std::env::var("AWS_ENDPOINT").unwrap_or_else(|_| "http://mock-aws:8080".into());
        let fab_id = std::env::var("FAB_ID").unwrap_or_else(|_| "TW-1".into());
        let instance_id =
            std::env::var("INSTANCE_ID").unwrap_or_else(|_| "exporter-1".into());
        let http_port: u16 = std::env::var("HTTP_PORT")
            .unwrap_or_else(|_| "9090".into())
            .parse()
            .unwrap_or(9090);
        let failover_timeout_secs: u64 = std::env::var("FAILOVER_TIMEOUT_SECS")
            .unwrap_or_else(|_| "15".into())
            .parse()
            .unwrap_or(15);
        let leader_claim_interval_secs: u64 = std::env::var("LEADER_CLAIM_INTERVAL_SECS")
            .unwrap_or_else(|_| "3".into())
            .parse()
            .unwrap_or(3);

        let peer_endpoint = std::env::var("PEER_ENDPOINT").ok().and_then(|v| {
            let trimmed = v.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        });

        let default_grace = leader_claim_interval_secs * 2 + 2;
        let startup_grace_secs: u64 = std::env::var("STARTUP_GRACE_SECS")
            .unwrap_or_else(|_| default_grace.to_string())
            .parse()
            .unwrap_or(default_grace);

        Ok(ExporterConfig {
            kafka_brokers: brokers.split(',').map(|s| s.trim().to_string()).collect(),
            aws_endpoint,
            fab_id,
            instance_id,
            normal_batch_size: 100,
            normal_flush_secs: 5,
            slow_batch_size: 500,
            slow_flush_secs: 15,
            backfill_bandwidth_cap_pct: 30,
            heartbeat_interval_secs: 5,
            failover_timeout_secs,
            leader_claim_interval_secs,
            http_port,
            peer_endpoint,
            startup_grace_secs,
        })
    }
}
