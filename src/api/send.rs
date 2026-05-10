//! `POST /v1/send` handler — with submission status tracking (RFC 086/087).

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

use crate::{
    api::ExtractRequestId,
    auth::AuthContext,
    error::AppError,
    mail, smtp, validation,
    status::{
        recipient_domains_from, ErrorCode, StatusUpdate, SubmissionStatus,
        SubmissionStatusRecord,
    },
    AppState,
};

pub use crate::validation::MailRequest;

pub async fn send_mail(
    State(state): State<Arc<AppState>>,
    ExtractRequestId(request_id): ExtractRequestId,
    auth: AuthContext,
    Json(payload): Json<MailRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    // ── Rate limits ────────────────────────────────────────────────────────
    state.rate_limiter.check_global().map_err(|e| {
        state.metrics.inc_rate_limited("global"); state.metrics.inc_request("4xx");
        tracing::warn!(event="rate_limited", tier="global", retry_after=e.retry_after_secs);
        AppError::RateLimited { retry_after_secs: Some(e.retry_after_secs) }
    })?;

    state.rate_limiter.check_ip(auth.client_ip).map_err(|e| {
        state.metrics.inc_rate_limited("ip"); state.metrics.inc_request("4xx");
        tracing::warn!(event="rate_limited", tier="ip",
            client_ip=%auth.client_ip, retry_after=e.retry_after_secs);
        AppError::RateLimited { retry_after_secs: Some(e.retry_after_secs) }
    })?;

    state.rate_limiter
        .check_key(&auth.key_id, auth.key_rate_limit_per_min, auth.key_burst)
        .map_err(|e| {
            state.metrics.inc_rate_limited("key"); state.metrics.inc_request("4xx");
            tracing::warn!(event="rate_limited", tier="key",
                key_id=%auth.key_id, retry_after=e.retry_after_secs);
            AppError::RateLimited { retry_after_secs: Some(e.retry_after_secs) }
        })?;

    // ── Initial status record (received) — created after auth+rate ────────
    let cfg = state.config();
    state.status_store.put(SubmissionStatusRecord::new(
        request_id.clone(), auth.key_id.clone(), vec![], 0, cfg.status.ttl_seconds,
    ));

    // ── Validate ───────────────────────────────────────────────────────────
    let validated = validation::validate_mail_request(payload, &cfg, &auth)
        .map_err(|e| {
            state.metrics.inc_validation_failure("request");
            state.metrics.inc_request("4xx");
            tracing::warn!(event="validation_failure", key_id=%auth.key_id, error=%e);
            state.status_store.update_status(&request_id, &auth.key_id, StatusUpdate {
                status: SubmissionStatus::Rejected,
                code: Some(error_to_code(&e)),
                message: Some("Request rejected during validation.".into()),
            });
            e
        })?;

    // Update record with real recipient domains.
    let domains = recipient_domains_from(&validated.to, &validated.cc);
    let recipient_count = (validated.to.len() + validated.cc.len()) as u32;
    state.status_store.put(SubmissionStatusRecord {
        request_id: request_id.clone(),
        key_id: auth.key_id.clone(),
        status: SubmissionStatus::Received,
        code: None,
        message: Some("Submission validated.".into()),
        recipient_domains: domains,
        recipient_count,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now()
            + chrono::Duration::seconds(cfg.status.ttl_seconds as i64),
    });

    // ── Resolve effective mask_recipient for this key (RFC 603) ──────────
    let effective_mask = cfg.security.api_keys.iter()
        .find(|k| k.id == auth.key_id)
        .and_then(|k| k.mask_recipient)
        .unwrap_or(cfg.logging.mask_recipient);

    // ── Build message ──────────────────────────────────────────────────────
    let message = mail::build_message(&validated, &cfg)?;

    // ── Status: smtp_submission_started ────────────────────────────────────
    state.status_store.update_status(&request_id, &auth.key_id, StatusUpdate {
        status: SubmissionStatus::SmtpSubmissionStarted,
        code: None,
        message: Some("Submitting to SMTP server.".into()),
    });

    // ── SMTP submit ────────────────────────────────────────────────────────
    let recipient_domain = validated.to.first()
        .and_then(|a| a.split('@').nth(1)).unwrap_or("unknown");

    let smtp_start = std::time::Instant::now();
    let smtp_result = if cfg.smtp.mode == "pipe" {
        smtp::submit_pipe(message, &cfg.smtp.pipe_command,
            cfg.smtp.submission_timeout_seconds).await
    } else {
        smtp::submit(&state.smtp, message, cfg.smtp.submission_timeout_seconds).await
    };
    state.metrics.observe_smtp_duration(smtp_start.elapsed().as_secs_f64());

    smtp_result.map_err(|e| {
        state.metrics.inc_smtp_error(); state.metrics.inc_request("5xx");
        tracing::error!(event="smtp_failure", key_id=%auth.key_id, recipient_domain, error=%e);
        state.status_store.update_status(&request_id, &auth.key_id, StatusUpdate {
            status: SubmissionStatus::SmtpFailed,
            code: Some(ErrorCode::SmtpUnavailable),
            message: Some("The configured SMTP server was unavailable.".into()),
        });
        e
    })?;

    // ── Status: smtp_accepted ──────────────────────────────────────────────
    state.status_store.update_status(&request_id, &auth.key_id, StatusUpdate {
        status: SubmissionStatus::SmtpAccepted,
        code: None,
        message: Some("The message was accepted by the configured SMTP server.".into()),
    });

    state.metrics.inc_smtp_ok(); state.metrics.inc_request("2xx");
    let logged_domain = if effective_mask { "***" } else { recipient_domain };
    tracing::info!(event="smtp_submitted", key_id=%auth.key_id,
        recipient_domain=logged_domain, request_id=%request_id);

    Ok((StatusCode::ACCEPTED, Json(json!({
        "request_id": request_id.to_string(),
        "status": "accepted",
    }))))
}

fn error_to_code(e: &AppError) -> ErrorCode {
    match e {
        AppError::Validation(_)       => ErrorCode::ValidationFailed,
        AppError::BadRequest          => ErrorCode::BadRequest,
        AppError::PayloadTooLarge     => ErrorCode::PayloadTooLarge,
        AppError::UnsupportedMediaType=> ErrorCode::UnsupportedMediaType,
        AppError::RateLimited { .. }  => ErrorCode::RateLimited,
        AppError::Forbidden           => ErrorCode::Forbidden,
        AppError::SmtpUnavailable     => ErrorCode::SmtpUnavailable,
        AppError::Unauthorized        => ErrorCode::Forbidden,
        _                             => ErrorCode::InternalError,
    }
}
