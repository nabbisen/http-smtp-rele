//! `POST /v1/send-bulk` handler.
//!
//! Accepts an array of independent mail messages and processes each through
//! the same validation → SMTP → status pipeline as `POST /v1/send`.
//!
//! # Key design points (RFC 701/702)
//!
//! - Each message is processed independently; one failure does not abort others.
//! - Rate limits are counted per message, not per bulk request.
//! - Each message gets its own `request_id` and status record.
//! - HTTP 202 is returned when auth passes and payload structure is valid,
//!   regardless of individual message outcomes.
//! - SMTP submissions are sequential in v0.9.

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    api::{send::MailRequest, ExtractRequestId},
    auth::AuthContext,
    error::AppError,
    mail, smtp, validation,
    request_id::RequestId,
    status::{
        recipient_domains_from, ErrorCode, StatusUpdate, SubmissionStatus,
        SubmissionStatusRecord,
    },
    AppState,
};

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

/// Request body for `POST /v1/send-bulk`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BulkMailRequest {
    pub messages: Vec<MailRequest>,
}

/// Per-message outcome in the bulk response.
#[derive(Debug, Serialize)]
pub struct MessageResult {
    pub index:      usize,
    pub request_id: String,
    pub status:     &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code:       Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message:    Option<String>,
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// POST /v1/send-bulk
///
/// Processes each message independently.  Returns 202 with per-message results.
pub async fn send_bulk(
    State(state):                    State<Arc<AppState>>,
    ExtractRequestId(bulk_req_id):   ExtractRequestId,
    auth:                            AuthContext,
    Json(payload):                   Json<BulkMailRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let cfg = state.config();

    // ── Payload-level validation ───────────────────────────────────────────
    if payload.messages.is_empty() {
        return Err(AppError::BadRequest);
    }
    if payload.messages.len() > cfg.mail.max_bulk_messages {
        tracing::warn!(
            event    = "bulk_too_large",
            key_id   = %auth.key_id,
            count    = payload.messages.len(),
            max      = cfg.mail.max_bulk_messages,
        );
        return Err(AppError::PayloadTooLarge);
    }

    let total = payload.messages.len();
    let mut results: Vec<MessageResult> = Vec::with_capacity(total);
    let mut accepted = 0usize;
    let mut rejected = 0usize;

    tracing::info!(
        event          = "bulk_started",
        key_id         = %auth.key_id,
        client_ip      = %auth.client_ip,
        total          = total,
        bulk_request_id = %bulk_req_id,
    );

    // ── Per-message processing ─────────────────────────────────────────────
    for (index, raw_msg) in payload.messages.into_iter().enumerate() {
        let req_id = RequestId::new();
        let result = process_one(
            index, raw_msg, req_id.clone(), &auth, &state, &cfg,
        ).await;

        match result {
            Ok(()) => {
                accepted += 1;
                results.push(MessageResult {
                    index,
                    request_id: req_id.to_string(),
                    status:     "accepted",
                    code:       None,
                    message:    None,
                });
            }
            Err((code, msg)) => {
                rejected += 1;
                results.push(MessageResult {
                    index,
                    request_id: req_id.to_string(),
                    status:     "rejected",
                    code:       Some(error_code_str(&code)),
                    message:    Some(msg),
                });
            }
        }
    }

    state.metrics.inc_request("2xx");
    tracing::info!(
        event           = "bulk_completed",
        key_id          = %auth.key_id,
        total           = total,
        accepted        = accepted,
        rejected        = rejected,
        bulk_request_id = %bulk_req_id,
    );

    Ok((
        StatusCode::ACCEPTED,
        Json(json!({
            "bulk_request_id": bulk_req_id.to_string(),
            "total":    total,
            "accepted": accepted,
            "rejected": rejected,
            "results":  results,
        })),
    ))
}

// ---------------------------------------------------------------------------
// Per-message pipeline
// ---------------------------------------------------------------------------

/// Returns `Ok(())` on SMTP acceptance, `Err((code, message))` on any failure.
async fn process_one(
    index:   usize,
    raw_msg: MailRequest,
    req_id:  RequestId,
    auth:    &AuthContext,
    state:   &Arc<AppState>,
    cfg:     &Arc<crate::config::AppConfig>,
) -> Result<(), (ErrorCode, String)> {
    // ── Rate limits (counted per message — RFC 702) ────────────────────────
    if let Err(e) = state.rate_limiter.check_global() {
        state.metrics.inc_rate_limited("global");
        tracing::warn!(event = "rate_limited", tier = "global", index, retry_after = e.retry_after_secs);
        record_rejected(&state, &req_id, &auth.key_id, ErrorCode::RateLimited,
            "Global rate limit exceeded.", cfg.status.ttl_seconds);
        return Err((ErrorCode::RateLimited, "Global rate limit exceeded.".into()));
    }

    if let Err(_e) = state.rate_limiter.check_ip(auth.client_ip) {
        state.metrics.inc_rate_limited("ip");
        tracing::warn!(event = "rate_limited", tier = "ip", index, client_ip = %auth.client_ip);
        record_rejected(&state, &req_id, &auth.key_id, ErrorCode::RateLimited,
            "IP rate limit exceeded.", cfg.status.ttl_seconds);
        return Err((ErrorCode::RateLimited, "IP rate limit exceeded.".into()));
    }

    if let Err(_e) = state.rate_limiter.check_key(&auth.key_id, auth.key_rate_limit_per_min, auth.key_burst) {
        state.metrics.inc_rate_limited("key");
        tracing::warn!(event = "rate_limited", tier = "key", index, key_id = %auth.key_id);
        record_rejected(&state, &req_id, &auth.key_id, ErrorCode::RateLimited,
            "Per-key rate limit exceeded.", cfg.status.ttl_seconds);
        return Err((ErrorCode::RateLimited, "Per-key rate limit exceeded.".into()));
    }

    // ── Initial status record (received) ──────────────────────────────────
    state.status_store.put(SubmissionStatusRecord::new(
        req_id.clone(), auth.key_id.clone(), vec![], 0, cfg.status.ttl_seconds,
    ));

    // ── Validate ──────────────────────────────────────────────────────────
    let validated = match validation::validate_mail_request(raw_msg, cfg, auth) {
        Ok(v) => v,
        Err(e) => {
            state.metrics.inc_validation_failure("request");
            tracing::warn!(event = "validation_failure", index, key_id = %auth.key_id, error = %e);
            let code = app_error_to_code(&e);
            state.status_store.update_status(&req_id, &auth.key_id, StatusUpdate {
                status:  SubmissionStatus::Rejected,
                code:    Some(code.clone()),
                message: Some("Request rejected during validation.".into()),
            });
            return Err((code, e.to_string()));
        }
    };

    // Update status record with real recipient domains.
    let domains = recipient_domains_from(&validated.to, &validated.cc);
    let count   = (validated.to.len() + validated.cc.len()) as u32;
    state.status_store.put(SubmissionStatusRecord {
        request_id:        req_id.clone(),
        key_id:            auth.key_id.clone(),
        status:            SubmissionStatus::Received,
        code:              None,
        message:           Some("Validated.".into()),
        recipient_domains: domains,
        recipient_count:   count,
        created_at:        chrono::Utc::now(),
        updated_at:        chrono::Utc::now(),
        expires_at:        chrono::Utc::now()
            + chrono::Duration::seconds(cfg.status.ttl_seconds as i64),
    });

    // ── Build message ──────────────────────────────────────────────────────
    let message = match mail::build_message(&validated, cfg) {
        Ok(m) => m,
        Err(e) => {
            state.status_store.update_status(&req_id, &auth.key_id, StatusUpdate {
                status: SubmissionStatus::Rejected,
                code: Some(ErrorCode::InternalError),
                message: Some("Failed to build mail message.".into()),
            });
            return Err((ErrorCode::InternalError, e.to_string()));
        }
    };

    // ── Status: smtp_submission_started ───────────────────────────────────
    state.status_store.update_status(&req_id, &auth.key_id, StatusUpdate {
        status:  SubmissionStatus::SmtpSubmissionStarted,
        code:    None,
        message: Some("Submitting to SMTP.".into()),
    });

    // ── SMTP submit ───────────────────────────────────────────────────────
    let recipient_domain = validated.to.first()
        .and_then(|a| a.split('@').nth(1)).unwrap_or("unknown");

    let effective_mask = cfg.security.api_keys.iter()
        .find(|k| k.id == auth.key_id)
        .and_then(|k| k.mask_recipient)
        .unwrap_or(cfg.logging.mask_recipient);
    let logged_domain = if effective_mask { "***" } else { recipient_domain };

    let smtp_start = std::time::Instant::now();
    let smtp_res = if cfg.smtp.mode == "pipe" {
        smtp::submit_pipe(message, &cfg.smtp.pipe_command, cfg.smtp.submission_timeout_seconds).await
    } else {
        smtp::submit(&state.smtp, message, cfg.smtp.submission_timeout_seconds).await
    };
    state.metrics.observe_smtp_duration(smtp_start.elapsed().as_secs_f64());

    match smtp_res {
        Ok(()) => {
            state.status_store.update_status(&req_id, &auth.key_id, StatusUpdate {
                status:  SubmissionStatus::SmtpAccepted,
                code:    None,
                message: Some("Accepted by SMTP server.".into()),
            });
            state.metrics.inc_smtp_ok();
            tracing::info!(
                event            = "smtp_submitted",
                key_id           = %auth.key_id,
                recipient_domain = logged_domain,
                index,
                request_id       = %req_id,
            );
            Ok(())
        }
        Err(e) => {
            state.metrics.inc_smtp_error();
            tracing::error!(
                event            = "smtp_failure",
                key_id           = %auth.key_id,
                recipient_domain = logged_domain,
                index,
                error            = %e,
            );
            state.status_store.update_status(&req_id, &auth.key_id, StatusUpdate {
                status:  SubmissionStatus::SmtpFailed,
                code:    Some(ErrorCode::SmtpUnavailable),
                message: Some("SMTP server unavailable.".into()),
            });
            Err((ErrorCode::SmtpUnavailable, "SMTP server unavailable.".into()))
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn record_rejected(
    state: &Arc<AppState>,
    req_id: &RequestId,
    key_id: &str,
    code: ErrorCode,
    msg: &str,
    ttl: u64,
) {
    state.status_store.put(SubmissionStatusRecord {
        request_id:        req_id.clone(),
        key_id:            key_id.to_string(),
        status:            SubmissionStatus::Rejected,
        code:              Some(code),
        message:           Some(msg.to_string()),
        recipient_domains: vec![],
        recipient_count:   0,
        created_at:        chrono::Utc::now(),
        updated_at:        chrono::Utc::now(),
        expires_at:        chrono::Utc::now()
            + chrono::Duration::seconds(ttl as i64),
    });
}

fn app_error_to_code(e: &AppError) -> ErrorCode {
    match e {
        AppError::Validation(_)        => ErrorCode::ValidationFailed,
        AppError::BadRequest           => ErrorCode::BadRequest,
        AppError::PayloadTooLarge      => ErrorCode::PayloadTooLarge,
        AppError::UnsupportedMediaType => ErrorCode::UnsupportedMediaType,
        AppError::RateLimited { .. }   => ErrorCode::RateLimited,
        AppError::Forbidden            => ErrorCode::Forbidden,
        AppError::SmtpUnavailable      => ErrorCode::SmtpUnavailable,
        _                              => ErrorCode::InternalError,
    }
}

fn error_code_str(c: &ErrorCode) -> String {
    serde_json::to_value(c)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
}
