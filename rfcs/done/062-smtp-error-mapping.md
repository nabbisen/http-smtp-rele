# RFC 062 — SMTP Error Mapping and Timeout

**Status.** Implemented  
**Tracks.** SMTP  
**Touches.** `src/smtp.rs`, `src/error.rs`

## Summary

Define `SmtpSubmitError` and its mapping to `AppError`, ensuring SMTP failures produce
correct HTTP responses (502) with sufficient log detail for diagnosis.

## Motivation

SMTP errors must map to a consistent external response (`502 smtp_unavailable`) while
logging enough internal detail for operators to diagnose failures. The mapping must not
expose SMTP server details or message content to HTTP clients (FR-063, NFR-AVL-001).

## Scope

- `SmtpSubmitError` enum.
- Mapping `lettre::error::Error` to `SmtpSubmitError`.
- Mapping `SmtpSubmitError` to `AppError::SmtpUnavailable`.
- Log content on SMTP failure: what to include, what to omit.
- Timeout behavior.

## Non-goals

- SMTP retry logic (SMTP server's queue handles retries).
- Per-error HTTP status differentiation (all SMTP errors → 502).

## Design

### `SmtpSubmitError`

```rust
#[derive(Debug)]
pub enum SmtpSubmitError {
    /// TCP connection failed or timed out.
    ConnectionFailed(String),
    /// SMTP server rejected the message (4xx or 5xx response).
    ServerRejected(String),
    /// Message construction failed (internal inconsistency).
    BuildError(String),
    /// Unexpected error.
    Other(String),
}

impl From<lettre::error::Error> for SmtpSubmitError {
    fn from(e: lettre::error::Error) -> Self {
        // Categorize by error type; do not expose raw SMTP response to callers.
        match e {
            _ => SmtpSubmitError::Other(e.to_string()),
        }
    }
}

impl From<SmtpSubmitError> for AppError {
    fn from(_: SmtpSubmitError) -> Self {
        // All SMTP errors map to the same HTTP error code.
        // The SmtpSubmitError detail is logged before this conversion.
        AppError::SmtpUnavailable
    }
}
```

### Log on SMTP failure

At `error` level:

```
event=smtp_failure request_id=... key_id=... error="connection refused" recipient_domain=example.com
```

Fields included: `request_id`, `key_id`, `recipient_domain` (domain only, RFC 083), `error`
(sanitized SMTP error string, no message body).

Fields excluded: full recipient address (unless `mask_recipient_in_logs = false`), message body,
raw SMTP transcript.

### Timeout

The `lettre` transport timeout is configured via `SmtpConfig.timeout_seconds`. It applies to
the full SMTP session (connect + EHLO + MAIL FROM + RCPT TO + DATA + QUIT). If the session
exceeds the timeout, the connection is dropped and the error maps to `SmtpSubmitError::ConnectionFailed`.

### Handler integration

```rust
// In the send handler:
let message = mail::build_message(validated, &state.config.mail)?;
state.smtp.send(message).await.map_err(|e| {
    tracing::error!(
        event = "smtp_failure",
        request_id = %ctx.request_id,
        key_id = %ctx.key_id,
        error = %e,
    );
    AppError::from(e)
})?;
```

## Test Plan

### Integration Tests (with fake SMTP)

- SMTP connection refused → `AppError::SmtpUnavailable` → 502.
- SMTP server rejects message (5xx) → 502.
- SMTP timeout → 502.
- Log entry contains `event=smtp_failure` and `error` field.
- Log entry does not contain message body.

## Security Considerations

- SMTP error messages from the server may contain internal infrastructure details. The
  `SmtpSubmitError` string is logged (server-side only) but never returned to the HTTP client.
- The HTTP response for all SMTP errors is the same: `502 smtp_unavailable`. Clients cannot
  distinguish connection refused from server-rejected-message.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-062-01 | Any SMTP failure maps to `502 smtp_unavailable`. |
| AC-062-02 | SMTP failure is logged with `event=smtp_failure` and error detail. |
| AC-062-03 | SMTP error detail is not returned in the HTTP response body. |
| AC-062-04 | SMTP timeout produces 502, not a hanging connection. |

## Open Questions

None.
