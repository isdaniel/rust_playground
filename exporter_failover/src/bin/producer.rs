/// Kafka metric producer for testing.
/// Produces random metric records to Kafka topics.
/// Also produces a periodic heartbeat to the __exporter_leader topic.
use chrono::Utc;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Serialize)]
struct MetricRecord {
    timestamp: chrono::DateTime<Utc>,
    equipment_id: String,
    metric_id: String,
    value: f64,
    unit: String,
    line_id: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let brokers = std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "kafka:9092".into());
    let interval_ms: u64 = std::env::var("PRODUCE_INTERVAL_MS")
        .unwrap_or_else(|_| "1000".into())
        .parse()
        .unwrap_or(1000);

    tracing::info!(brokers = %brokers, interval_ms = interval_ms, "Starting metric producer");

    // Wait for Kafka to be ready
    tokio::time::sleep(Duration::from_secs(10)).await;

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Failed to create producer");

    // Also produce leader heartbeats
    let leader_producer = producer.clone();
    tokio::spawn(async move {
        loop {
            let _ = leader_producer
                .send(
                    FutureRecord::to("__exporter_leader")
                        .payload("heartbeat")
                        .key("leader"),
                    Duration::from_secs(1),
                )
                .await;
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    let topics = ["metrics.alarm", "metrics.key", "metrics.raw"];
    let equipment = [
        "CMP-A-001", "CMP-A-002", "Etch-B-001", "Etch-B-002", "Litho-C-001",
    ];
    let metrics = [
        ("temperature", "C"),
        ("pressure", "PSI"),
        ("vibration", "mm/s"),
        ("flow_rate", "L/min"),
    ];

    let mut counter: u64 = 0;

    loop {
        let equip = equipment[counter as usize % equipment.len()];
        let topic = topics[counter as usize % topics.len()];
        let (metric_id, unit) = &metrics[counter as usize % metrics.len()];

        let record = MetricRecord {
            timestamp: Utc::now(),
            equipment_id: equip.to_string(),
            metric_id: metric_id.to_string(),
            value: 20.0 + (counter as f64 * 0.1).sin() * 5.0,
            unit: unit.to_string(),
            line_id: format!("LINE-{}", &equip[..1]),
        };

        let payload = serde_json::to_string(&record).unwrap();

        match producer
            .send(
                FutureRecord::to(topic)
                    .payload(&payload)
                    .key(equip),
                Duration::from_secs(1),
            )
            .await
        {
            Ok(delivery) => {
                if counter % 50 == 0 {
                    tracing::info!(
                        topic = topic,
                        partition = delivery.partition,
                        offset = delivery.offset,
                        equip = equip,
                        "Produced metric"
                    );
                }
            }
            Err((e, _)) => {
                tracing::warn!(error = %e, "Failed to produce metric");
            }
        }

        counter += 1;
        tokio::time::sleep(Duration::from_millis(interval_ms)).await;
    }
}
