//! `POST /v1/send` handler.
//!
//! RFC 805: raw `Bytes` extraction so Content-Type / JSON parse / rate-limit
//! failures are recorded in the status store after auth succeeds.
//!
//! Processing order (architect spec):
//! 1. request_id (middleware)
//! 2. auth
//! 3. status = received
//! 4. Content-Type check
//! 5. body size check
//! 6. JSON parse
//! 7. rate limit
//! 8. validation / sanitization
//! 9. mail build
//! 10. smtp_submission_started
//! 11. smtp_accepted / smtp_failed / rejected

use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde_json::{json, Value};

use crate::{
    api::ExtractRequestId,
    auth::AuthContext,
    error::{AppError, ErrorCode, RequestError},
    mail, smtp, validation,
    status::{
        recipient_domains_from, StatusStore, StatusUpdate,
        SubmissionStatus, SubmissionStatusRecord,
    },
    AppState,
};

pub async fn send_mail(
    State(state):                State<Arc<AppState>>,
    ExtractRequestId(request_id): ExtractRequestId,
    auth:                        AuthContext,
    headers:                     HeaderMap,
    body:                        Bytes,
) -> Result<(StatusCode, Json<Value>), RequestError> {
    let cfg = state.config();
    let rid = request_id.clone();

    macro_rules! request_err {
        ($e:expr) => { Err(RequestError::new(rid.clone(), $e)) };
    }

    // ── Step 3: status = received ─────────────────────────────────────────
    let record = SubmissionStatusRecord::new_received(
        request_id.clone(),
        auth.key_id.clone(),
        cfg.status.ttl_seconds,
    );
    if let Err(e) = state.status_store.put_received(record) {
        tracing::warn!(error = %e, "status put_received failed (degraded)");
    }

    // Helper: reject and update status store
    let reject = |store: &Arc<dyn StatusStore>, status: AppError| {
        let code = store_code_from_app_error(&status);
        if let Err(e) = store.update_status(
            &request_id, &auth.key_id,
            StatusUpdate { status: SubmissionStatus::Rejected, code, message: None },
        ) {
            tracing::warn!(error = %e, "status update failed on reject");
        }
    };

    // ── Step 4: Content-Type ─────────────────────────────────────────────
    let ct = headers.get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !ct.starts_with("application/json") {
        reject(&state.status_store, AppError::UnsupportedMediaType);
        state.metrics.inc_request("4xx");
        return request_err!(AppError::UnsupportedMediaType);
    }

    // ── Step 5: body size ─────────────────────────────────────────────────
    if body.len() > cfg.server.max_request_body_bytes {
        reject(&state.status_store, AppError::PayloadTooLarge);
        state.metrics.inc_request("4xx");
        return request_err!(AppError::PayloadTooLarge);
    }

    // ── Step 6: JSON parse ────────────────────────────────────────────────
    let payload: validation::MailRequest = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(_) => {
            reject(&state.status_store, AppError::BadRequest);
            state.metrics.inc_validation_failure("json_parse");
            state.metrics.inc_request("4xx");
            return request_err!(AppError::BadRequest);
        }
    };

    // ── Step 7: rate limits ───────────────────────────────────────────────
    macro_rules! check_rate {
        ($check:expr, $tier:literal) => {
            if let Err(e) = $check {
                state.metrics.inc_rate_limited($tier);
                state.metrics.inc_request("4xx");
                tracing::warn!(event="rate_limited", tier=$tier,
                    request_id=%request_id, retry_after=e.retry_after_secs);
                let app_err = AppError::RateLimited { retry_after_secs: Some(e.retry_after_secs) };
                reject(&state.status_store, AppError::RateLimited { retry_after_secs: None });
                return request_err!(app_err);
            }
        };
    }
    check_rate!(state.rate_limiter.check_global(), "global");
    check_rate!(state.rate_limiter.check_ip(auth.client_ip), "ip");
    check_rate!(state.rate_limiter.check_key(&auth.key_id, auth.key_rate_limit_per_min, auth.key_burst), "key");

    // ── Step 8: validation ────────────────────────────────────────────────
    let validated = match validation::validate_mail_request(payload, &cfg, &auth) {
        Ok(v) => v,
        Err(e) => {
            state.metrics.inc_validation_failure("request");
            state.metrics.inc_request("4xx");
            reject(&state.status_store, AppError::Validation(String::new()));
            return request_err!(e);
        }
    };

    // Update recipient metadata after successful validation (RFC 813).
    let domains = recipient_domains_from(&validated.to, &validated.cc);
    let count   = (validated.to.len() + validated.cc.len()) as u32;
    if let Err(e) = state.status_store.set_recipient_metadata(
        &request_id, &auth.key_id, domains, count,
    ) {
        tracing::warn!(error = %e, "status set_recipient_metadata failed (degraded)");
    }

    // ── Step 9: build mail ────────────────────────────────────────────────
    let message = match mail::build_message(&validated, &cfg) {
        Ok(m) => m,
        Err(e) => {
            // RFC 809: build failure → terminal status
            if let Err(se) = state.status_store.update_status(
                &request_id, &auth.key_id,
                StatusUpdate {
                    status: SubmissionStatus::Rejected,
                    code: Some(ErrorCode::InternalError),
                    message: Some("Failed to build mail message.".into()),
                },
            ) {
                tracing::warn!(error = %se, "status update failed on build error");
            }
            state.metrics.inc_request("5xx");
            return request_err!(e);
        }
    };

    // ── Step 10: smtp_submission_started ──────────────────────────────────
    if let Err(e) = state.status_store.update_status(
        &request_id, &auth.key_id,
        StatusUpdate {
            status: SubmissionStatus::SmtpSubmissionStarted,
            code: None,
            message: None,
        },
    ) {
        tracing::warn!(error = %e, "status smtp_submission_started failed (degraded)");
    }

    // ── Step 11: SMTP submit ──────────────────────────────────────────────
    match smtp::submit(&state.smtp, message, cfg.smtp.submission_timeout_seconds).await {
        Ok(()) => {
            state.metrics.inc_smtp_ok();
            state.metrics.inc_request("2xx");
            if let Err(e) = state.status_store.update_status(
                &request_id, &auth.key_id,
                StatusUpdate {
                    status: SubmissionStatus::SmtpAccepted,
                    code: None,
                    message: Some("The message was accepted by the configured SMTP server.".into()),
                },
            ) {
                tracing::warn!(error = %e, "status smtp_accepted failed (degraded)");
            }
            Ok((
                StatusCode::ACCEPTED,
                Json(json!({ "status": "accepted", "request_id": request_id.as_str() })),
            ))
        }
        Err(app_err) => {
            let smtp_code = match &app_err {
                AppError::SmtpRejected   => ErrorCode::SmtpRejected,
                _                        => ErrorCode::SmtpUnavailable,
            };
            let smtp_status = match &app_err {
                AppError::SmtpRejected => SubmissionStatus::SmtpFailed,
                _                      => SubmissionStatus::SmtpFailed,
            };
            state.metrics.inc_smtp_error();
            state.metrics.inc_request("5xx");
            if let Err(e) = state.status_store.update_status(
                &request_id, &auth.key_id,
                StatusUpdate { status: smtp_status, code: Some(smtp_code), message: None },
            ) {
                tracing::warn!(error = %e, "status smtp_failed update failed (degraded)");
            }
            request_err!(app_err)
        }
    }
}

/// Map AppError to the ErrorCode stored in the status record.
/// RFC 838: delegates to AppError::error_code().
fn store_code_from_app_error(e: &AppError) -> Option<ErrorCode> {
    match e.error_code() {
        ErrorCode::InternalError => None, // don't expose in status
        code => Some(code),
    }
}
