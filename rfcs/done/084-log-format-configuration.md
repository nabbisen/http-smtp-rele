# RFC 084 — Log Format Configuration

**Status.** Implemented  
**Tracks.** Ops  
**Touches.** `src/logging.rs`, `src/config.rs`

## Summary

Allow the log output format to be switched between human-readable compact text (default) and
newline-delimited JSON, controlled by `[logging].json = true/false`.

## Motivation

Human-readable logs are preferable for local development and OpenBSD syslog piping. JSON logs
are required for structured log aggregation (Elasticsearch, Loki, Splunk). A config flag allows
operators to choose without recompiling (NFR-OPS-004).

## Design

`logging::init(level: &str, json: bool)` already accounts for this (RFC 013):

```rust
if json {
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().json().with_current_span(true).with_writer(std::io::stderr))
        .init();
} else {
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().compact().with_writer(std::io::stderr))
        .init();
}
```

The JSON format uses `tracing-subscriber`'s built-in JSON formatter. Each event is a single
line of JSON on stderr.

### Log level from config

The `[logging].level` config field is converted to an `EnvFilter` string. `RUST_LOG` env var
takes precedence.

```rust
let level_str = std::env::var("RUST_LOG").unwrap_or_else(|_| config.logging.level.clone());
let filter = EnvFilter::new(level_str);
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-084-01 | `json = false` produces compact text log lines on stderr. |
| AC-084-02 | `json = true` produces newline-delimited JSON on stderr. |
| AC-084-03 | Both formats include `request_id` in audit events. |
| AC-084-04 | `RUST_LOG` overrides config-level log level. |

## Open Questions

None.
