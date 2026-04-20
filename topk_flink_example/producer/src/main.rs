//! Synthetic event producer.
//!
//! Four scenarios drive Kafka ingestion so the Top-K system can be exercised
//! under qualitatively different workloads:
//!
//!   zipf     — steady Zipf(s=1.2) over the full item universe; baseline load
//!   burst    — low baseline + periodic short spikes on a tiny hot set
//!   shifting — hot set rotates every few minutes; last_hour should track it
//!   viral    — one item ramps from zero to dominant over N minutes
//!
//! Each scenario yields an item_id and the producer paces overall throughput
//! against `--rate` events/sec via a fixed tick.

use anyhow::Result;
use clap::{Parser, ValueEnum};
use rand::distributions::Distribution;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use rand_distr::Zipf;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord, Producer};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use topk_common::Event;
use tracing::info;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Scenario {
    /// Steady Zipf background traffic across the whole item universe.
    Zipf,
    /// Low baseline + periodic bursts on a small hot set (exercises last_hour vs all_time divergence).
    Burst,
    /// Hot set rotates every `--shift-period-sec`; verifies last_hour tracks changes.
    Shifting,
    /// A small cohort of items ramps from zero to dominant (viral content).
    Viral,
}

#[derive(Parser, Debug)]
#[command(about = "Top-K demo event producer")]
struct Args {
    #[arg(long, env = "KAFKA_BROKERS", default_value = "localhost:9092")]
    brokers: String,

    #[arg(long, default_value = "events")]
    topic: String,

    /// Events per second (approximate).
    #[arg(long, default_value_t = 5_000)]
    rate: u64,

    /// Distinct item universe size.
    #[arg(long, default_value_t = 10_000_000u64)]
    items: u64,

    /// Distinct user universe size.
    #[arg(long, default_value_t = 100_000_000u64)]
    users: u64,

    /// Zipf exponent (>1.0 heavy head). Used by `zipf` scenario and as the
    /// background distribution in other scenarios.
    #[arg(long, default_value_t = 1.2_f64)]
    zipf_s: f64,

    /// How many events to produce before exiting; 0 = forever.
    #[arg(long, default_value_t = 0u64)]
    max_events: u64,

    /// Which workload pattern to produce.
    #[arg(long, value_enum, default_value_t = Scenario::Zipf)]
    scenario: Scenario,

    /// Tag included in logs so compose logs show which producer is which.
    #[arg(long, default_value = "producer")]
    label: String,

    // ===== Scenario knobs =====
    /// `burst`: seconds between bursts.
    #[arg(long, default_value_t = 30u64)]
    burst_period_sec: u64,
    /// `burst`: seconds each burst lasts.
    #[arg(long, default_value_t = 5u64)]
    burst_duration_sec: u64,
    /// `burst` / `shifting`: size of the hot set.
    #[arg(long, default_value_t = 20u64)]
    hot_set_size: u64,

    /// `shifting`: seconds before the hot set rotates.
    #[arg(long, default_value_t = 120u64)]
    shift_period_sec: u64,

    /// `viral`: number of items being inflated.
    #[arg(long, default_value_t = 5u64)]
    viral_items: u64,
    /// `viral`: seconds over which the viral items ramp from 0 -> dominant.
    #[arg(long, default_value_t = 300u64)]
    viral_ramp_sec: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();
    info!(?args, "starting producer");

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &args.brokers)
        .set("message.timeout.ms", "5000")
        .set("linger.ms", "20")
        .set("compression.type", "lz4")
        .create()?;

    let zipf = Zipf::new(args.items, args.zipf_s).expect("valid zipf params");
    let mut rng = SmallRng::from_entropy();
    let started = std::time::Instant::now();

    let tick = Duration::from_micros(1_000_000 / args.rate.max(1));
    let mut next = tokio::time::Instant::now();
    let mut sent = 0u64;

    loop {
        next += tick;
        tokio::time::sleep_until(next).await;

        let elapsed = started.elapsed().as_secs();
        let item_rank = pick_item(&args, elapsed, &mut rng, &zipf);
        let user_id = rng.gen_range(0..args.users);
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let event = Event {
            user_id: format!("u{}", user_id),
            item_id: format!("item_{}", item_rank),
            ts,
        };
        let payload = serde_json::to_string(&event)?;
        let record = FutureRecord::to(&args.topic).key(&event.item_id).payload(&payload);
        let _ = producer.send_result(record);

        sent += 1;
        if sent % 10_000 == 0 {
            info!(label = %args.label, scenario = ?args.scenario, sent, "progress");
        }
        if args.max_events > 0 && sent >= args.max_events {
            break;
        }
    }

    producer.flush(Duration::from_secs(10))?;
    info!(label = %args.label, sent, "producer done");
    Ok(())
}

/// Returns an item rank in `[0, items)`. Item 0 is reserved; we offset rank by 1
/// so logs stay 1-indexed ("item_1" is the head of the Zipf distribution).
fn pick_item(args: &Args, elapsed_sec: u64, rng: &mut SmallRng, zipf: &Zipf<f64>) -> u64 {
    match args.scenario {
        Scenario::Zipf => zipf.sample(rng) as u64,

        Scenario::Burst => {
            let phase = elapsed_sec % args.burst_period_sec;
            let in_burst = phase < args.burst_duration_sec;
            if in_burst && rng.gen_bool(0.9) {
                // 90% of burst events land on the hot set; hot items are 1..=hot_set_size.
                rng.gen_range(1..=args.hot_set_size)
            } else {
                zipf.sample(rng) as u64
            }
        }

        Scenario::Shifting => {
            // Deterministic per-period offset so all `shifting` replicas agree.
            let period_idx = elapsed_sec / args.shift_period_sec;
            // Hot set = [base, base + hot_set_size). Keep base away from rank 1..1000 so
            // we don't collide with zipf head and can visibly see the rotation.
            let base = 1_000 + (period_idx * args.hot_set_size) % args.items.max(1);
            if rng.gen_bool(0.7) {
                base + rng.gen_range(0..args.hot_set_size)
            } else {
                zipf.sample(rng) as u64
            }
        }

        Scenario::Viral => {
            // Linear ramp: fraction of events on the viral cohort grows 0 -> 0.9 over viral_ramp_sec,
            // then stays at 0.9. Viral items occupy ranks 50..50+viral_items to avoid the Zipf head
            // so their climb is observable.
            let frac = (elapsed_sec as f64 / args.viral_ramp_sec as f64).min(1.0) * 0.9;
            if rng.gen_bool(frac) {
                50 + rng.gen_range(0..args.viral_items)
            } else {
                zipf.sample(rng) as u64
            }
        }
    }
}
