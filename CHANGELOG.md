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

## [0.6.0] — 2026-05-10

### Theme: Submission Status Tracking

This release adds **Submission Status Tracking** — the ability to query what
`http-smtp-rele` observed during request handling and SMTP submission.
This is not asynchronous mail delivery: `POST /v1/send` remains synchronous.
The status store records metadata only; it never stores mail body, subject,
full recipient addresses, or credentials.

### Added

**`RequestId` newtype — breaking change (RFC 036/086)**
- `request_id` format changed from UUID v4 to `req_` + ULID (e.g. `req_01HX...`)
- Clients must treat `request_id` as an opaque string
- `X-Request-Id` response header now uses the new format
- `src/request_id.rs`: `Display`, `FromStr`, `Serialize`, `Deserialize`, `Clone`, `Eq`, `Hash`

**Submission Status API (RFC 036)**
- `GET /v1/submissions/{request_id}` — metadata-only status lookup
- Same API key required; other keys receive 404 (not 403) to prevent enumeration
- Returns: `status`, `code`, `recipient_domains`, `recipient_count`, timestamps
- 404 for: unknown, expired, different-key, or invalid format `request_id`

**StatusStore abstraction (RFC 086)**
- `StatusStore` trait: `put / update_status / get / expire_old_records / reload_config`
- `SubmissionStatus`: `received → smtp_submission_started → smtp_accepted / smtp_failed / rejected`
- Terminal states (`rejected`, `smtp_accepted`, `smtp_failed`) cannot be overwritten
- `ErrorCode` enum unified with HTTP error response codes
- `Domain` newtype — stores domain only, never full recipient address
- `recipient_domains: Vec<Domain>` — deduplicated and sorted

**In-memory StatusStore (RFC 087)**
- `InMemoryStatusStore` — `RwLock<HashMap>`, thread-safe
- Hybrid TTL cleanup: lazy expiry on `get()` + background task every `cleanup_interval_seconds`
- `max_records` eviction: expired first, then oldest by `created_at`
- `NoopStatusStore` — used when `enabled = false`
- `[status].reload_config()` — SIGHUP updates `ttl_seconds`, `max_records`, `cleanup_interval`
- On restart: all records cleared (documented behaviour)
- OpenBSD: no additional pledge promises required

**`[status]` configuration section (RFC 087)**
```toml
[status]
enabled                  = true
store                    = "memory"
ttl_seconds              = 3600
max_records              = 10000
cleanup_interval_seconds = 60
```
SIGHUP-reloadable: `ttl_seconds`, `max_records`, `cleanup_interval_seconds`
Restart required: `enabled`, `store`

**Status store write integration in `send_mail` (RFC 086/087)**
- Status records created after auth + rate limit pass (`key_id` is known)
- `received` → `smtp_submission_started` → `smtp_accepted` / `smtp_failed`
- Validation or rate limit failures: `rejected` with `ErrorCode`
- Pre-auth rejections have `X-Request-Id` header but no status record

**Persistent status store design (RFC 088, not implemented)**
- SQLite and Redis/Valkey candidate designs documented
- Single text/JSON file explicitly rejected as primary status store
- Deferred to v0.7+

### Changed

- `request_id` format: UUID v4 → `req_` + ULID (breaking)
- `AppState` gains `status_store: Arc<dyn StatusStore>` field
- `AppState::reload_config()` now also calls `status_store.reload_config()`
- Background status cleanup task spawned in `crates/cli/src/main.rs`
- Dependencies added: `ulid = "1"`, `chrono = "0.4"`, `async-trait = "0.1"`

---

## [0.5.0] — 2026-05-10

### Added

**Cargo workspace split (RFC 501)**
- Root package is now the pure library crate (`http-smtp-rele`)
- `crates/cli/` is the binary package (`http-smtp-rele-cli`) producing the `http-smtp-rele` binary
- Workspace shares a single `target/` directory (no redundant builds)
- Integration tests in `tests/` remain in the root library package — no imports changed
- `lib.rs` visible without `[[bin]]` coupling

