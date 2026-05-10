//! SMTP relay transport.
//!
//! Implements RFC 062–063: initializes the async SMTP transport to localhost:25
//! and submits mail messages. SMTP errors are mapped to `AppError`.
//!
//! # Transport
//!
//! Uses `lettre::AsyncSmtpTransport` over plain TCP to localhost (no TLS
//! needed for loopback relay). The transport is initialized once and stored
//! in `AppState`.
//!
//! # Timeout
//!
//! Submission is wrapped in `tokio::time::timeout` using
//! `config.smtp.submission_timeout_seconds`.

use std::time::Duration;

use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use tokio::time::timeout;
use tracing::error;

use crate::{config::SmtpConfig, error::AppError};

// ---------------------------------------------------------------------------
// Transport type alias
// ---------------------------------------------------------------------------

/// Async SMTP transport used for all submissions.
pub type SmtpTransport = AsyncSmtpTransport<Tokio1Executor>;

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

/// Build the SMTP transport from config.
///
/// Uses unencrypted ("dangerous") transport to localhost — appropriate for
/// loopback relay to a local SMTP daemon.
///
/// # Errors
///
/// Returns `AppError::Internal` if the transport cannot be constructed
/// (e.g., invalid host name).
pub fn build_transport(cfg: &SmtpConfig) -> Result<SmtpTransport, AppError> {
    let transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&cfg.host)
        .port(cfg.port)
        .timeout(Some(Duration::from_secs(cfg.connect_timeout_seconds)))
        .build();
    Ok(transport)
}

// ---------------------------------------------------------------------------
// Submission
// ---------------------------------------------------------------------------

/// Submit a `lettre::Message` to the SMTP server.
///
/// - Wraps the SMTP call in a timeout (`submission_timeout_seconds`).
/// - Maps SMTP-level errors to `AppError::SmtpUnavailable`.
///
/// # Errors
///
/// | Condition           | Error                  |
/// |---------------------|------------------------|
/// | Timeout             | `AppError::SmtpUnavailable` |
/// | Connection refused  | `AppError::SmtpUnavailable` |
/// | SMTP rejection      | `AppError::SmtpUnavailable` |
pub async fn submit(
    transport: &SmtpTransport,
    message: Message,
    timeout_seconds: u64,
) -> Result<(), AppError> {
    let result = timeout(
        Duration::from_secs(timeout_seconds),
        transport.send(message),
    )
    .await;

    match result {
        Ok(Ok(_response)) => Ok(()),
        Ok(Err(e)) => {
            error!(smtp_error = %e, "SMTP submission failed");
            Err(AppError::SmtpUnavailable)
        }
        Err(_elapsed) => {
            error!("SMTP submission timed out after {timeout_seconds}s");
            Err(AppError::SmtpUnavailable)
        }
    }
}

// ---------------------------------------------------------------------------
// Readiness check (RFC 064)
// ---------------------------------------------------------------------------

/// Test whether the SMTP server is reachable by attempting a TCP connection.
///
/// Used by `/readyz`. Returns `true` if SMTP responds, `false` otherwise.
pub async fn is_smtp_reachable(cfg: &SmtpConfig) -> bool {
    let addr = format!("{}:{}", cfg.host, cfg.port);
    timeout(
        Duration::from_secs(cfg.connect_timeout_seconds),
        tokio::net::TcpStream::connect(&addr),
    )
    .await
    .map(|r| r.is_ok())
    .unwrap_or(false)
}
