# RFC 061 — SMTP Relay Transport

**Status.** Implemented  
**Tracks.** SMTP  
**Touches.** `src/smtp.rs`

## Summary

Define `SmtpHandle` — the async wrapper around `lettre`'s SMTP transport — including
initialization, connection pooling strategy, and message submission.

## Motivation

The SMTP submission step is where the relay actually delivers value. It must be reliable,
timeout-bounded, and produce clear errors that map to the correct HTTP response codes
(FR-060, FR-063, NFR-AVL-001).

## Scope

- `SmtpHandle` struct and its construction from `SmtpConfig`.
- Async SMTP submission: `SmtpHandle::send(message) → Result<(), SmtpError>`.
- TCP probe for readiness: `SmtpHandle::probe() → Result<(), ()>` (used by RFC 034).
- Timeout enforcement.
- `lettre::AsyncSmtpTransport<Tokio1Executor>` as the underlying transport.

## Non-goals

- SMTP error mapping (RFC 062).
- Sendmail pipe mode (RFC 064 — deferred).
- SMTP AUTH (not in MVP).
- Connection pooling with per-message reuse (lettre manages this internally).

## Design

### `SmtpHandle`

```rust
pub struct SmtpHandle {
    transport: lettre::AsyncSmtpTransport<lettre::Tokio1Executor>,
    host: String,
    port: u16,
    timeout: Duration,
}

impl SmtpHandle {
    /// Build an SMTP transport from configuration.
    pub fn from_config(config: &SmtpConfig) -> Result<Self, SmtpBuildError> {
        let transport = match config.tls {
            SmtpTls::None => {
                lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::builder_dangerous(&config.host)
                    .port(config.port)
                    .timeout(Some(Duration::from_secs(config.timeout_seconds)))
                    .hello_name(lettre::transport::smtp::extension::ClientId::Domain(
                        config.helo_name.clone(),
                    ))
                    .build()
            }
            SmtpTls::StartTls => {
                lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::starttls_relay(&config.host)?
                    .port(config.port)
                    .timeout(Some(Duration::from_secs(config.timeout_seconds)))
                    .build()
            }
            SmtpTls::Tls => {
                lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::relay(&config.host)?
                    .port(config.port)
                    .timeout(Some(Duration::from_secs(config.timeout_seconds)))
                    .build()
            }
        };

        Ok(Self {
            transport,
            host: config.host.clone(),
            port: config.port,
            timeout: Duration::from_secs(config.timeout_seconds),
        })
    }

    /// Submit a message to the SMTP server.
    pub async fn send(&self, message: lettre::Message) -> Result<(), SmtpSubmitError> {
        use lettre::AsyncTransport;
        self.transport
            .send(message)
            .await
            .map(|_| ())
            .map_err(SmtpSubmitError::from)
    }

    /// TCP-level connectivity probe for /readyz.
    pub async fn probe(&self) -> Result<(), ()> {
        let addr = format!("{}:{}", self.host, self.port);
        tokio::time::timeout(self.timeout, tokio::net::TcpStream::connect(&addr))
            .await
            .ok()
            .and_then(|r| r.ok())
            .map(|_| ())
            .ok_or(())
    }
}
```

### Connection reuse

`lettre::AsyncSmtpTransport` internally manages connection pooling. Multiple `send` calls
reuse established connections when possible. This is transparent to `SmtpHandle`.

### `AppState` integration

```rust
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub rate_limiter: Arc<RateLimiter>,
    pub smtp: Arc<SmtpHandle>,   // shared across all requests
}
```

`SmtpHandle` is constructed once during startup and shared via `Arc`.

## Implementation Plan

1. Define `SmtpHandle` in `src/smtp.rs`.
2. Implement `from_config`.
3. Implement `send`.
4. Implement `probe`.
5. Define `SmtpSubmitError` (RFC 062).
6. Construct `SmtpHandle` in `main.rs` and add to `AppState`.
7. Call `state.smtp.send(message)` in the send handler.

## Test Plan

### Integration Tests (with fake SMTP)

- Valid message is submitted and SMTP server receives it.
- SMTP unavailable → error is returned (mapped in RFC 062).
- Timeout → error is returned.

### Operational Tests

- `probe()` returns `Ok` when a TCP listener is active on the configured port.
- `probe()` returns `Err` when no listener is active.

## Security Considerations

- `builder_dangerous` (no TLS) is appropriate only for localhost. The `SmtpTls::None` path
  should only be reachable when `host` is a loopback address. This is documented but not
  programmatically enforced in MVP (an operator warning in RFC 021 is sufficient).
- The `SmtpHandle` is shared across requests via `Arc`. It must not carry per-request state.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-061-01 | `SmtpHandle::from_config` succeeds with the example SMTP config. |
| AC-061-02 | `SmtpHandle::send` submits a `lettre::Message` to a test SMTP listener. |
| AC-061-03 | `SmtpHandle::probe` returns `Ok` when the SMTP port is open. |
| AC-061-04 | `SmtpHandle::probe` returns `Err` when the SMTP port is closed. |

## Open Questions

None.
