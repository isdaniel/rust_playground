/// HTTP API server: exposes health, status, and metrics endpoints.
///
/// Endpoints:
/// - GET /health       -> 200 OK (for peer health checks and load balancer)
/// - GET /status       -> JSON with current role, connection state, instance info
/// - GET /metrics      -> Prometheus-style metrics (simplified)
use std::sync::Arc;

use axum::extract::State;
use axum::response::Json;
use axum::routing::get;
use axum::Router;
use serde::Serialize;
use tracing::info;

use crate::state::SharedState;

#[derive(Serialize)]
struct StatusResponse {
    instance_id: String,
    fab_id: String,
    role: String,
    connection_state: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
}

async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

async fn status_handler(State(state): State<Arc<SharedState>>) -> Json<StatusResponse> {
    let role = state.get_role().await;
    let conn = state.get_connection_state();

    Json(StatusResponse {
        instance_id: state.config.instance_id.clone(),
        fab_id: state.config.fab_id.clone(),
        role: role.to_string(),
        connection_state: conn.to_string(),
    })
}

async fn metrics_handler(State(state): State<Arc<SharedState>>) -> String {
    let role = state.get_role().await;
    let conn = state.get_connection_state();

    let role_val = match role {
        crate::state::HaRole::Active => 1,
        crate::state::HaRole::Standby => 0,
    };

    let conn_val = match conn {
        crate::state::ConnectionState::Connected => 0,
        crate::state::ConnectionState::Disconnected => 1,
        crate::state::ConnectionState::Backfilling => 2,
    };

    format!(
        "# HELP exporter_ha_role Current HA role (1=active, 0=standby)\n\
         # TYPE exporter_ha_role gauge\n\
         exporter_ha_role{{instance=\"{}\"}} {}\n\
         # HELP exporter_connection_state Connection state (0=connected, 1=disconnected, 2=backfill)\n\
         # TYPE exporter_connection_state gauge\n\
         exporter_connection_state{{instance=\"{}\"}} {}\n",
        state.config.instance_id,
        role_val,
        state.config.instance_id,
        conn_val,
    )
}

pub async fn run_http_server(state: Arc<SharedState>) {
    let port = state.config.http_port;

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/status", get(status_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    info!(addr = %addr, "HTTP server starting");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