**Attachment support (RFC 502)**
- `attachments: Option<Vec<AttachmentSpec>>` field in `POST /v1/send`
- Each attachment: `filename` (no path separators), `content_type` (MIME), `data` (base64)
- Base64 decoded and validated: size ≤ `mail.max_attachment_bytes` (default 10 MiB)
- Count checked against `mail.max_attachments` (default 5)
- MIME structure: `multipart/mixed` wrapping the body part when attachments present
- New dependency: `base64 = "0.22"`

**reply_to array (RFC 503)**
- `reply_to` now accepts a JSON string or array (consistent with `to`/`cc`)
- Validated by the same `Recipients` deserializer
- lettre: `.reply_to()` called for each validated address
- `ValidatedMailRequest.reply_to` changed from `Option<String>` to `Vec<String>`

**Prometheus full instrumentation (RFC 504)**
- Auth failures now increment `rele_auth_failures_total{reason}`:
  - `missing_token` — no Authorization header
  - `invalid_token` — token not matched
  - `disabled_key` — matched key but disabled
- Rate limit hits now increment `rele_rate_limited_total{tier}` by `global`/`ip`/`key`
- Validation failures now increment `rele_validation_failures_total{field}`
- All rejection paths also increment `rele_requests_total{status="4xx"}`

### Changed

- `MailConfig` gains `max_attachments` (default 5) and `max_attachment_bytes` (default 10 MiB)
- `ValidatedMailRequest.reply_to` type changed from `Option<String>` to `Vec<String>`
- `MailRequest.reply_to` type changed from `Option<String>` to `Option<Recipients>`

---

## [0.4.0] — 2026-05-10

### Added

**Prometheus /metrics endpoint (RFC 401)**
- `GET /metrics` returns Prometheus text exposition format (version 0.0.4)
- Metrics registered per-instance in a private `prometheus::Registry` on `AppState`
- `rele_requests_total{status}` — HTTP request count by 2xx/4xx/5xx class
- `rele_smtp_submissions_total{result}` — SMTP submissions by ok/error
- `rele_smtp_duration_seconds` — SMTP session duration histogram (1ms–8s buckets)
- `rele_auth_failures_total{reason}` — auth failures by reason
- `rele_rate_limited_total{tier}` — rate limit hits by tier (global/ip/key)
- `rele_validation_failures_total{field}` — validation failures by field name
- Access restriction guidance: restrict `/metrics` at the reverse proxy layer

**SMTP STARTTLS and TLS (RFC 402)**
- `[smtp].tls` field: `"none"` (default), `"starttls"` (port 587), `"tls"` (port 465)
- lettre STARTTLS and implicit TLS builders selected at startup based on config
- `rustls-tls` feature already present; no new dependencies required
- Invalid `tls` value causes fail-fast startup error

**HTML body — multipart/alternative (RFC 403)**
- `body_html: Option<String>` field in `POST /v1/send` request
- When both `body` and `body_html` are provided: `multipart/alternative` message
- When only `body` is provided: plain `text/plain` (unchanged behaviour)
- `body_html` subject to same `max_body_bytes` size limit as `body`
- NUL byte rejection applied to `body_html`

**cc recipients (RFC 404)**
- `cc: Optional<string | string[]>` field in `POST /v1/send` request
- Same `Recipients` deserializer as `to` (string or array)
- Each cc address: RFC 5321 format validation, CR/LF rejection, domain/address policy
- Combined `to + cc` count checked against `mail.max_recipients`
- lettre: `.cc(mailbox)` called for each validated cc address

### Changed

- `config::validate_config` is now public (enables external config validation)
- `AppState.metrics: Arc<Metrics>` added; all handler paths instrument SMTP timing

---

## [0.3.0] — 2026-05-10

### Added

**SMTP AUTH (RFC 301)**
- `[smtp].auth_user` and `[smtp].auth_password` (stored as `SecretString`) for
  non-localhost relay servers that require credentials
- lettre `Credentials` injected into the SMTP transport when both fields are set
- Config validation: both must be set together or both absent; incompatible with pipe mode

