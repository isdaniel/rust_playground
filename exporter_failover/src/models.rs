use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single metric record from a factory equipment sensor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricRecord {
    pub timestamp: DateTime<Utc>,
    pub equipment_id: String,
    pub metric_id: String,
    pub value: f64,
    pub unit: String,
    pub line_id: String,
}

/// A batch of metric records to be sent to the cloud.
#[derive(Debug, Serialize)]
pub struct MetricsBatch {
    pub fab_id: String,
    pub batch_id: String,
    pub records: Vec<MetricRecord>,
    pub compressed: bool,
}
