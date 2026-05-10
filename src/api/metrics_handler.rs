//! `GET /metrics` handler — Prometheus text format export.
//!
//! Implements RFC 401. Returns all registered metrics in the Prometheus
//! text exposition format (version 0.0.4).
//!
//! Access restriction: document at the proxy layer, same as `/readyz`.

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse};

use crate::{metrics, AppState};

/// Prometheus metrics endpoint.
///
/// Returns 200 with `Content-Type: text/plain; version=0.0.4` on success,
/// or 500 with a plain error message if serialization fails.
pub async fn metrics_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match metrics::encode(&state.metrics.registry) {
        Ok(body) => (
            StatusCode::OK,
            [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
            body,
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}
