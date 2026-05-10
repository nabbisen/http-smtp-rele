# RFC 085 — Failure Observability

**Status.** Implemented  
**Tracks.** Ops  
**Touches.** `src/smtp.rs`, `src/error.rs`, `src/api/handlers.rs`

## Summary

Ensure every failure mode produces a structured log event with enough context for operators
to diagnose the problem without inspecting request content.

## Motivation

Silent failures or underspecified error logs leave operators unable to distinguish
misconfiguration from attack from transient infrastructure failure (NFR-OPS-004).

## Design

### Failure → log event mapping

| Failure | Level | Mandatory fields |
|---------|-------|-----------------|
| Config load failure | ERROR | `event=config_error`, `error`, `path` |
| Auth failure | WARN | `event=auth_failure`, `reason`, `client_ip`, `request_id` |
| Rate limit | WARN | `event=rate_limited`, `tier`, `request_id` |
| Validation failure | WARN | `event=validation_failure`, `field`, `request_id`, `key_id` |
| Header injection | WARN | `event=header_injection_attempt`, `field`, `request_id` |
| Policy rejection | WARN | `event=policy_rejected`, `reason`, `request_id`, `key_id` |
| SMTP failure | ERROR | `event=smtp_failure`, `error`, `recipient_domain`, `request_id`, `key_id` |
| Internal error | ERROR | `event=internal_error`, `error`, `request_id` |

### What is always excluded

- Request body content.
- API key secrets.
- Full recipient address (domain only via RFC 083).
- Raw SMTP session transcript.

### Implementation pattern

Each failure site logs before mapping to `AppError`:

```rust
state.smtp.send(message).await.map_err(|smtp_err| {
    tracing::error!(
        event = "smtp_failure",
        request_id = %ctx.request_id,
        key_id = %ctx.key_id,
        recipient_domain = %log_recipient(&validated.to, mask),
        error = %smtp_err,
    );
    AppError::SmtpUnavailable
})?;
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-085-01 | Every failure mode produces a log event at the level in the table. |
| AC-085-02 | Every failure log includes `request_id`. |
| AC-085-03 | No failure log contains body or secret values. |

## Open Questions

None.
