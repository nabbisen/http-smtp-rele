//! HTTP API router and middleware wiring.

use std::sync::Arc;
use std::time::Duration;

use axum::{extract::DefaultBodyLimit, routing::get, Router};
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};

use crate::AppState;

pub mod health;
pub mod send;

/// Build the Axum application router with all middleware layers applied.
pub fn build_router(state: Arc<AppState>) -> Router {
    let max_body = state.config.server.max_request_body_bytes;
    let timeout_secs = state.config.server.request_timeout_seconds;

    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
        .route("/v1/send", axum::routing::post(send::send_mail))
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(axum::http::StatusCode::REQUEST_TIMEOUT, Duration::from_secs(timeout_secs)))
        .layer(DefaultBodyLimit::max(max_body))
        .with_state(state)
}
