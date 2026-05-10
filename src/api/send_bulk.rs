//! `POST /v1/send-bulk` handler — two-phase processing with bounded SMTP parallelism.
//!
//! Implements RFC 701 (bulk API), RFC 702 (rate limiting), RFC 711 (SMTP parallelism).
//!
//! # Processing pipeline
//!
//! Phase 1 (sequential): rate limit → validate → build message → PreparedMessage or rejection
//! Phase 2 (parallel):   bounded by `[smtp].bulk_concurrency` semaphore → SMTP submit
//!
//! Rate limit checks remain sequential for fairness.
//! Results are merged and sorted by request index before serialisation.

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::{
    api::{send::MailRequest, ExtractRequestId},
    auth::AuthContext,
    error::AppError,
    mail, smtp, validation,
    config::AppConfig,
    request_id::RequestId,
    status::{
        recipient_domains_from, ErrorCode, StatusUpdate, SubmissionStatus,
        SubmissionStatusRecord,
    },
    AppState,
};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BulkMailRequest {
    pub messages: Vec<MailRequest>,
}

/// Per-message result in the bulk response.
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

/// Output of Phase 1: a message ready for SMTP submission.
struct PreparedMessage {
    index:            usize,
    req_id:           RequestId,
    key_id:           String,
    message:          lettre::Message,
    recipient_domain: String,
    effective_mask:   bool,
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn send_bulk(
    State(state):                  State<Arc<AppState>>,
    ExtractRequestId(bulk_req_id): ExtractRequestId,
    auth:                          AuthContext,
    Json(payload):                 Json<BulkMailRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let cfg = state.config();

    // Payload-level guards
    if payload.messages.is_empty() {
        return Err(AppError::BadRequest);
    }
    if payload.messages.len() > cfg.mail.max_bulk_messages {
        tracing::warn!(
            event   = "bulk_too_large",
            key_id  = %auth.key_id,
            count   = payload.messages.len(),
            max     = cfg.mail.max_bulk_messages,
        );
        return Err(AppError::PayloadTooLarge);
    }

    let total = payload.messages.len();
    tracing::info!(
        event           = "bulk_started",
        key_id          = %auth.key_id,
        client_ip       = %auth.client_ip,
        total,
        bulk_request_id = %bulk_req_id,
    );

    // ── Phase 1: sequential rate limit + validate + build ─────────────────
    let mut rejected: Vec<MessageResult> = Vec::new();
    let mut prepared: Vec<PreparedMessage> = Vec::new();

    for (index, raw_msg) in payload.messages.into_iter().enumerate() {
        let req_id = RequestId::new();
        match phase1_prepare(index, raw_msg, req_id.clone(), &auth, &state, &cfg).await {
            Ok(p)  => prepared.push(p),
            Err(r) => rejected.push(r),
        }
    }

    // ── Phase 2: bounded-parallel SMTP submission (RFC 711) ───────────────
    let concurrency = match cfg.smtp.bulk_concurrency {
        0 => prepared.len().max(1),
        n => n,
    };
    let sem = Arc::new(Semaphore::new(concurrency));
    let mut join_set: JoinSet<MessageResult> = JoinSet::new();

    for p in prepared {
        let sem   = Arc::clone(&sem);
        let state = Arc::clone(&state);
        let cfg   = Arc::clone(&cfg);
        join_set.spawn(async move {
            let _permit = sem.acquire().await.expect("semaphore closed");
            phase2_submit(p, &state, &cfg).await
        });
    }

    let mut submitted: Vec<MessageResult> = Vec::with_capacity(join_set.len());
    while let Some(res) = join_set.join_next().await {
        submitted.push(res.expect("bulk task panicked"));
    }

    // Merge + sort by index to guarantee response order matches request order.
    let mut all_results: Vec<MessageResult> =
        rejected.into_iter().chain(submitted).collect();
    all_results.sort_by_key(|r| r.index);

    let accepted  = all_results.iter().filter(|r| r.status == "accepted").count();
    let n_rejected = all_results.iter().filter(|r| r.status != "accepted").count();

    state.metrics.inc_request("2xx");
    tracing::info!(
        event           = "bulk_completed",
        key_id          = %auth.key_id,
        total,
        accepted,
        rejected        = n_rejected,
        bulk_request_id = %bulk_req_id,
    );

    Ok((
        StatusCode::ACCEPTED,
        Json(json!({
            "bulk_request_id": bulk_req_id.to_string(),
            "total":    total,
            "accepted": accepted,
            "rejected": n_rejected,
            "results":  all_results,
        })),
    ))
}

// ---------------------------------------------------------------------------
// Phase 1 — sequential: rate limit, validate, build
// ---------------------------------------------------------------------------

async fn phase1_prepare(
    index:   usize,
    raw_msg: MailRequest,
    req_id:  RequestId,
    auth:    &AuthContext,
    state:   &Arc<AppState>,
    cfg:     &Arc<AppConfig>,
) -> Result<PreparedMessage, MessageResult> {
    // Rate limits — counted per message (RFC 702)
    if let Err(e) = state.rate_limiter.check_global() {
        state.metrics.inc_rate_limited("global");
        tracing::warn!(event = "rate_limited", tier = "global", index,
            retry_after = e.retry_after_secs);
        store_rejected(state, &req_id, &auth.key_id, ErrorCode::RateLimited,
            "Global rate limit exceeded.", cfg.status.ttl_seconds);
        return Err(rejected_result(index, req_id, ErrorCode::RateLimited,
            "Global rate limit exceeded."));
    }
    if let Err(_) = state.rate_limiter.check_ip(auth.client_ip) {
        state.metrics.inc_rate_limited("ip");
        tracing::warn!(event = "rate_limited", tier = "ip", index,
            client_ip = %auth.client_ip);
        store_rejected(state, &req_id, &auth.key_id, ErrorCode::RateLimited,
            "IP rate limit exceeded.", cfg.status.ttl_seconds);
        return Err(rejected_result(index, req_id, ErrorCode::RateLimited,
            "IP rate limit exceeded."));
    }
    if let Err(_) = state.rate_limiter.check_key(
        &auth.key_id, auth.key_rate_limit_per_min, auth.key_burst)
    {
        state.metrics.inc_rate_limited("key");
        tracing::warn!(event = "rate_limited", tier = "key", index,
            key_id = %auth.key_id);
        store_rejected(state, &req_id, &auth.key_id, ErrorCode::RateLimited,
            "Per-key rate limit exceeded.", cfg.status.ttl_seconds);
        return Err(rejected_result(index, req_id, ErrorCode::RateLimited,
            "Per-key rate limit exceeded."));
    }

    // Initial status record
    state.status_store.put(SubmissionStatusRecord::new(
        req_id.clone(), auth.key_id.clone(), vec![], 0, cfg.status.ttl_seconds,
    ));

    // Validate
    let validated = match validation::validate_mail_request(raw_msg, cfg, auth) {
        Ok(v) => v,
        Err(e) => {
            state.metrics.inc_validation_failure("request");
            tracing::warn!(event = "validation_failure", index, key_id = %auth.key_id,
                error = %e);
            let code = apperr_to_code(&e);
            state.status_store.update_status(&req_id, &auth.key_id, StatusUpdate {
                status:  SubmissionStatus::Rejected,
                code:    Some(code.clone()),
                message: Some("Request rejected during validation.".into()),
            });
            return Err(rejected_result(index, req_id, code, e.to_string()));
        }
    };

    // Update status with real recipient domains
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

    let recipient_domain = validated.to.first()
        .and_then(|a| a.split('@').nth(1))
        .unwrap_or("unknown")
        .to_string();
    let effective_mask = cfg.security.api_keys.iter()
        .find(|k| k.id == auth.key_id)
        .and_then(|k| k.mask_recipient)
        .unwrap_or(cfg.logging.mask_recipient);

    // Build lettre Message
    let message = match mail::build_message(&validated, cfg) {
        Ok(m) => m,
        Err(e) => {
            state.status_store.update_status(&req_id, &auth.key_id, StatusUpdate {
                status:  SubmissionStatus::Rejected,
                code:    Some(ErrorCode::InternalError),
                message: Some("Failed to build mail message.".into()),
            });
            return Err(rejected_result(index, req_id, ErrorCode::InternalError,
                e.to_string()));
        }
    };

    Ok(PreparedMessage {
        index,
        req_id,
        key_id: auth.key_id.clone(),
        message,
        recipient_domain,
        effective_mask,
    })
}

// ---------------------------------------------------------------------------
// Phase 2 — parallel SMTP submission (runs inside JoinSet tasks)
// ---------------------------------------------------------------------------

async fn phase2_submit(
    p:     PreparedMessage,
    state: &Arc<AppState>,
    cfg:   &Arc<AppConfig>,
) -> MessageResult {
    state.status_store.update_status(&p.req_id, &p.key_id, StatusUpdate {
        status:  SubmissionStatus::SmtpSubmissionStarted,
        code:    None,
        message: Some("Submitting to SMTP.".into()),
    });

    let logged_domain = if p.effective_mask { "***" } else { &p.recipient_domain };

    let smtp_start = std::time::Instant::now();
    let result = if cfg.smtp.mode == "pipe" {
        smtp::submit_pipe(p.message, &cfg.smtp.pipe_command,
            cfg.smtp.submission_timeout_seconds).await
    } else {
        smtp::submit(&state.smtp, p.message,
            cfg.smtp.submission_timeout_seconds).await
    };
    state.metrics.observe_smtp_duration(smtp_start.elapsed().as_secs_f64());

    match result {
        Ok(()) => {
            state.status_store.update_status(&p.req_id, &p.key_id, StatusUpdate {
                status:  SubmissionStatus::SmtpAccepted,
                code:    None,
                message: Some("Accepted by SMTP server.".into()),
            });
            state.metrics.inc_smtp_ok();
            tracing::info!(event = "smtp_submitted", key_id = %p.key_id,
                recipient_domain = logged_domain, index = p.index,
                request_id = %p.req_id);
            MessageResult {
                index:      p.index,
                request_id: p.req_id.to_string(),
                status:     "accepted",
                code:       None,
                message:    None,
            }
        }
        Err(e) => {
            state.metrics.inc_smtp_error();
            tracing::error!(event = "smtp_failure", key_id = %p.key_id,
                recipient_domain = logged_domain, index = p.index, error = %e);
            state.status_store.update_status(&p.req_id, &p.key_id, StatusUpdate {
                status:  SubmissionStatus::SmtpFailed,
                code:    Some(ErrorCode::SmtpUnavailable),
                message: Some("SMTP server unavailable.".into()),
            });
            rejected_result(p.index, p.req_id, ErrorCode::SmtpUnavailable,
                "SMTP server unavailable.")
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn store_rejected(
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

fn rejected_result(
    index: usize,
    req_id: RequestId,
    code: ErrorCode,
    message: impl Into<String>,
) -> MessageResult {
    let code_str = serde_json::to_value(&code)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();
    MessageResult {
        index,
        request_id: req_id.to_string(),
        status:     "rejected",
        code:       Some(code_str),
        message:    Some(message.into()),
    }
}

fn apperr_to_code(e: &AppError) -> ErrorCode {
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
