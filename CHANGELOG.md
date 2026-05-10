# Changelog

All notable changes to this project will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased] — v0.2 (in progress)

### Added

**Test infrastructure (RFC 100–103)**
- `tests/smtp_stub.rs` — in-process SMTP stub: accepts full SMTP dialog, records messages, configurable failure injection
- `tests/common.rs` — integration test harness with `test_router()`, `RequestBuilder`, `TestResponse` helpers
- `tests/integration_tests.rs` — 23 integration tests including 9 SEC regression tests and 9 E2E scenarios with SMTP stub

**Security**
- `X-Request-Id` response header on all responses via `request_id_layer` middleware (RFC 035)
  - UUID generated once per request; consistent across response header and JSON body
- Global and per-IP rate limit checks wired into send handler (RFC 072, alongside per-key check)

**Rate limiting (RFC 201–203, 206)**
- Per-tier burst configuration: `global_burst`, `per_ip_burst`, `per_key_burst` in `[rate_limit]`
- Default per-key rate limit: `per_key_per_min` field in `[rate_limit]`
- Per-key burst override: `ApiKeyConfig.burst` field
- IP bucket LRU eviction: `ip_table_size` cap on the per-IP rate limit map
- Legacy `burst_size` field retained for backward compatibility with deprecation warning

**Tests added (total: 82)**
- SEC-001–003, 008–011, 013, 015 as integration tests (previously unit-only)
- E2E-001–009 with real SMTP stub (full HTTP → auth → validation → SMTP pipeline)
- 4 SMTP stub self-tests

### Changed
- `api/mod.rs`: `request_id_layer` middleware added to all routes
- `api/send.rs`: reads `request_id` from middleware header; wires global/IP rate limit checks

---

## [0.2.0] — 2026-05-10

### Added

**Test infrastructure (RFC 100–103)**
- `tests/smtp_stub.rs` — in-process SMTP stub server; handles EHLO/MAIL FROM/RCPT TO/DATA/QUIT,
  records messages, supports connection refusal and delivery rejection modes
- `tests/common.rs` — integration test harness: `test_router()`, `RequestBuilder`, `TestResponse`
- `tests/integration_tests.rs` — 29 integration tests: SEC-001–015, E2E-001–009, v0.2 feature tests
- `X-Request-Id` response header on every response via `request_id_layer` middleware (RFC 035 complete)

**Rate limit enhancements (RFC 201–203, 206)**
- Per-tier burst settings: `[rate_limit].global_burst`, `per_ip_burst`, `per_key_burst`
  (legacy `burst_size` still accepted with deprecation warning)
- `[rate_limit].per_key_per_min` — default per-key rate (was incorrectly inheriting `per_ip_per_min`)
- `ApiKeyConfig.burst` — per-key burst override (0 = inherit `per_key_burst`)
- LRU eviction on the per-IP rate limit map (`[rate_limit].ip_table_size`, default 10 000)

**Per-address recipient allowlist (RFC 204)**
- `ApiKeyConfig.allowed_recipients: Vec<String>` — restrict a key to specific email addresses
- Takes precedence over `allowed_recipient_domains` when non-empty
- Case-insensitive full-address matching

**Server concurrency limit (RFC 205)**
- `server.concurrency_limit` — optional cap via `tower::limit::ConcurrencyLimitLayer`
- 0 = unlimited (default; behaviour unchanged from v0.1)

### Changed

- `rate_limit.burst_size` deprecated in favour of per-tier fields (still parsed; emits warning)
- `RateLimiter::check_key` signature extended with `burst_override: u32` parameter

---

## [0.1.0] — 2026-05-10

Initial release.

### Added

**Core relay**
- `POST /v1/send` — accept JSON mail request, validate, relay to local SMTP server
- `GET /healthz` — liveness probe (SMTP-independent)
- `GET /readyz` — readiness probe (TCP probe to configured SMTP host:port)
- `--config <path>` CLI flag; default path `/etc/http-smtp-rele.toml`

**Authentication**
- Bearer token and `X-API-Key` header support
- Constant-time comparison with `subtle::ConstantTimeEq` (RFC 043)
- Full-iteration auth loop — no early exit to prevent timing-based key enumeration
- Per-key `enabled` flag for zero-downtime key revocation

**Security**
- Header injection prevention: CR/LF in `to`, `subject`, `from_name`, `reply_to` → 400 (RFC 051)
- `deny_unknown_fields` on request DTO: `from`, `bcc`, `headers` fields → 422 (RFC 031)
- Fixed `From` address from config — clients cannot override (RFC 060)
- Recipient domain allowlist (`mail.allowed_recipient_domains`); global and per-key (RFC 023)
- Source IP allowlist (`security.allowed_source_cidrs`) and proxy header trust (`security.trusted_source_cidrs`) as separate controls (RFC 024, RFC 041)
- `SecretString` with redacted `Debug`/`Display` — secrets never appear in logs (RFC 022)
- `ValidatedMailRequest` proof type — SMTP unreachable without passing validation (RFC 050)

**Rate limiting**
- Three-tier in-memory token bucket: global → per-IP → per-key (RFC 070, RFC 071)
- `Retry-After` header in 429 responses (RFC 073)
- Per-key rate limit override via `ApiKeyConfig.rate_limit_per_min`

**Logging**
- Structured `tracing` output; text and JSON formats (`logging.format`)
- Recipient masking in logs (`logging.mask_recipient`)
- Audit events: `startup`, `auth_failure`, `rate_limited`, `validation_failure`, `header_injection_attempt`, `smtp_submitted`, `smtp_failure`

**OpenBSD hardening**
- `pledge("stdio inet")` after config load (RFC 091)
- `unveil(NULL, NULL)` before pledge — no filesystem access at runtime (RFC 091)
- `_http_smtp_rele` system user; rc.d script in `examples/` (RFC 092)

**Configuration**
- TOML schema with fail-fast validation on startup (RFC 020, RFC 021)
- Separate `trusted_source_cidrs` (proxy header trust) and `allowed_source_cidrs` (IP allowlist)
- `smtp.mode = "smtp"` only; `"pipe"` reserved for a future release (RFC 064)

**Project**
- 63 RFCs covering M0–M12 milestones; 59 moved to `rfcs/done/` at release
- `scripts/check-rfcs.sh` — structural integrity check for the RFC directory
- 55 unit and integration tests including SEC-001–011 security regression coverage
- `docs/` — getting-started, API reference, configuration, security, OpenBSD deployment, architecture, FAQ
- `examples/` — annotated TOML config, curl script, OpenBSD rc.d script

### Known limitations

- Rate limit state is in-memory; resets on restart (documented)
- `smtp.host` must be an IP address on OpenBSD (no DNS after `pledge`)
- Full integration test harness (fake SMTP, E2E) deferred to v0.2 (RFC 100–103)
- Sendmail pipe mode deferred to v0.2 (RFC 064)
- Per-address recipient allowlist deferred to v0.2
