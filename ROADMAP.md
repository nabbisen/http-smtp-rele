# Roadmap

## v0.1.0 — MVP (current)

Core HTTP-to-SMTP relay with security hardening.

- `POST /v1/send` with bearer token auth
- Three-tier rate limiting (global / IP / key)
- Header injection prevention
- Recipient domain allowlist
- Fixed `From` address (config-controlled, never client-supplied)
- OpenBSD pledge/unveil hardening
- Structured JSON logging
- `GET /healthz` and `GET /readyz`

## v0.2.0 — Shipped (2026-05-10)

- **Sendmail pipe mode** (`smtp.mode = "pipe"`) — RFC 064
  - SMTP submit via `sendmail -t` instead of direct TCP
  - Requires additional OpenBSD pledge promises (`exec proc`)
- **W3C `Forwarded` header** support — alternative to `X-Forwarded-For`
- **SMTP AUTH** — for relays that require authentication
- **Per-request `Retry-After` probe cache TTL config** — RFC 063
- **IP bucket eviction** — LRU cap on the per-IP rate limit map
- **Signal-based config reload** — `SIGHUP` triggers config re-read

## v0.3.0 — Shipped (2026-05-10)

- HTML body support (opt-in)
- Multi-recipient `to` (array)
- Attachment support (base64-encoded, size-limited)
- Prometheus `/metrics` endpoint
- Workspace split (separate library and binary crates)

## v0.4.0 — Shipped (2026-05-10)

- Prometheus `/metrics` endpoint (request count, SMTP latency histograms)
- Cargo workspace split (separate `http-smtp-rele-core` library crate)
- HTML body support (`body_html` field, multipart/alternative)
- Attachment support (base64-encoded, size-limited)
- Multiple `cc` recipients (array, same validation pipeline as `to`)
- OpenBSD: SIGHUP reload with `rpath` pledge re-application

## v0.5 — Planned

- Cargo workspace split (`http-smtp-rele-core` library crate + `http-smtp-rele` binary)
- Attachment support (base64-encoded in JSON, MIME size-limited)
- Multiple `cc` via lettre's CC header builder (already done in v0.4)
- OpenBSD SIGHUP reload with `rpath` pledge re-application window
- Prometheus: auth failure and rate limit counters wired to middleware layers
- `reply_to` array support (multiple Reply-To addresses)