**Multi-recipient `to` (RFC 302)**
- `to` field now accepts a JSON string or array: `"alice@b.com"` or `["a@b.com","c@d.com"]`
- All recipients are validated and policy-checked independently
- `mail.max_recipients` config field (default 10) caps the array length
- `lettre::Message` gets a `.to()` call per recipient

**W3C `Forwarded` header (RFC 303)**
- `parse_forwarded_for()` in `auth.rs` parses RFC 7239 `Forwarded: for=<addr>`
- Preferred over `X-Forwarded-For` when both headers are present
- Handles IPv6 addresses in brackets; falls back gracefully on malformed values

**Sendmail pipe mode (RFC 304)**
- `smtp.mode = "pipe"` now works; `smtp.pipe_command` (default `/usr/sbin/sendmail`)
- `submit_pipe()` in `smtp.rs` spawns the command, pipes the formatted message to stdin,
  and maps non-zero exit codes to `502 smtp_unavailable`
- OpenBSD: pipe mode uses `pledge("stdio exec proc")` + `unveil(pipe_command, "x")`
  instead of `pledge("stdio inet")` used in SMTP relay mode
- `main.rs` selects `RuntimeMode` based on `smtp.mode` at startup

**SIGHUP config reload (RFC 305)**
- On `SIGHUP`, the config file is re-read and validated atomically
- `AppState.config` is now `arc_swap::ArcSwap<AppConfig>` — all handlers read via `.load()`
- Invalid config on SIGHUP is logged and the current config is kept
- Primarily useful on Linux; on OpenBSD, `pledge("stdio inet")` prevents file re-read

### Changed

- `AppState.config` type: `Arc<AppConfig>` → `arc_swap::ArcSwap<AppConfig>`
  (callers use `.load()` to get a snapshot `Arc<AppConfig>`)
- `security::RuntimeMode` extended with `SendmailPipe { pipe_command }` variant
- `MailRequest.to`: `String` → `Recipients` (custom deserializer accepting both forms)
- `ValidatedMailRequest.to`: `String` → `Vec<String>`

---

## [0.6.0] — 2026-05-10

### Theme: Submission Status Tracking

This release adds **Submission Status Tracking** — the ability to query what
`http-smtp-rele` observed during request handling and SMTP submission.
This is not asynchronous mail delivery: `POST /v1/send` remains synchronous.
The status store records metadata only; it never stores mail body, subject,
full recipient addresses, or credentials.

### Added

**`RequestId` newtype — breaking change (RFC 036/086)**
- `request_id` format changed from UUID v4 to `req_` + ULID (e.g. `req_01HX...`)
- Clients must treat `request_id` as an opaque string
- `X-Request-Id` response header now uses the new format
- `src/request_id.rs`: `Display`, `FromStr`, `Serialize`, `Deserialize`, `Clone`, `Eq`, `Hash`

**Submission Status API (RFC 036)**
- `GET /v1/submissions/{request_id}` — metadata-only status lookup
- Same API key required; other keys receive 404 (not 403) to prevent enumeration
- Returns: `status`, `code`, `recipient_domains`, `recipient_count`, timestamps
- 404 for: unknown, expired, different-key, or invalid format `request_id`

**StatusStore abstraction (RFC 086)**
- `StatusStore` trait: `put / update_status / get / expire_old_records / reload_config`
- `SubmissionStatus`: `received → smtp_submission_started → smtp_accepted / smtp_failed / rejected`
- Terminal states (`rejected`, `smtp_accepted`, `smtp_failed`) cannot be overwritten
- `ErrorCode` enum unified with HTTP error response codes
- `Domain` newtype — stores domain only, never full recipient address
- `recipient_domains: Vec<Domain>` — deduplicated and sorted

**In-memory StatusStore (RFC 087)**
- `InMemoryStatusStore` — `RwLock<HashMap>`, thread-safe
- Hybrid TTL cleanup: lazy expiry on `get()` + background task every `cleanup_interval_seconds`
- `max_records` eviction: expired first, then oldest by `created_at`
- `NoopStatusStore` — used when `enabled = false`
- `[status].reload_config()` — SIGHUP updates `ttl_seconds`, `max_records`, `cleanup_interval`
- On restart: all records cleared (documented behaviour)
- OpenBSD: no additional pledge promises required

