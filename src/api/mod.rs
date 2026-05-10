//! HTTP API router and middleware wiring.

use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::DefaultBodyLimit,
    http::HeaderValue,
    middleware::{self, Next},
    response::Response,
    routing::get,
    Router,
};
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};

use crate::AppState;

pub mod health;
pub mod send;

// ---------------------------------------------------------------------------
// X-Request-Id middleware (RFC 035)
// ---------------------------------------------------------------------------

/// Axum middleware that injects an `X-Request-Id` header on every response.
///
/// The UUID is generated once per request and added to the response header.
/// Handlers access it via the `ReqId` extension if they need to echo it in
/// the response body.
async fn request_id_layer(mut req: axum::http::Request<axum::body::Body>, next: Next) -> Response {
    let request_id = uuid::Uuid::new_v4().to_string();
    // Store in a request header so handlers can read it via HeaderMap extractor.
    // Using an internal header name avoids conflict with client-supplied headers.
    if let Ok(val) = HeaderValue::from_str(&request_id) {
        req.headers_mut().insert("x-internal-request-id", val.clone());
    }
    let mut resp = next.run(req).await;
    // Inject into the response header so clients can correlate with server logs.
    if let Ok(val) = HeaderValue::from_str(&request_id) {
        resp.headers_mut().insert("x-request-id", val);
    }
    resp
}

// ---------------------------------------------------------------------------
// Router construction
// ---------------------------------------------------------------------------

/// Build the Axum application router with all middleware layers applied.
pub fn build_router(state: Arc<AppState>) -> Router {
    let max_body     = state.config.server.max_request_body_bytes;
    let timeout_secs = state.config.server.request_timeout_seconds;
    let concurrency  = state.config.server.concurrency_limit;

    let mut router = Router::new()
        .route("/healthz", get(health::healthz))
        .route("/readyz",  get(health::readyz))
        .route("/v1/send", axum::routing::post(send::send_mail))
        .layer(middleware::from_fn(request_id_layer))
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(timeout_secs),
        ))
        .layer(DefaultBodyLimit::max(max_body))
        .with_state(state);

    // RFC 205: optional concurrency cap (outermost layer — applied first)
    if concurrency > 0 {
        router = router.layer(tower::limit::ConcurrencyLimitLayer::new(concurrency));
    }

    router
}
