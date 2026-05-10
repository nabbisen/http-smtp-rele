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

use crate::{AppState, RequestId};

pub mod health;
pub mod keys;
pub mod metrics_handler;
pub mod send;
pub mod send_bulk;
pub mod submissions;

// ---------------------------------------------------------------------------
// X-Request-Id middleware (RFC 035)
// ---------------------------------------------------------------------------

/// Axum middleware that generates a `RequestId` (req_ + ULID) per request,
/// stores it as a request Extension for handlers, and injects `X-Request-Id`
/// into the response header (RFC 035, RFC 036).
async fn request_id_layer(mut req: axum::http::Request<axum::body::Body>, next: Next) -> Response {
    let request_id = RequestId::new();
    let header_val = request_id.to_string();
    req.extensions_mut().insert(request_id);
    let mut resp = next.run(req).await;
    if let Ok(val) = HeaderValue::from_str(&header_val) {
        resp.headers_mut().insert("x-request-id", val);
    }
    resp
}

// ---------------------------------------------------------------------------
// Router construction
// ---------------------------------------------------------------------------

/// Axum extractor that reads `RequestId` set by `request_id_layer`, or generates a
/// new one as a fallback (ensures tests that bypass the middleware still work).
pub struct ExtractRequestId(pub crate::RequestId);

impl<S: Send + Sync> axum::extract::FromRequestParts<S> for ExtractRequestId {
    type Rejection = std::convert::Infallible;

    fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let id = parts
            .extensions
            .get::<crate::RequestId>()
            .cloned()
            .unwrap_or_else(crate::RequestId::new);
        async move { Ok(ExtractRequestId(id)) }
    }
}

/// Build the Axum application router with all middleware layers applied.
pub fn build_router(state: Arc<AppState>) -> Router {
    let cfg = state.config();
    let max_body     = cfg.server.max_request_body_bytes;
    let timeout_secs = cfg.server.request_timeout_seconds;
    let concurrency  = cfg.server.concurrency_limit;

    let mut router = Router::new()
        .route("/healthz", get(health::healthz))
        .route("/readyz",  get(health::readyz))
        .route("/metrics", get(metrics_handler::metrics_handler))
        .route("/v1/submissions/{request_id}", get(submissions::get_submission_status))
        .route("/v1/keys/self", get(keys::get_key_self))
        .route("/v1/send", axum::routing::post(send::send_mail))
        .route("/v1/send-bulk", axum::routing::post(send_bulk::send_bulk))
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
