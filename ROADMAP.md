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

## v0.5.0 — Shipped (2026-05-10)

- Cargo workspace split (`http-smtp-rele-core` library crate + `http-smtp-rele` binary)
- Attachment support (base64-encoded in JSON, MIME size-limited)
- Multiple `cc` via lettre's CC header builder (already done in v0.4)
- OpenBSD SIGHUP reload with `rpath` pledge re-application window
- Prometheus: auth failure and rate limit counters wired to middleware layers
- `reply_to` array support (multiple Reply-To addresses)

## v0.6.0 — Shipped (2026-05-10)

- OpenBSD SIGHUP reload with `rpath` pledge re-application window
- Bulk send endpoint (`POST /v1/send-bulk`) with per-message parallelism cap
- Webhook delivery: `POST /v1/send` webhook mode (HTTP instead of SMTP)
- Per-key logging mask policy (`mask_recipient` override per API key)
- Admin `GET /v1/keys` read-only key status endpoint

## v0.7.0 — Shipped (2026-05-10)

- RFC 088 implementation: SQLite persistent status store
- RFC 089 implementation: Prometheus metrics for status store
- OpenBSD SIGHUP reload with `rpath` pledge re-application window
- Admin `GET /v1/keys` read-only key status endpoint
- Per-key logging mask policy (`mask_recipient` override per API key)

## v0.8.0 — Shipped (2026-05-10)

- RFC 088 implementation: SQLite persistent status store
- OpenBSD SIGHUP reload with `rpath` pledge re-application window
- Bulk send endpoint (`POST /v1/send-bulk`) with parallelism cap
- Admin endpoint for listing all keys (admin API key concept)

## v0.9.0 — Shipped (2026-05-10)

- Redis/Valkey shared status store (RFC 088 design, distributed option)
- Bulk send endpoint (`POST /v1/send-bulk`) with per-message parallelism cap
- OpenBSD SIGHUP reload with `rpath` pledge re-application window
- SQLite schema version 2 (if additive changes needed)


## v1.0 — Planned

- Redis/Valkey shared status store
- Parallel SMTP submission for send-bulk (bounded concurrency)
- OpenBSD SIGHUP `rpath` re-application window

## v0.10.0 — Shipped (2026-05-10)

- Bulk SMTP parallelism (RFC 711)
- HTTP server TLS, optional --features tls (RFC 712)

## v0.11 — Planned

- OpenBSD SIGHUP `rpath` fix: keep rpath in runtime pledge, tight unveil
- Redis/Valkey shared status store (optional feature)

## v0.11.0 — Shipped (2026-05-10)

- OpenBSD SIGHUP rpath fix (RFC 721)
- Redis/Valkey shared status store, optional --features redis (RFC 722)

## v0.12.0 — Shipped (2026-05-10)

- mdbook documentation structure (RFC 731)
- Security checklist (RFC 732)

## v0.13.0 — Shipped (2026-05-10)

Maintenance: edition 2024, module reorganisation, dep cleanup, test splitting.

## v0.14.0 — Shipped (2026-05-10)

Deploy automation: OpenBSD rc.d, Linux systemd, deployment guide.

## v0.15.0 — Shipped (2026-05-11)

Hardening release: architect review remediation (42 RFCs, functional + non-functional).
Production core re-established; extended features default-off.
