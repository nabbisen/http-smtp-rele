# RFC 837 — HTTP Timeout and SMTP Timeout Coordination

**Status.** Proposed  
**Tracks.** T7 — Stability  
**Touches.** `src/config.rs`, `src/config/validate.rs`, docs

## Problem

The router applies a `TimeoutLayer` with `request_timeout_seconds`.
SMTP submission also has a connect/data timeout. If:

```
SMTP timeout > HTTP request timeout
```

then the HTTP layer times out the request and returns 408 while the SMTP
submission is still in flight. The submission may eventually succeed, but
the client received an error. Status store is left in a non-terminal state.

With bulk requests, validation + N sequential SMTP submissions can easily
exceed a request_timeout that was sized for single sends.

## Fix

### Config validation: enforce ordering

```rust
if cfg.server.request_timeout_seconds <= cfg.smtp.connection_timeout_seconds {
    return Err(ConfigError::Validation(
        "server.request_timeout_seconds must exceed smtp.connection_timeout_seconds".into()
    ));
}
```

Recommended margin: at least 2× the SMTP timeout.

### Document the relationship

Add to configuration reference:

```
request_timeout_seconds should be set to:
  (smtp.connection_timeout_seconds × bulk_concurrency_rounds) + validation_margin

For single send: request_timeout ≥ smtp_timeout × 2
For bulk N msgs: request_timeout ≥ ceil(N / bulk_concurrency) × smtp_timeout + 5s
```

### Future: separate bulk timeout

A `bulk_request_timeout_seconds` config field would allow a longer timeout
for bulk requests without loosening single-send latency guarantees.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-837-01 | Config validation rejects `request_timeout_seconds ≤ smtp.connection_timeout_seconds`. |
| AC-837-02 | Timeout relationship is documented in configuration reference. |
