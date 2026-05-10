# RFC 840 — MailTransport Trait Abstraction (Long-term)

**Status.** Proposed  
**Tracks.** T8 — Extensibility  
**Touches.** `src/smtp.rs`, `src/lib.rs`, `src/api/send.rs`

## Problem

`AppState` holds a concrete `SmtpTransport` field. Pipe mode is handled
by an `if cfg.smtp.mode == "pipe"` branch inside the handler:

```rust
if cfg.smtp.mode == "pipe" {
    smtp::submit_pipe(message, &cfg.smtp).await?
} else {
    smtp::submit(&state.smtp, message).await?
}
```

This couples transport selection to the handler, makes testing difficult
(cannot swap transport), and will not scale to future transport types
(sendgrid API, mock in tests).

## Decision

```rust
#[async_trait]  // or native async trait (Rust 2024)
pub trait MailTransport: Send + Sync {
    async fn submit(&self, message: lettre::Message) -> Result<(), AppError>;
    async fn ready(&self) -> TransportReadiness;
}

pub enum TransportReadiness {
    Ready,
    Unavailable(String),
}
```

`AppState` holds `Arc<dyn MailTransport>`:

```rust
pub struct AppState {
    pub transport: Arc<dyn MailTransport>,
    ...
}
```

Implementations:
- `SmtpMailTransport` — wraps `lettre::AsyncSmtpTransport`
- `PipeMailTransport` — wraps `tokio::process::Command`
- `StubMailTransport` — for integration tests (replaces `SmtpStub` coupling)

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-840-01 | Transport selection is resolved at startup, not per-request. |
| AC-840-02 | Handler code contains no transport-mode branch. |
| AC-840-03 | Integration tests use `StubMailTransport` rather than real SMTP. |
| AC-840-04 | `GET /readyz` calls `transport.ready()`. |
