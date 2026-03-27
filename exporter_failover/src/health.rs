/// Health Monitor: periodically pings the cloud endpoint.
///
/// State Machine transitions:
/// - CONNECTED -> DISCONNECTED: 3 consecutive health check failures
/// - DISCONNECTED -> BACKFILLING: health check succeeds again
/// - BACKFILLING -> CONNECTED: backfill engine completes
use std::sync::Arc;
use std::time::Duration;

use tracing::{error, info, warn};

use crate::state::{ConnectionState, SharedState};

pub async fn health_monitor(state: Arc<SharedState>) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    let mut consecutive_failures: u32 = 0;
    let health_url = format!("{}/health", state.config.aws_endpoint);
    let interval = state.config.heartbeat_interval_secs;

    info!(
        url = %health_url,
        interval_secs = interval,
        "Health monitor started"
    );

    loop {
        tokio::time::sleep(Duration::from_secs(interval)).await;

        match client.head(&health_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let prev_state = state.get_connection_state();
                consecutive_failures = 0;

                match prev_state {
                    ConnectionState::Disconnected => {
                        info!("WAN recovered! Transitioning to BACKFILL");
                        let _ = state
                            .connection_tx
                            .send(ConnectionState::Backfilling);
                        state.backfill_notify.notify_one();
                    }
                    ConnectionState::Backfilling => {
                        // Backfill in progress, keep state
                    }
                    ConnectionState::Connected => {
                        // Normal, nothing to do
                    }
                }
            }
            Ok(resp) => {
                warn!(status = %resp.status(), "Health check returned non-success");
                consecutive_failures += 1;
            }
            Err(e) => {
                warn!(error = %e, "Health check failed");
                consecutive_failures += 1;
            }
        }

        // 3 consecutive failures -> mark as disconnected
        if consecutive_failures >= 3 {
            let current = state.get_connection_state();
            if current == ConnectionState::Connected {
                error!(
                    failures = consecutive_failures,
                    "WAN disconnected! Consecutive health check failures exceeded threshold"
                );
                let _ = state
                    .connection_tx
                    .send(ConnectionState::Disconnected);
            }
        }
    }
}
