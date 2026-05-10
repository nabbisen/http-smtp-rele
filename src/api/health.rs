//! Health and readiness probe handlers.
//!
//! - `GET /healthz`: liveness probe, SMTP-independent, no auth required.
//! - `GET /readyz`: readiness probe, checks SMTP TCP reachability.

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

use crate::AppState;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Liveness probe.
///
/// Returns 200 as long as the process is running and serving requests.
/// No authentication required. SMTP-independent.
pub async fn healthz() -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "version": VERSION,
        })),
    )
}

/// Readiness probe.
///
/// Returns 200 when the SMTP server is reachable via TCP.
/// Returns 503 when SMTP is unreachable.
/// No authentication required.
pub async fn readyz(State(state): State<Arc<AppState>>) -> (StatusCode, Json<Value>) {
    let smtp_ok = check_smtp_reachable(&state).await;

    if smtp_ok {
        (
            StatusCode::OK,
            Json(json!({
                "status": "ready",
                "smtp": "ok",
            })),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "not_ready",
                "smtp": "unavailable",
            })),
        )
    }
}

/// Lightweight TCP connectivity check against the configured SMTP server.
///
/// Uses a hardcoded 2-second timeout, independent of `connect_timeout_seconds`.
/// Does not send any SMTP commands — only checks TCP reachability.
async fn check_smtp_reachable(state: &AppState) -> bool {
    let cfg = state.config();
    let addr = format!("{}:{}", cfg.smtp.host, cfg.smtp.port);
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        tokio::net::TcpStream::connect(&addr),
    )
    .await
    .map(|result| result.is_ok())
    .unwrap_or(false)
}
