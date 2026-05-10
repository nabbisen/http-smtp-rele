# RFC 081 — Audit Event Taxonomy

**Status.** Implemented  
**Tracks.** Ops / Security  
**Touches.** `src/logging.rs`, all handler and middleware code

## Summary

Define the canonical set of audit events, their log levels, and the structured fields each
must carry, so that security monitoring tooling can reliably detect and alert on relevant events.

## Motivation

Without a defined event taxonomy, monitoring rules must match on ad-hoc log strings that change
between versions. Named, structured events with consistent fields enable reliable alerting
(NFR-SEC-007, NFR-OPS-004).

## Scope

- Canonical event names (`event=` field value).
- Log level for each event.
- Required fields for each event.
- Fields that must NOT appear in each event.

## Non-goals

- JSON log format (RFC 084).
- Log redaction implementation (RFC 082).

## Design

### Event table

| `event` | Level | Required fields | Excluded fields |
|---------|-------|-----------------|-----------------|
| `startup` | INFO | `version` | — |
| `shutdown` | INFO | — | — |
| `config_loaded` | DEBUG | `path` | `api_keys.*` |
| `request_received` | DEBUG | `request_id`, `client_ip`, `method`, `path` | `body`, `authorization` |
| `auth_failure` | WARN | `request_id`, `client_ip`, `reason` | token, secret |
| `rate_limited` | WARN | `request_id`, `client_ip`, `tier` | secret |
| `validation_failure` | WARN | `request_id`, `key_id`, `field` | field value |
| `header_injection_attempt` | WARN | `request_id`, `key_id`, `field` | field value |
| `policy_rejected` | WARN | `request_id`, `key_id`, `reason` | recipient address |
| `smtp_submitted` | INFO | `request_id`, `key_id`, `recipient_domain` | body, recipient address |
| `smtp_failure` | ERROR | `request_id`, `key_id`, `error`, `recipient_domain` | body |
| `internal_error` | ERROR | `request_id`, `error` | stack trace |

### Field definitions

| Field | Type | Description |
|-------|------|-------------|
| `request_id` | String | UUIDv4 from RequestContext |
| `client_ip` | String | Resolved IP (RFC 041) |
| `key_id` | String | API key identifier |
| `method` | String | HTTP method |
| `path` | String | URL path (no query string) |
| `field` | String | Validation field name |
| `tier` | String | Rate limit tier: global/ip/key |
| `reason` | String | Auth failure reason: no_credentials/invalid_token/disabled_key |
| `error` | String | Sanitized error description |
| `recipient_domain` | String | Domain portion of recipient (RFC 083) |
| `version` | String | Binary version string |

### Emitting events

```rust
// Startup
tracing::info!(event = "startup", version = env!("CARGO_PKG_VERSION"));

// Auth failure
tracing::warn!(
    event = "auth_failure",
    request_id = %ctx.request_id,
    client_ip = %ctx.client_ip,
    reason = "invalid_token",
);

// SMTP submitted
tracing::info!(
    event = "smtp_submitted",
    request_id = %ctx.request_id,
    key_id = %ctx.key_id,
    recipient_domain = %domain_of(&validated.to),
);
```

## Test Plan

### Integration Tests

- Auth failure produces a `warn` log with `event=auth_failure` and `reason` field.
- Successful submission produces `event=smtp_submitted` with `recipient_domain`.
- SMTP failure produces `event=smtp_failure` with `error` field.
- No event contains `body` or token/secret values.

## Security Considerations

- The `reason` field in `auth_failure` must use fixed strings (`no_credentials`, `invalid_token`,
  `disabled_key`), not the raw error message, which might include internal detail.
- The `field` in `validation_failure` and `header_injection_attempt` logs only the field name,
  not the value (which could contain injected content or sensitive data).

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-081-01 | All events in the taxonomy are emitted in the relevant code paths. |
| AC-081-02 | Each event carries the required fields. |
| AC-081-03 | No event carries excluded fields. |
| AC-081-04 | `event=` field is present on all audit events. |

## Open Questions

None.
