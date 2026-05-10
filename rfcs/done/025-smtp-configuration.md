# RFC 025 — SMTP Configuration

**Status.** Implemented  
**Tracks.** Foundation  
**Touches.** `src/config.rs`, `src/smtp.rs`

## Summary

Define `SmtpConfig` and how it controls the SMTP transport layer: SMTP relay mode vs. sendmail
pipe mode, host/port/timeout, TLS options, and the EHLO hostname.

## Motivation

The SMTP configuration determines how the relay submits mail to the local SMTP server. Getting
this wrong causes silent delivery failures or security gaps (no TLS on a non-localhost relay,
incorrect EHLO identity). The configuration must be well-defined before the transport layer
(RFC 061) is implemented (FR-060, FR-061, FR-062).

## Scope

- `SmtpConfig` struct definition.
- `SmtpMode` enum: `Smtp` vs. `Pipe`.
- `SmtpTls` enum: `None`, `StartTls`, `Tls`.
- Validation rules (integrated with RFC 021).
- How `SmtpConfig` fields translate to `lettre` transport options.

## Non-goals

- Actual transport implementation (RFC 061).
- SMTP error mapping (RFC 062).
- SMTP authentication (not in MVP; SMTP auth is for non-localhost relays).
- Sendmail pipe implementation (RFC 064 — deferred).

## Design

### `SmtpMode`

```rust
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SmtpMode {
    /// Connect to the SMTP server via TCP.
    Smtp,
    /// Fork the sendmail-compatible binary and write RFC 5322 message to stdin.
    Pipe,
}
```

Default: `SmtpMode::Smtp`.

### `SmtpTls`

```rust
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SmtpTls {
    /// No TLS. Appropriate for localhost connections.
    None,
    /// Upgrade to TLS via STARTTLS after connection.
    StartTls,
    /// Connect directly to a TLS port (port 465).
    Tls,
}
```

Default: `SmtpTls::None` (localhost assumption for MVP).

### `SmtpConfig`

```rust
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct SmtpConfig {
    /// Submission mode.
    pub mode: SmtpMode,

    /// SMTP server hostname or IP. Default: "127.0.0.1"
    pub host: String,

    /// SMTP server port. Default: 25
    pub port: u16,

    /// Connection and read timeout in seconds. Default: 10
    pub timeout_seconds: u64,

    /// EHLO hostname sent to the SMTP server. Default: "localhost"
    pub helo_name: String,

    /// TLS mode. Default: None
    pub tls: SmtpTls,

    /// Path to sendmail binary. Only used when mode = Pipe.
    /// Default: "/usr/sbin/sendmail"
    pub pipe_command: String,
}
```

### Defaults

```rust
impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            mode: SmtpMode::Smtp,
            host: "127.0.0.1".into(),
            port: 25,
            timeout_seconds: 10,
            helo_name: "localhost".into(),
            tls: SmtpTls::None,
            pipe_command: "/usr/sbin/sendmail".into(),
        }
    }
}
```

### Validation rules

| Field | Rule |
|-------|------|
| `host` | Not empty |
| `port` | 1–65535 |
| `timeout_seconds` | > 0 |
| `helo_name` | Not empty |
| `mode = Pipe` | `pipe_command` must not be empty |
| `tls != None` when `host = 127.0.0.1` | Emit `debug!`; non-localhost TLS is fine |

### Mapping to `lettre`

In `smtp.rs`, `SmtpConfig` maps to `lettre::transport::smtp::SmtpTransport`:

| `SmtpConfig` | `lettre` configuration |
|-------------|----------------------|
| `mode = Smtp`, `tls = None` | `SmtpTransport::builder_dangerous(host).port(port)` |
| `mode = Smtp`, `tls = StartTls` | `SmtpTransport::starttls_relay(host)?.port(port)` |
| `mode = Smtp`, `tls = Tls` | `SmtpTransport::relay(host)?.port(port)` |
| `mode = Pipe` | `SendmailTransport::new_with_command(pipe_command)` |

Timeout is applied via `SmtpTransportBuilder::timeout(Duration::from_secs(...))`.

The exact API calls depend on the lettre version in use; the transport RFC (061) documents
the final implementation.

## Implementation Plan

1. Define `SmtpMode`, `SmtpTls`, `SmtpConfig` in `src/config.rs`.
2. Implement `Default for SmtpConfig`.
3. Add validation rules to `config::validate` (RFC 021).
4. Write unit tests for TOML parsing of the enum values.

## Test Plan

### Unit Tests

- `mode = "smtp"` parses as `SmtpMode::Smtp`.
- `mode = "pipe"` parses as `SmtpMode::Pipe`.
- `tls = "none"` parses as `SmtpTls::None`.
- `tls = "starttls"` parses as `SmtpTls::StartTls`.
- `port = 0` fails validation.
- `port = 65535` passes validation.
- `host = ""` fails validation.
- Default config has `host = "127.0.0.1"`, `port = 25`.

## Security Considerations

- `SmtpTls::None` is the default for localhost, where TLS overhead is unnecessary.
  Operators must set `tls = "starttls"` or `tls = "tls"` for any non-localhost SMTP relay
  to prevent credential and message interception.
- The `pipe_command` must not be user-supplied at request time — it is a config-time constant.
  This prevents command injection via the SMTP pipe path.
- SMTP AUTH credentials are not in scope for MVP. If the SMTP server requires AUTH, the
  operator should use a local SMTP server that accepts unauthenticated connections from localhost.

## Operational Considerations

- The SMTP timeout applies to the entire SMTP session (connect + EHLO + MAIL + RCPT + DATA).
  A 10-second default is generous for localhost; tighten for high-throughput scenarios.
- `helo_name` should match the MTA's configured hostname for interoperability.
- `pipe_command` is only needed on systems where `sendmail` pipe mode is preferred over TCP.

## Documentation Changes

- Document all `[smtp]` fields in `docs/configuration.md`.
- Document the TLS guidance in `docs/security.md`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-025-01 | `mode = "smtp"` and `mode = "pipe"` parse correctly. |
| AC-025-02 | `tls = "none"`, `"starttls"`, `"tls"` parse correctly. |
| AC-025-03 | `port = 0` fails config validation. |
| AC-025-04 | Default `SmtpConfig` has `host = "127.0.0.1"` and `port = 25`. |

## Open Questions

- Whether to add SMTP AUTH support in v0.1. Decision: not in MVP. If the upstream SMTP server
  requires AUTH, the operator should configure a local relay (e.g., OpenSMTPD) to accept
  unauthenticated local connections.