**`[status]` configuration section (RFC 087)**
```toml
[status]
enabled                  = true
store                    = "memory"
ttl_seconds              = 3600
max_records              = 10000
cleanup_interval_seconds = 60
```
SIGHUP-reloadable: `ttl_seconds`, `max_records`, `cleanup_interval_seconds`
Restart required: `enabled`, `store`

**Status store write integration in `send_mail` (RFC 086/087)**
- Status records created after auth + rate limit pass (`key_id` is known)
- `received` → `smtp_submission_started` → `smtp_accepted` / `smtp_failed`
- Validation or rate limit failures: `rejected` with `ErrorCode`
- Pre-auth rejections have `X-Request-Id` header but no status record

**Persistent status store design (RFC 088, not implemented)**
- SQLite and Redis/Valkey candidate designs documented
- Single text/JSON file explicitly rejected as primary status store
- Deferred to v0.7+

### Changed

- `request_id` format: UUID v4 → `req_` + ULID (breaking)
- `AppState` gains `status_store: Arc<dyn StatusStore>` field
- `AppState::reload_config()` now also calls `status_store.reload_config()`
- Background status cleanup task spawned in `crates/cli/src/main.rs`
- Dependencies added: `ulid = "1"`, `chrono = "0.4"`, `async-trait = "0.1"`

---

## [0.5.0] — 2026-05-10

### Added

**Cargo workspace split (RFC 501)**
- Root package is now the pure library crate (`http-smtp-rele`)
- `crates/cli/` is the binary package (`http-smtp-rele-cli`) producing the `http-smtp-rele` binary
- Workspace shares a single `target/` directory (no redundant builds)
- Integration tests in `tests/` remain in the root library package — no imports changed
- `lib.rs` visible without `[[bin]]` coupling

**Attachment support (RFC 502)**
- `attachments: Option<Vec<AttachmentSpec>>` field in `POST /v1/send`
- Each attachment: `filename` (no path separators), `content_type` (MIME), `data` (base64)
- Base64 decoded and validated: size ≤ `mail.max_attachment_bytes` (default 10 MiB)
- Count checked against `mail.max_attachments` (default 5)
- MIME structure: `multipart/mixed` wrapping the body part when attachments present
- New dependency: `base64 = "0.22"`

**reply_to array (RFC 503)**
- `reply_to` now accepts a JSON string or array (consistent with `to`/`cc`)
- Validated by the same `Recipients` deserializer
- lettre: `.reply_to()` called for each validated address
- `ValidatedMailRequest.reply_to` changed from `Option<String>` to `Vec<String>`

**Prometheus full instrumentation (RFC 504)**
- Auth failures now increment `rele_auth_failures_total{reason}`:
  - `missing_token` — no Authorization header
  - `invalid_token` — token not matched
  - `disabled_key` — matched key but disabled
- Rate limit hits now increment `rele_rate_limited_total{tier}` by `global`/`ip`/`key`
- Validation failures now increment `rele_validation_failures_total{field}`
- All rejection paths also increment `rele_requests_total{status="4xx"}`

### Changed

- `MailConfig` gains `max_attachments` (default 5) and `max_attachment_bytes` (default 10 MiB)
- `ValidatedMailRequest.reply_to` type changed from `Option<String>` to `Vec<String>`
- `MailRequest.reply_to` type changed from `Option<String>` to `Option<Recipients>`

---

## [0.4.0] — 2026-05-10

### Added

**Prometheus /metrics endpoint (RFC 401)**
- `GET /metrics` returns Prometheus text exposition format (version 0.0.4)
- Metrics registered per-instance in a private `prometheus::Registry` on `AppState`
- `rele_requests_total{status}` — HTTP request count by 2xx/4xx/5xx class
- `rele_smtp_submissions_total{result}` — SMTP submissions by ok/error
- `rele_smtp_duration_seconds` — SMTP session duration histogram (1ms–8s buckets)
- `rele_auth_failures_total{reason}` — auth failures by reason
- `rele_rate_limited_total{tier}` — rate limit hits by tier (global/ip/key)
- `rele_validation_failures_total{field}` — validation failures by field name
- Access restriction guidance: restrict `/metrics` at the reverse proxy layer

