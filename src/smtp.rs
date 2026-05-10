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

/// Build the SMTP transport from config (RFC 301).
///
/// Uses unencrypted ("dangerous") transport for loopback relay.
/// When `auth_user` and `auth_password` are both set, SMTP AUTH is configured.
///
/// # Errors
///
/// Returns `AppError::Internal` if the transport cannot be constructed.
pub fn build_transport(cfg: &SmtpConfig) -> Result<SmtpTransport, AppError> {
    let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&cfg.host)
        .port(cfg.port)
        .timeout(Some(Duration::from_secs(cfg.connect_timeout_seconds)));

    if let (Some(user), Some(pass)) = (&cfg.auth_user, &cfg.auth_password) {
        use lettre::transport::smtp::authentication::Credentials;
        builder = builder.credentials(Credentials::new(
            user.clone(),
            pass.expose().to_string(),
        ));
        tracing::debug!(smtp_user = %user, "SMTP AUTH enabled");
    }

    Ok(builder.build())
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

// ---------------------------------------------------------------------------
// Pipe mode submission (RFC 304)
// ---------------------------------------------------------------------------

/// Submit a mail message via a pipe command (e.g., `sendmail -t`).
///
/// The message is formatted as a raw RFC 5322 string and piped to the
/// configured command via stdin. Used when `smtp.mode = "pipe"`.
///
/// # Errors
///
/// Returns `AppError::SmtpUnavailable` if the command fails to start or exits
/// with a non-zero status code.
pub async fn submit_pipe(
    message: Message,
    pipe_command: &str,
    timeout_seconds: u64,
) -> Result<(), AppError> {
    use tokio::process::Command;
    use tokio::io::AsyncWriteExt;
    use std::process::Stdio;

    // Format the message to bytes (RFC 5322).
    let raw = message.formatted();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_seconds),
        async {
            let mut child = Command::new(pipe_command)
                .arg("-t")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| {
                    tracing::error!(
                        command = pipe_command,
                        error = %e,
                        "failed to spawn pipe command"
                    );
                    AppError::SmtpUnavailable
                })?;

            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(&raw).await.map_err(|e| {
                    tracing::error!(error = %e, "failed to write to pipe command stdin");
                    AppError::SmtpUnavailable
                })?;
            }

            let output = child.wait_with_output().await.map_err(|e| {
                tracing::error!(error = %e, "pipe command wait failed");
                AppError::SmtpUnavailable
            })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::error!(
                    command = pipe_command,
                    exit_code = ?output.status.code(),
                    stderr = %stderr,
                    "pipe command exited with error"
                );
                return Err(AppError::SmtpUnavailable);
            }

            Ok(())
        }
    ).await;

    match result {
        Ok(r) => r,
        Err(_elapsed) => {
            tracing::error!(
                command = pipe_command,
                timeout_seconds,
                "pipe command timed out"
            );
            Err(AppError::SmtpUnavailable)
        }
    }
}
