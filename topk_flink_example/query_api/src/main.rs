//! Query API: axum HTTP endpoints that read CMS + heavy-hitter candidate heaps
//! from Redis and estimate Top-K.
//!
//! Endpoints:
//!   GET /topk/all_time?k=100
//!   GET /topk/last_1m?k=100
//!   GET /topk/last_5m?k=100
//!   GET /topk/last_30m?k=100
//!   GET /topk/last_hour?k=100
//!   GET /topk/range?from=<epoch_seconds>&to=<epoch_seconds>&k=100
//!
//! `last_*` endpoints merge the last N minute-granularity sketches; `range`
//! merges every minute sketch in [from, to). CMS is additive so cell-wise
//! summing yields a valid merged sketch; candidate sets are unioned and then
//! re-estimated against the merged sketch to produce the final top k.

use anyhow::{anyhow, Result};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use redis::AsyncCommands;
use serde::Deserialize;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use topk_common::{epoch_minute, keys, CountMinSketch, HeavyHitter};
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    redis: redis::Client,
}

#[derive(Debug, Deserialize)]
struct TopkParams {
    #[serde(default = "default_k")]
    k: usize,
}
fn default_k() -> usize {
    100
}

#[derive(Debug, Deserialize)]
struct RangeParams {
    from: i64,
    to: i64,
    #[serde(default = "default_k")]
    k: usize,
}

struct ApiError(anyhow::Error);
impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        warn!(error = %self.0, "request failed");
        (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string()).into_response()
    }
}
impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(e: E) -> Self {
        Self(e.into())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let bind = std::env::var("BIND").unwrap_or_else(|_| "0.0.0.0:8080".into());

    let state = AppState {
        redis: redis::Client::open(redis_url.clone())?,
    };
    let state = Arc::new(state);

    let app = Router::new()
        .route("/", get(dashboard))
        .route("/health", get(|| async { "ok" }))
        .route("/topk/all_time", get(all_time))
        .route("/topk/last_1m", get(last_1m))
        .route("/topk/last_5m", get(last_5m))
        .route("/topk/last_30m", get(last_30m))
        .route("/topk/last_hour", get(last_hour))
        .route("/topk/range", get(range))
        .with_state(state);

    let addr: SocketAddr = bind.parse()?;
    info!(%addr, %redis_url, "query_api listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn dashboard() -> Html<&'static str> {
    Html(include_str!("../assets/index.html"))
}

async fn all_time(
    State(state): State<Arc<AppState>>,
    Query(p): Query<TopkParams>,
) -> Result<Json<Vec<HeavyHitter>>, ApiError> {
    let mut conn = state.redis.get_multiplexed_async_connection().await?;
    let cms_bytes: Option<Vec<u8>> = conn.get(keys::ALL_TIME_CMS).await?;
    let heap_json: Option<String> = conn.get(keys::ALL_TIME_HEAP).await?;

    let cms = match cms_bytes {
        Some(b) => CountMinSketch::from_bytes(&b).map_err(|e| anyhow!(e))?,
        None => return Ok(Json(vec![])),
    };
    let candidates: Vec<HeavyHitter> = heap_json
        .map(|j| serde_json::from_str(&j).unwrap_or_default())
        .unwrap_or_default();
    Ok(Json(rank(&cms, candidates.into_iter().map(|h| h.item), p.k)))
}

async fn last_1m(state: State<Arc<AppState>>, p: Query<TopkParams>) -> Result<Json<Vec<HeavyHitter>>, ApiError> {
    recent_minutes(state, p, 1).await
}
async fn last_5m(state: State<Arc<AppState>>, p: Query<TopkParams>) -> Result<Json<Vec<HeavyHitter>>, ApiError> {
    recent_minutes(state, p, 5).await
}
async fn last_30m(state: State<Arc<AppState>>, p: Query<TopkParams>) -> Result<Json<Vec<HeavyHitter>>, ApiError> {
    recent_minutes(state, p, 30).await
}
async fn last_hour(state: State<Arc<AppState>>, p: Query<TopkParams>) -> Result<Json<Vec<HeavyHitter>>, ApiError> {
    recent_minutes(state, p, 60).await
}

async fn recent_minutes(
    State(state): State<Arc<AppState>>,
    Query(p): Query<TopkParams>,
    n: i64,
) -> Result<Json<Vec<HeavyHitter>>, ApiError> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    let to_m = epoch_minute(now) + 1; // exclusive; include the in-flight minute
    let from_m = to_m - n;
    let merged = load_range(&state.redis, from_m, to_m).await?;
    Ok(Json(merged.top_k(p.k)))
}

async fn range(
    State(state): State<Arc<AppState>>,
    Query(p): Query<RangeParams>,
) -> Result<Json<Vec<HeavyHitter>>, ApiError> {
    if p.to <= p.from {
        return Err(ApiError(anyhow!("`to` must be greater than `from`")));
    }
    let from_m = epoch_minute(p.from);
    let to_m = epoch_minute(p.to - 1) + 1;
    let merged = load_range(&state.redis, from_m, to_m).await?;
    Ok(Json(merged.top_k(p.k)))
}

struct MergedWindow {
    cms: CountMinSketch,
    candidates: HashSet<String>,
}

impl MergedWindow {
    fn top_k(self, k: usize) -> Vec<HeavyHitter> {
        rank(&self.cms, self.candidates.into_iter(), k)
    }
}

async fn load_range(client: &redis::Client, from_m: i64, to_m: i64) -> Result<MergedWindow> {
    let mut conn = client.get_multiplexed_async_connection().await?;
    let span = (to_m - from_m).max(0) as usize;
    let mut cms_keys = Vec::with_capacity(span);
    let mut heap_keys = Vec::with_capacity(span);
    for m in from_m..to_m {
        cms_keys.push(keys::minute_cms(m));
        heap_keys.push(keys::minute_heap(m));
    }

    let cms_blobs: Vec<Option<Vec<u8>>> = if cms_keys.is_empty() {
        vec![]
    } else {
        conn.mget(&cms_keys).await?
    };
    let heap_blobs: Vec<Option<String>> = if heap_keys.is_empty() {
        vec![]
    } else {
        conn.mget(&heap_keys).await?
    };

    let mut merged = CountMinSketch::new();
    for blob in cms_blobs.into_iter().flatten() {
        match CountMinSketch::from_bytes(&blob) {
            Ok(c) => merged.merge(&c),
            Err(e) => warn!(%e, "skipping malformed sketch"),
        }
    }
    let mut candidates: HashSet<String> = HashSet::new();
    for blob in heap_blobs.into_iter().flatten() {
        if let Ok(items) = serde_json::from_str::<Vec<HeavyHitter>>(&blob) {
            candidates.extend(items.into_iter().map(|h| h.item));
        }
    }
    Ok(MergedWindow { cms: merged, candidates })
}

fn rank(cms: &CountMinSketch, candidates: impl IntoIterator<Item = String>, k: usize) -> Vec<HeavyHitter> {
    let mut scored: Vec<HeavyHitter> = candidates
        .into_iter()
        .map(|item| {
            let est = cms.estimate(&item);
            HeavyHitter { item, est }
        })
        .collect();
    scored.sort_by(|a, b| b.est.cmp(&a.est));
    scored.truncate(k);
    scored
}