**SMTP STARTTLS and TLS (RFC 402)**
- `[smtp].tls` field: `"none"` (default), `"starttls"` (port 587), `"tls"` (port 465)
- lettre STARTTLS and implicit TLS builders selected at startup based on config
- `rustls-tls` feature already present; no new dependencies required
- Invalid `tls` value causes fail-fast startup error

**HTML body — multipart/alternative (RFC 403)**
- `body_html: Option<String>` field in `POST /v1/send` request
- When both `body` and `body_html` are provided: `multipart/alternative` message
- When only `body` is provided: plain `text/plain` (unchanged behaviour)
- `body_html` subject to same `max_body_bytes` size limit as `body`
- NUL byte rejection applied to `body_html`

**cc recipients (RFC 404)**
- `cc: Optional<string | string[]>` field in `POST /v1/send` request
- Same `Recipients` deserializer as `to` (string or array)
- Each cc address: RFC 5321 format validation, CR/LF rejection, domain/address policy
- Combined `to + cc` count checked against `mail.max_recipients`
- lettre: `.cc(mailbox)` called for each validated cc address

### Changed

- `config::validate_config` is now public (enables external config validation)
- `AppState.metrics: Arc<Metrics>` added; all handler paths instrument SMTP timing

---

## [0.3.0] — 2026-05-10

### Added

**W3C Forwarded header (RFC 303)**
- `parse_forwarded_for()` in `auth.rs` — RFC 7239 `Forwarded: for=<addr>` header support
- Takes precedence over `X-Forwarded-For` when both headers are present
- Handles IPv4, IPv6 (bracket notation), and multi-param `Forwarded` values

**SMTP AUTH (RFC 301)**
- `[smtp].auth_user` and `[smtp].auth_password` (stored as `SecretString`)
- SMTP AUTH injected via `lettre::Credentials` when both fields are set
- `auth_password` never logged; startup validation requires both or neither
- Not applicable when `smtp.mode = "pipe"` (validated at startup)

**Multi-recipient `to` (RFC 302)**
- `to` field accepts a JSON string `"a@x.com"` or array `["a@x.com","b@x.com"]`
- `Recipients` serde deserializer handles both forms transparently
- All recipients validated independently: address format, CR/LF, domain policy
- `[mail].max_recipients` cap (default 10) prevents abuse
- `lettre::Message::builder().to()` called once per recipient

**Sendmail pipe mode (RFC 304)**
- `[smtp].mode = "pipe"` — submits mail via `sendmail -t` instead of direct TCP
- `[smtp].pipe_command` — configurable path (default `/usr/sbin/sendmail`)
- Subprocess stdin receives RFC 5322 formatted message; exit code checked
- Timeout enforced via `tokio::time::timeout`
- OpenBSD: pledge changes to `"stdio exec proc"` with `unveil(pipe_command, "x")`
- `mode = "smtp"` pledge unchanged: `"stdio inet"`

**SIGHUP config reload (RFC 305)**
- `kill -HUP <pid>` reloads config from the original path without restart
- `AppState.config_store: ArcSwap<AppConfig>` — atomic hot-swap via `arc-swap` crate
- `AppState::config()` — returns current `Arc<AppConfig>` snapshot per request
- `AppState::reload_config()` — atomically replaces the stored config
- Invalid config on SIGHUP: logged as error, current config unchanged
- In-flight requests are not interrupted during reload
- OpenBSD note: `pledge("stdio inet")` excludes `rpath` — SIGHUP reload is only
  supported on non-OpenBSD or when pledge is not active

### Changed

- `AppState.config` field replaced by `config()` and `reload_config()` methods
- `smtp.rs::build_transport` now injects SMTP AUTH credentials when configured

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
