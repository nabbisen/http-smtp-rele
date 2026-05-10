# RFC 013 — Logging Foundation

**Status.** Implemented  
**Tracks.** Foundation  
**Touches.** `src/logging.rs`, `src/main.rs`

## Summary

Initialize the `tracing` subscriber, define log level defaults, establish the rule that
`stderr` is the output target, and set the convention for structured log fields used across
the codebase. Full audit event taxonomy is RFC 081; secret/body redaction is RFC 082.

## Motivation

Logging must be initialized before any other system component, because startup errors (config
parse failures, port bind failures) need to be recorded. The format and field conventions must
be established early so every subsequent RFC can assume a consistent logging substrate
(NFR-OPS-004, NFR-SEC-007).

## Scope

- `logging::init()` function: sets up the `tracing_subscriber` registry.
- Log level: configurable via `RUST_LOG` env variable; default `info`.
- Output target: `stderr` (compatible with `syslogd` piping on OpenBSD).
- Format: human-readable compact by default; JSON optional (config flag).
- Standard field names used across all log events.
- `#[instrument]` usage convention.

## Non-goals

- Audit event taxonomy (RFC 081).
- Secret redaction enforcement (RFC 082).
- Recipient masking (RFC 083).
- JSON log format configuration surface (RFC 084).
- `request_id` injection into log spans (RFC 011 / RFC 083).

## Design

### `logging::init()`

Called as the very first statement in `main()`, before config loading, so even config parse
errors are logged properly.

```rust
pub fn init(level: &str, json: bool) {
    use tracing_subscriber::{fmt, EnvFilter, prelude::*};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));

    if json {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json().with_writer(std::io::stderr))
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_writer(std::io::stderr))
            .init();
    }
}
```

Called from `main.rs` before anything else:

```rust
fn main() {
    logging::init("info", false); // level and json flag come from early CLI parse
    // ... rest of startup
}
```

For the MVP, the `json` flag defaults to `false` and is set from the `[logging]` section of
the config (RFC 084). During the initial startup before config is loaded, plain text is used.

### Log levels

| Level | Usage |
|-------|-------|
| `error` | Unrecoverable failures: SMTP down, internal panic, startup fail |
| `warn` | Recoverable anomalies: auth failure, rate limit, validation rejection |
| `info` | Normal lifecycle: startup, shutdown, mail accepted |
| `debug` | Diagnostic: config loaded, SMTP connected |
| `trace` | Verbose: request path details (never in production) |

### Standard field names

All structured log events use consistent field names:

| Field | Type | Description |
|-------|------|-------------|
| `request_id` | `String` | From `RequestContext` |
| `client_ip` | `String` | Resolved IP |
| `key_id` | `String` | API key identifier |
| `recipient_domain` | `String` | Domain part only (RFC 083) |
| `event` | `String` | Audit event name (RFC 081) |
| `error` | `String` | Error description (sanitized) |

### `#[instrument]` convention

- Use `#[instrument]` on handlers and major processing functions.
- Always `skip` the full request payload, body, and auth headers.
- Include `request_id` and `key_id` as fields when available.

```rust
#[instrument(
    skip(state, payload),
    fields(
        request_id = %ctx.request_id,
        key_id = %ctx.key_id,
    )
)]
async fn handle_send(
    State(state): State<AppState>,
    ctx: RequestContext,
    Json(payload): Json<MailRequest>,
) -> Result<Json<SendResponse>, AppError> {
    // ...
}
```

### Output target: stderr

`with_writer(std::io::stderr)` is used because:
- OpenBSD daemons conventionally log to stderr, piped through `logger(1)` to syslog.
- stderr does not require `rpath` or file-open syscalls after startup.
- After `pledge("stdio inet", ...)`, stderr remains available because it's already open.

### Startup and shutdown logs

Required events at info level:

```
http-smtp-rele starting version=0.1.0
config loaded path=/etc/http-smtp-rele.toml
listening on 127.0.0.1:8080
shutting down
```

## Implementation Plan

1. Create `src/logging.rs` with `pub fn init(level: &str, json: bool)`.
2. Call `logging::init` as the first statement in `main`.
3. Add `RUST_LOG` documentation to `README.md`.
4. Confirm `tracing` events appear on stderr in plain text.
5. Write unit/integration tests.

## Test Plan

### Unit Tests

- `logging::init` does not panic when called with valid level strings.
- `logging::init` does not panic when `RUST_LOG` is unset.

### Integration Tests

- Startup log messages appear on stderr.
- Unknown `RUST_LOG` level falls back to `info` (no panic).

### Operational Tests

- Starting the binary with `RUST_LOG=debug` produces more verbose output.
- Starting the binary with `RUST_LOG=off` suppresses all output.

## Security Considerations

- Logging is initialized before config and before any request handling. There is no window
  where errors go silently unlogged.
- The `tracing` subscriber is global; it must not be initialized more than once (use the
  `try_init` variant in tests to avoid panics).
- Log output goes to stderr, not to files opened at runtime, so `unveil` does not need to
  grant write access to any log file path.

## Operational Considerations

- To ship logs to syslog on OpenBSD: `daemon_flags` in rc.d can pipe stderr to `logger -t http-smtp-rele`.
- The JSON format (RFC 084) is designed for use with log aggregation tools.
- `RUST_LOG` takes precedence over the config file log level setting. Document this clearly.

## Documentation Changes

- Document `RUST_LOG` and the config log level in `docs/configuration.md`.
- Document the stderr output convention and syslog piping in `docs/openbsd.md`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-013-01 | `logging::init` is the first call in `main`. |
| AC-013-02 | Default log level is `info`. |
| AC-013-03 | Log output goes to stderr. |
| AC-013-04 | `RUST_LOG` env variable overrides the configured level. |
| AC-013-05 | Startup and shutdown log events are emitted at `info` level. |

## Open Questions

None.
