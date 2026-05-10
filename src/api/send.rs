//! `POST /v1/send` handler.
//!
//! Thin handler: delegates all business logic to domain modules.
//! Authentication, validation, mail construction, and SMTP submission
//! are each handled by their own module.

use std::sync::Arc;

use axum::{extract::State, http::{HeaderMap, StatusCode}, Json};
use serde_json::{json, Value};

use crate::{
    auth::AuthContext,
    error::AppError,
    mail, smtp, validation,
    AppState,
};

// Request DTO is defined in validation.rs; re-export here for handler use.
pub use crate::validation::MailRequest;

/// Mail submission handler.
///
/// Processing pipeline (RFC 040, 050, 060–062, 070–072):
/// 1. Rate limit: global and IP limits run as Tower middleware before this handler.
/// 2. Auth: `AuthContext` extractor enforces authentication.
/// 3. Rate limit: per-key limit checked here, after auth.
/// 4. Validate and sanitize request fields.
/// 5. Build lettre Message.
/// 6. Submit to SMTP.
/// 7. Return 202 Accepted.
pub async fn send_mail(
    State(state): State<Arc<AppState>>,
    req_headers: HeaderMap,
    auth: AuthContext,
    Json(payload): Json<MailRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    // 3a. Global rate limit — all requests regardless of identity (RFC 072).
    state.rate_limiter.check_global().map_err(|e| {
        state.metrics.inc_rate_limited("global");
        state.metrics.inc_request("4xx");
        tracing::warn!(event = "rate_limited", tier = "global", retry_after = e.retry_after_secs);
        AppError::RateLimited { retry_after_secs: Some(e.retry_after_secs) }
    })?;

    // 3b. Per-IP rate limit (RFC 072).
    state.rate_limiter.check_ip(auth.client_ip).map_err(|e| {
        state.metrics.inc_rate_limited("ip");
        state.metrics.inc_request("4xx");
        tracing::warn!(event = "rate_limited", tier = "ip", client_ip = %auth.client_ip, retry_after = e.retry_after_secs);
        AppError::RateLimited { retry_after_secs: Some(e.retry_after_secs) }
    })?;

    // 3c. Per-key rate limit — after auth so we know key_id and per-key config (RFC 072).
    state
        .rate_limiter
        .check_key(&auth.key_id, auth.key_rate_limit_per_min, auth.key_burst)
        .map_err(|e| {
            state.metrics.inc_rate_limited("key");
            state.metrics.inc_request("4xx");
            tracing::warn!(
                event = "rate_limited",
                tier = "key",
                key_id = %auth.key_id,
                retry_after = e.retry_after_secs,
            );
            AppError::RateLimited {
                retry_after_secs: Some(e.retry_after_secs),
            }
        })?;

    // 4. Validate and sanitize.
    let cfg = state.config();
    let validated = validation::validate_mail_request(payload, &cfg, &auth)
        .map_err(|e| {
            state.metrics.inc_validation_failure("request");
            state.metrics.inc_request("4xx");
            tracing::warn!(
                event = "validation_failure",
                key_id = %auth.key_id,
                client_ip = %auth.client_ip,
                error = %e,
            );
            e
        })?;

    // 5. Build mail message.
    let message = mail::build_message(&validated, &cfg)?;

    // 6. Submit to SMTP.
    // For logging: use domain of the first recipient.
    let recipient_domain = validated.to.first()
        .and_then(|a| a.split('@').nth(1))
        .unwrap_or("unknown");

    // Dispatch to pipe or direct SMTP based on mode (RFC 304).
    // Time SMTP for Prometheus histogram (RFC 401).
    let smtp_start = std::time::Instant::now();
    let smtp_result = if cfg.smtp.mode == "pipe" {
        smtp::submit_pipe(
            message,
            &cfg.smtp.pipe_command,
            cfg.smtp.submission_timeout_seconds,
        ).await
    } else {
        smtp::submit(&state.smtp, message, cfg.smtp.submission_timeout_seconds).await
    };
    state.metrics.observe_smtp_duration(smtp_start.elapsed().as_secs_f64());

    smtp_result.map_err(|e| {
            state.metrics.inc_smtp_error();
            state.metrics.inc_request("5xx");
            tracing::error!(
                event = "smtp_failure",
                key_id = %auth.key_id,
                client_ip = %auth.client_ip,
                recipient_domain = recipient_domain,
                error = %e,
            );
            e
        })?;

    state.metrics.inc_smtp_ok();
    state.metrics.inc_request("2xx");
    tracing::info!(
        event = "smtp_submitted",
        key_id = %auth.key_id,
        recipient_domain = recipient_domain,
    );

    // 7. Return 202 Accepted.
    // Read the request_id set by request_id_layer middleware (RFC 035).
    // The middleware stores it in x-internal-request-id so the response body
    // and X-Request-Id response header carry the same value.
    let request_id = req_headers
        .get("x-internal-request-id")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({
            "request_id": request_id,
            "status": "accepted",
        })),
    ))
}
