/// Mock AWS endpoint for testing.
/// A simple Axum HTTP server that accepts ingest requests and responds 200 OK.
/// Can be toggled to simulate WAN outage by sending POST /admin/disconnect
/// and POST /admin/connect.
use axum::extract::State as AxumState;
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{get, head, post};
use axum::Router;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

struct MockState {
    connected: AtomicBool,
    metrics_received: AtomicU64,
    backfill_received: AtomicU64,
}

#[derive(Serialize)]
struct StatsResponse {
    connected: bool,
    metrics_batches: u64,
    backfill_batches: u64,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = Arc::new(MockState {
        connected: AtomicBool::new(true),
        metrics_received: AtomicU64::new(0),
        backfill_received: AtomicU64::new(0),
    });

    let app = Router::new()
        // Health endpoint (used by exporter's health monitor)
        .route("/health", head(health_handler))
        .route("/health", get(health_get_handler))
        // Ingest endpoints
        .route("/ingest/metrics", post(metrics_handler))
        .route("/ingest/backfill", post(backfill_handler))
        // Admin endpoints (to simulate WAN outage)
        .route("/admin/disconnect", post(disconnect_handler))
        .route("/admin/connect", post(connect_handler))
        .route("/admin/stats", get(stats_handler))
        .with_state(state);

    let addr = "0.0.0.0:8080";
    tracing::info!("Mock AWS endpoint listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_handler(
    AxumState(state): AxumState<Arc<MockState>>,
) -> StatusCode {
    if state.connected.load(Ordering::Relaxed) {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

async fn health_get_handler(
    AxumState(state): AxumState<Arc<MockState>>,
) -> StatusCode {
    if state.connected.load(Ordering::Relaxed) {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

async fn metrics_handler(
    AxumState(state): AxumState<Arc<MockState>>,
    body: axum::body::Bytes,
) -> StatusCode {
    if !state.connected.load(Ordering::Relaxed) {
        return StatusCode::SERVICE_UNAVAILABLE;
    }
    state.metrics_received.fetch_add(1, Ordering::Relaxed);
    tracing::info!(bytes = body.len(), "Received metrics batch");
    StatusCode::OK
}

async fn backfill_handler(
    AxumState(state): AxumState<Arc<MockState>>,
    body: axum::body::Bytes,
) -> StatusCode {
    if !state.connected.load(Ordering::Relaxed) {
        return StatusCode::SERVICE_UNAVAILABLE;
    }
    state.backfill_received.fetch_add(1, Ordering::Relaxed);
    tracing::info!(bytes = body.len(), "Received backfill batch");
    StatusCode::OK
}

async fn disconnect_handler(
    AxumState(state): AxumState<Arc<MockState>>,
) -> &'static str {
    state.connected.store(false, Ordering::Relaxed);
    tracing::warn!("SIMULATED WAN DISCONNECT");
    "disconnected"
}

async fn connect_handler(
    AxumState(state): AxumState<Arc<MockState>>,
) -> &'static str {
    state.connected.store(true, Ordering::Relaxed);
    tracing::info!("SIMULATED WAN RECONNECT");
    "connected"
}

async fn stats_handler(
    AxumState(state): AxumState<Arc<MockState>>,
) -> Json<StatsResponse> {
    Json(StatsResponse {
        connected: state.connected.load(Ordering::Relaxed),
        metrics_batches: state.metrics_received.load(Ordering::Relaxed),
        backfill_batches: state.backfill_received.load(Ordering::Relaxed),
    })
}
