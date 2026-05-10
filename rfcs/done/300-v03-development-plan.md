# RFC 300 — v0.3 Development Plan

**Status.** Implemented  
**Tracks.** Governance  
**Touches.** All

## Summary

Define the scope for v0.3.

## Scope

| RFC | Feature | Priority |
|-----|---------|----------|
| 301 | SMTP AUTH (user/password for non-localhost relay) | High |
| 302 | Multi-recipient `to` (string or array) | High |
| 303 | W3C `Forwarded` header support | Medium |
| 304 | Sendmail pipe mode (implements deferred RFC 064) | Medium |
| 305 | `SIGHUP` signal-based config reload | Medium |

Deferred to v0.4: Prometheus `/metrics`, workspace split, HTML body, attachments.

## Implementation order

1. RFC 303 — W3C Forwarded (self-contained, touches auth.rs only)
2. RFC 301 — SMTP AUTH (config.rs + smtp.rs)
3. RFC 302 — Multi-recipient (config, validation, mail, tests)
4. RFC 304 — Sendmail pipe mode (smtp.rs, security.rs)
5. RFC 305 — SIGHUP reload (main.rs, AppState)

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-300-01 | `cargo test` passes with 0 failures. |
| AC-300-02 | `make gate` passes. |
| AC-300-03 | All v0.3 RFCs in `rfcs/done/`. |
