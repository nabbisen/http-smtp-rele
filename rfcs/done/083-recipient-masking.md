# RFC 083 — Recipient Masking Policy

**Status.** Implemented  
**Tracks.** Ops / Security  
**Touches.** `src/logging.rs`, `src/config.rs`

## Summary

When `mask_recipient_in_logs = true` (the default), log only the domain portion of recipient
email addresses; never the full address.

## Motivation

Full recipient email addresses in logs constitute personal data in many regulatory contexts.
Logging only the domain portion retains operational utility while avoiding unnecessary PII
storage (NFR-OPS-004, `[mail].mask_recipient_in_logs`).

## Scope

- `log_recipient(address, mask) -> String` helper.
- Default: `mask_recipient_in_logs = true`.
- Startup warning when masking is disabled.
- All audit events that reference the recipient use this helper.

## Non-goals

- Subject masking (subject is logged only as a field name, never value).
- Body masking (handled by RFC 082 via `skip(payload)`).

## Design

```rust
/// Returns the recipient field value for logging.
/// When masking is enabled, returns only the domain portion.
pub fn log_recipient(address: &str, mask: bool) -> Cow<str> {
    if mask {
        let domain = address.rfind('@').map(|i| &address[i+1..]).unwrap_or("unknown");
        Cow::Borrowed(domain)
    } else {
        Cow::Borrowed(address)
    }
}
```

Usage in audit events:

```rust
let r = log_recipient(&validated.to, state.config.mail.mask_recipient_in_logs);
tracing::info!(event = "smtp_submitted", recipient_domain = %r, ...);
```

### Startup warning

```rust
if !config.mail.mask_recipient_in_logs {
    tracing::warn!("mask_recipient_in_logs is false; full recipient addresses will appear in logs");
}
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-083-01 | Default config logs recipient domain only. |
| AC-083-02 | Full address logged only when `mask_recipient_in_logs = false`. |
| AC-083-03 | `mask_recipient_in_logs = false` emits a startup warning. |

## Open Questions

None.
