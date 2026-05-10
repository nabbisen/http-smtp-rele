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

## v0.3 — Planned

- HTML body support (opt-in)
- Multi-recipient `to` (array)
- Attachment support (base64-encoded, size-limited)
- Prometheus `/metrics` endpoint
- Workspace split (separate library and binary crates)
- SMTP AUTH (for non-localhost relay)
- W3C `Forwarded` header support
- Signal-based config reload (`SIGHUP`)
- Sendmail pipe mode (`smtp.mode = "pipe"`, implements RFC 064)

