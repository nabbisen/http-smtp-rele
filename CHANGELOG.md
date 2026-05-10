# Changelog

All notable changes to this project will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

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
