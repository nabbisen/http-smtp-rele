# RFC 400 — v0.4 Development Plan

**Status.** Implemented  
**Tracks.** Governance

## Theme: Observability and Content Richness

v0.4 adds two axes of value:

**Observability** — operators can monitor the relay with standard tooling:
- RFC 401: Prometheus `/metrics` endpoint

**Content richness** — clients can send richer mail:
- RFC 402: SMTP STARTTLS (TLS for non-localhost relay)
- RFC 403: HTML body (`body_html` field, multipart/alternative)
- RFC 404: `cc` recipients (array, same pipeline as `to`)
- RFC 405: `reply_to` array (multiple Reply-To addresses)

## Implementation order

1. RFC 402 — STARTTLS (config.rs, smtp.rs only; no new deps)
2. RFC 404 — cc recipients (validation.rs, mail.rs; builds on RFC 302 patterns)
3. RFC 403 — HTML body (mail.rs; lettre MultiPart)
4. RFC 401 — Prometheus (new dep, new module, new route)

## Deferred to v0.5

- Cargo workspace split (library crate + binary crate)
- Attachment support (base64 decode, MIME size limits)
- OpenBSD SIGHUP with rpath pledge re-application

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-400-01 | `GET /metrics` returns Prometheus text format. |
| AC-400-02 | TLS/STARTTLS established when configured. |
| AC-400-03 | `cc` and `body_html` fields pass validation and appear in sent mail. |
| AC-400-04 | `cargo test` passes with 0 failures. |
