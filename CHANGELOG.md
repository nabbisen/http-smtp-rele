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

## [0.12.0] — 2026-05-10

### Theme: Documentation and Security Checklist

No code changes. This release is documentation-only.

### Added

**mdbook documentation structure (RFC 731)**
- `docs/book.toml` — mdbook configuration
- `docs/src/SUMMARY.md` — persona-based navigation (13 chapters)
- `docs/src/introduction.md` — project overview and feature table
- `docs/src/guides/status-tracking.md` — end-to-end status tracking guide
- `docs/src/guides/bulk-sending.md` — bulk send guide with curl examples
- `docs/src/operations/reverse-proxy.md` — nginx / Caddy / relayd / HAProxy configs
- `docs/src/development/contributing.md` — RFC process, code style, test expectations
- Existing docs (`api.md`, `configuration.md`, `openbsd.md`, `security.md`,
  `architecture.md`, `testing.md`, `getting-started.md`, `faq.md`)
  copied into `docs/src/` hierarchy

**Security checklist (RFC 732)**
- `docs/src/operations/security-checklist.md` — 9-category pre-deployment checklist
  - Authentication and API keys (5 items)
  - Recipient and domain policy (3 items)
  - Rate limiting (3 items)
  - Network exposure (3 items)
  - TLS and transport security (2 items)
  - Logging and privacy (3 items)
  - OpenBSD hardening (4 items, platform-conditional)
  - Monitoring and alerting (3 items)
  - Operations and incident response (3 items)
  - Sign-off table

### Documentation structure

```
docs/
  book.toml
  src/
    SUMMARY.md           ← 13 chapters, all links verified
    introduction.md
    getting-started.md
    faq.md
    guides/
      api-reference.md
      configuration.md
      status-tracking.md   ← new
      bulk-sending.md      ← new
    operations/
      security-checklist.md ← new (RFC 732)
      openbsd.md
      reverse-proxy.md      ← new
    development/
      architecture.md
      testing.md
      contributing.md       ← new
```

---

## [0.11.0] — 2026-05-10

### Theme: Hardening Correctness and Shared State

### Added

**OpenBSD SIGHUP `rpath` fix (RFC 721)**
- `rpath` is now always kept in the runtime pledge
- SIGHUP config reload works correctly on OpenBSD
- Security rationale: `unveil` already restricts readable paths to the config
  file only; keeping `rpath` does not expand the effective read surface
- Pledge sets updated: `stdio inet rpath` (smtp/memory), `stdio inet rpath wpath cpath` (sqlite)

**Redis/Valkey shared status store (RFC 722, `--features redis`)**
- `[status] store = "redis"` with `redis_url = "redis://host:port/db"`
- Multi-instance deployments share a single status view
- Key schema: `rele:s:{request_id}` → JSON, TTL via Redis native EXPIRE
- Degraded mode: Redis unavailability logs WARN, does not fail mail delivery
- `max_records` not enforced (use Redis `maxmemory-policy allkeys-lru`)
- `expire_old_records()` is a no-op (Redis handles TTL natively)
- Status lookup during Redis outage returns 404
- Unit tests: serialisation round-trip, key prefix format
- Integration tests: require `REDIS_TEST_URL` env; skipped when unset
- `redis = "0.25"` optional dependency; `[features] redis = ["dep:redis"]`

### Changed

- OpenBSD runtime pledge always includes `rpath` (was dropped in v0.10)
- `[status]` config now accepts `store = "redis"` with `redis_url`
- Store validation: `status.store` must be `memory`, `sqlite`, or `redis`
- Status store error message updated to mention all valid store options

### Docs

- `docs/configuration.md`: `store = "redis"` section added
- `docs/openbsd.md`: SIGHUP reload section added (rpath rationale)

### Test coverage

| Build | unit | integration | total |
|-------|------|-------------|-------|
| default | 81 | 89 | 170 |
| `--features redis` | 88 | — | 88 unit |

---

## [0.10.0] — 2026-05-10

### Theme: Performance and Transport Security

### Added

**Bounded-parallel SMTP submission for send-bulk (RFC 711)**
- Two-phase processing: sequential rate-limit/validate/build → parallel SMTP submit
- `[smtp].bulk_concurrency = 5` (default, 0 = unlimited)
- `tokio::task::JoinSet` + `tokio::sync::Semaphore` for bounded concurrency
- Response `results` always sorted by request index regardless of completion order
- All existing bulk tests pass unchanged; 2 new parallelism tests added

**HTTP server TLS / HTTPS (RFC 712, `--features tls`)**
- `[server].tls_cert` + `[server].tls_key` — PEM certificate and key paths
- `axum-server = "0.7"` with `tls-rustls` (pure-Rust TLS, no OpenSSL)
- Feature flag: `cargo build --release --features tls`
- OpenBSD: `unveil(tls_cert, "r")` + `unveil(tls_key, "r")` before runtime pledge
- Certs loaded into memory before pledge; no additional pledge promises required
- Config validation: both fields required together or neither
- Non-TLS build with `tls_cert` configured → startup error with build instruction
- 4 TLS config validation tests added

**`[rate_limit]` and `[logging]` sections now optional in config**
- Added `#[serde(default)]` + `Default` impls for `RateLimitConfig` and `LoggingConfig`
- Allows minimal TOMLs without explicitly spelling out default-only sections
- Backward-compatible (existing full configs continue to work)

### Changed

- `[smtp].bulk_concurrency` field added (default: 5)
- `[server].tls_cert`, `[server].tls_key` fields added (default: None)
- `axum-server = "0.7"` optional dependency (feature: `tls`)
- `[features] tls = ["dep:axum-server"]` added to Cargo.toml

### Test coverage

| Build | unit | integration | total |
|-------|------|-------------|-------|
| default | 81 | 85 | 166 |
| `--features tls` | 81 | 84 | 165 |

---

## [0.9.0] — 2026-05-10

### Theme: Bulk Submission

### Added

**`POST /v1/send-bulk` (RFC 701)**
- Accepts array of independent mail messages (same schema as `POST /v1/send` per element)
- Per-message pipeline: rate limit → validate → build → SMTP submit → status update
- 202 Accepted with per-message `results` array regardless of individual outcomes
- Partial success is supported: one failed message does not abort others
- Each message gets its own `request_id` and status record
- `bulk_request_id` in response for log correlation
- `GET /v1/submissions/{request_id}` works per-message

**Rate limiting per message (RFC 702)**
- Global, IP, and per-key limits decremented once per message
- Exhaustion mid-array: earlier messages unaffected, remainder rejected with `rate_limited`

**`[mail].max_bulk_messages`** (default: `10`) — cap on messages per bulk request

**`src/api/send_bulk.rs`** — new handler module

### Docs

- `docs/api.md`: `POST /v1/send-bulk` endpoint reference
- `docs/configuration.md`: `max_bulk_messages` field documented

### Test coverage

| Build | unit | integration | total |
|-------|------|-------------|-------|
| default | 81 | 79 | 160 |

---

## [0.8.0] — 2026-05-10

### Theme: SQLite Persistent Status Store

**SQLite status store (RFC 088, `--features sqlite`)**

Provides durable submission status records that survive application restarts.
Memory store remains the default. SQLite is an optional Cargo feature.

Status records are metadata-only in both stores: mail body, subject,
attachments, full recipient addresses, API keys, and SMTP credentials
are never stored.

#### Build

```sh
cargo build --features sqlite
cargo build --release --features sqlite
```

#### Configuration

```toml
[status]
store   = "sqlite"
db_path = "/var/db/http-smtp-rele/status.db"
```

The parent directory must exist before startup:
```sh
install -d -o _http_smtp_rele -m 750 /var/db/http-smtp-rele
```

#### Implementation

- `src/status_sqlite.rs` — `SqliteStatusStore` implementing `StatusStore` trait
- `migrations/001_initial.sql` — schema embedded via `include_str!`
- Single `Mutex<Connection>` with WAL mode
- Schema migration via `PRAGMA user_version` (version 1)
- Breaking migration clears all records and logs a `WARN` event
- Downgrade (newer DB than binary) → startup error
- `max_records` enforced at `put()` and `expire_old_records()`
- Lazy TTL expiry on `get()` + periodic background sweep

#### Security / OpenBSD

`store = "sqlite"` adds `rpath wpath cpath` to pledge and
`unveil(db_path, "rwc")`.  These are applied automatically before the main
pledge call. Use `store = "memory"` for maximum pledge hardening.

#### Non-SQLite builds

`store = "sqlite"` in config with a non-SQLite binary produces a clear
startup error:

```
fatal: status.store = "sqlite" is not available in this build.
       Rebuild with: cargo build --features sqlite
```

#### Dependency

`rusqlite = { version = "0.32", features = ["bundled"], optional = true }`

### Changed

- `security::apply_runtime_restrictions(mode, has_sqlite: bool)` — new signature
- `security::apply_sqlite_restrictions(db_path)` — new public function
- `StatusConfig` gains `db_path: Option<PathBuf>`
- `config::validate_config` checks `store`/`db_path` consistency

### Docs

- `docs/configuration.md` — `[status]` section with SQLite setup guide
- `docs/openbsd.md` — SQLite pledge/unveil additions
- `examples/http-smtp-rele.toml` — `[status]` section with commented `db_path`

---

## [0.12.0] — 2026-05-10

### Theme: Documentation and Security Checklist

No code changes. This release is documentation-only.

### Added

**mdbook documentation structure (RFC 731)**
- `docs/book.toml` — mdbook configuration
- `docs/src/SUMMARY.md` — persona-based navigation (13 chapters)
- `docs/src/introduction.md` — project overview and feature table
- `docs/src/guides/status-tracking.md` — end-to-end status tracking guide
- `docs/src/guides/bulk-sending.md` — bulk send guide with curl examples
- `docs/src/operations/reverse-proxy.md` — nginx / Caddy / relayd / HAProxy configs
- `docs/src/development/contributing.md` — RFC process, code style, test expectations
- Existing docs (`api.md`, `configuration.md`, `openbsd.md`, `security.md`,
  `architecture.md`, `testing.md`, `getting-started.md`, `faq.md`)
  copied into `docs/src/` hierarchy

**Security checklist (RFC 732)**
- `docs/src/operations/security-checklist.md` — 9-category pre-deployment checklist
  - Authentication and API keys (5 items)
  - Recipient and domain policy (3 items)
  - Rate limiting (3 items)
  - Network exposure (3 items)
  - TLS and transport security (2 items)
  - Logging and privacy (3 items)
  - OpenBSD hardening (4 items, platform-conditional)
  - Monitoring and alerting (3 items)
  - Operations and incident response (3 items)
  - Sign-off table

### Documentation structure

```
docs/
  book.toml
  src/
    SUMMARY.md           ← 13 chapters, all links verified
    introduction.md
    getting-started.md
    faq.md
    guides/
      api-reference.md
      configuration.md
      status-tracking.md   ← new
      bulk-sending.md      ← new
    operations/
      security-checklist.md ← new (RFC 732)
      openbsd.md
      reverse-proxy.md      ← new
    development/
      architecture.md
      testing.md
      contributing.md       ← new
```

---

## [0.11.0] — 2026-05-10

### Theme: Hardening Correctness and Shared State

### Added

**OpenBSD SIGHUP `rpath` fix (RFC 721)**
- `rpath` is now always kept in the runtime pledge
- SIGHUP config reload works correctly on OpenBSD
- Security rationale: `unveil` already restricts readable paths to the config
  file only; keeping `rpath` does not expand the effective read surface
- Pledge sets updated: `stdio inet rpath` (smtp/memory), `stdio inet rpath wpath cpath` (sqlite)

**Redis/Valkey shared status store (RFC 722, `--features redis`)**
- `[status] store = "redis"` with `redis_url = "redis://host:port/db"`
- Multi-instance deployments share a single status view
- Key schema: `rele:s:{request_id}` → JSON, TTL via Redis native EXPIRE
- Degraded mode: Redis unavailability logs WARN, does not fail mail delivery
- `max_records` not enforced (use Redis `maxmemory-policy allkeys-lru`)
- `expire_old_records()` is a no-op (Redis handles TTL natively)
- Status lookup during Redis outage returns 404
- Unit tests: serialisation round-trip, key prefix format
- Integration tests: require `REDIS_TEST_URL` env; skipped when unset
- `redis = "0.25"` optional dependency; `[features] redis = ["dep:redis"]`

### Changed

- OpenBSD runtime pledge always includes `rpath` (was dropped in v0.10)
- `[status]` config now accepts `store = "redis"` with `redis_url`
- Store validation: `status.store` must be `memory`, `sqlite`, or `redis`
- Status store error message updated to mention all valid store options

### Docs

- `docs/configuration.md`: `store = "redis"` section added
- `docs/openbsd.md`: SIGHUP reload section added (rpath rationale)

### Test coverage

| Build | unit | integration | total |
|-------|------|-------------|-------|
| default | 81 | 89 | 170 |
| `--features redis` | 88 | — | 88 unit |

---

## [0.10.0] — 2026-05-10

### Theme: Performance and Transport Security

### Added

**Bounded-parallel SMTP submission for send-bulk (RFC 711)**
- Two-phase processing: sequential rate-limit/validate/build → parallel SMTP submit
- `[smtp].bulk_concurrency = 5` (default, 0 = unlimited)
- `tokio::task::JoinSet` + `tokio::sync::Semaphore` for bounded concurrency
- Response `results` always sorted by request index regardless of completion order
- All existing bulk tests pass unchanged; 2 new parallelism tests added

**HTTP server TLS / HTTPS (RFC 712, `--features tls`)**
- `[server].tls_cert` + `[server].tls_key` — PEM certificate and key paths
- `axum-server = "0.7"` with `tls-rustls` (pure-Rust TLS, no OpenSSL)
- Feature flag: `cargo build --release --features tls`
- OpenBSD: `unveil(tls_cert, "r")` + `unveil(tls_key, "r")` before runtime pledge
- Certs loaded into memory before pledge; no additional pledge promises required
- Config validation: both fields required together or neither
- Non-TLS build with `tls_cert` configured → startup error with build instruction
- 4 TLS config validation tests added

**`[rate_limit]` and `[logging]` sections now optional in config**
- Added `#[serde(default)]` + `Default` impls for `RateLimitConfig` and `LoggingConfig`
- Allows minimal TOMLs without explicitly spelling out default-only sections
- Backward-compatible (existing full configs continue to work)

### Changed

- `[smtp].bulk_concurrency` field added (default: 5)
- `[server].tls_cert`, `[server].tls_key` fields added (default: None)
- `axum-server = "0.7"` optional dependency (feature: `tls`)
- `[features] tls = ["dep:axum-server"]` added to Cargo.toml

### Test coverage

| Build | unit | integration | total |
|-------|------|-------------|-------|
| default | 81 | 85 | 166 |
| `--features tls` | 81 | 84 | 165 |

---

## [0.9.0] — 2026-05-10

### Theme: Bulk Submission

### Added

**`POST /v1/send-bulk` (RFC 701)**
- Accepts array of independent mail messages (same schema as `POST /v1/send` per element)
- Per-message pipeline: rate limit → validate → build → SMTP submit → status update
- 202 Accepted with per-message `results` array regardless of individual outcomes
- Partial success is supported: one failed message does not abort others
- Each message gets its own `request_id` and status record
- `bulk_request_id` in response for log correlation
- `GET /v1/submissions/{request_id}` works per-message

**Rate limiting per message (RFC 702)**
- Global, IP, and per-key limits decremented once per message
- Exhaustion mid-array: earlier messages unaffected, remainder rejected with `rate_limited`

**`[mail].max_bulk_messages`** (default: `10`) — cap on messages per bulk request

**`src/api/send_bulk.rs`** — new handler module

### Docs

- `docs/api.md`: `POST /v1/send-bulk` endpoint reference
- `docs/configuration.md`: `max_bulk_messages` field documented

### Test coverage

| Build | unit | integration | total |
|-------|------|-------------|-------|
| default | 81 | 79 | 160 |

---

## [0.8.0] — 2026-05-10

### Theme: SQLite Persistent Status Store

Implements RFC 088 (SQLite backend) with an optional Cargo feature.
The default build remains unchanged; SQLite is opt-in.

### Added

**`--features sqlite` — optional SQLite status store (RFC 088)**
- `[status] store = "sqlite"` — persistent status records that survive restarts
- `[status] db_path = "/var/db/http-smtp-rele/status.db"` — required with sqlite
- `src/status_sqlite.rs`: `SqliteStatusStore` implementing `StatusStore` trait
- `migrations/001_initial.sql`: initial schema, embedded via `include_str!`
- WAL mode (`PRAGMA journal_mode=WAL`) for concurrent read performance
- Schema migration via `PRAGMA user_version` (runs in a transaction)
- Breaking schema changes clear all records and log `WARN` (data is TTL-bounded)
- Downgrade to older binary with newer DB schema: startup error

**Build commands:**
```sh
cargo build --release                    # default: memory store only
cargo build --release --features sqlite  # includes SQLite backend
```

**`AppState::new_with_store`** — constructor accepting pre-built `StatusStore`
for tests and shared-store scenarios.

**`config::load_from_str`** — parse and validate config from a TOML string
(used for feature availability checks in tests).

### Changed

- `security::apply_runtime_restrictions` gains `has_sqlite: bool` parameter
- `security::apply_sqlite_restrictions(db_path)` added (OpenBSD: unveil)
- OpenBSD pledge for SQLite build: adds `rpath wpath cpath`
- `[status].store` now validated: must be `"memory"` or `"sqlite"`
- `[status].db_path` validated: required when `store = "sqlite"`
- Non-SQLite build with `store = "sqlite"` → clear startup error message
- Dependencies: `rusqlite = "0.32"` (optional, bundled SQLite C library)
- Dev-dependencies: `tempfile = "3"`

### Docs

- `docs/getting-started.md`: SQLite build instructions
- `docs/configuration.md`: `[status]` SQLite section (already present; refined)
- `docs/openbsd.md`: pledge surface comparison for memory vs sqlite store

### Test coverage

| Build | unit | integration | total |
|-------|------|-------------|-------|
| default (no sqlite) | 81 | 69 | 150 |
| `--features sqlite` | 92 | 76 | 168 |

---

## [0.7.0] — 2026-05-10

### Theme: Observability and Operations

**Status store Prometheus metrics (RFC 601, implements RFC 089)**
- `rele_status_store_records_current` (Gauge) — live record count
- `rele_status_store_transitions_total{status, code}` (Counter) — per-transition counts
- `rele_status_store_expired_total` (Counter) — TTL expiry events
- `Arc<Metrics>` passed to `InMemoryStatusStore` at construction
- Label cardinality bounded: no `request_id`, `key_id`, `client_ip`, or `recipient_domain` labels

**Authenticated key info endpoint (RFC 602)**
- `GET /v1/keys/self` — returns non-secret config of the authenticated API key
- Fields: `id`, `enabled`, `description`, `allowed_recipient_domains`, `allowed_recipients`,
  `rate_limit_per_min`, `burst`, `mask_recipient`, `effective_rate_limit_per_min`, `effective_burst`
- `secret` is never returned
- 401/403 for missing/invalid auth

**Per-key `mask_recipient` override (RFC 603)**
- `ApiKeyConfig.mask_recipient: Option<bool>` — overrides `[logging].mask_recipient`
- `None` (default) — inherit global setting
- `Some(true)` — always mask `recipient_domain` in logs
- `Some(false)` — never mask, regardless of global setting
- Effective value resolved in `send_mail` handler for the `smtp_submitted` log event

### Changed

- `InMemoryStatusStore::new()` now takes `Arc<Metrics>` as second argument
- `NoopStatusStore` unchanged (no metrics to wire)

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

## [0.12.0] — 2026-05-10

### Theme: Documentation and Security Checklist

No code changes. This release is documentation-only.

### Added

**mdbook documentation structure (RFC 731)**
- `docs/book.toml` — mdbook configuration
- `docs/src/SUMMARY.md` — persona-based navigation (13 chapters)
- `docs/src/introduction.md` — project overview and feature table
- `docs/src/guides/status-tracking.md` — end-to-end status tracking guide
- `docs/src/guides/bulk-sending.md` — bulk send guide with curl examples
- `docs/src/operations/reverse-proxy.md` — nginx / Caddy / relayd / HAProxy configs
- `docs/src/development/contributing.md` — RFC process, code style, test expectations
- Existing docs (`api.md`, `configuration.md`, `openbsd.md`, `security.md`,
  `architecture.md`, `testing.md`, `getting-started.md`, `faq.md`)
  copied into `docs/src/` hierarchy

**Security checklist (RFC 732)**
- `docs/src/operations/security-checklist.md` — 9-category pre-deployment checklist
  - Authentication and API keys (5 items)
  - Recipient and domain policy (3 items)
  - Rate limiting (3 items)
  - Network exposure (3 items)
  - TLS and transport security (2 items)
  - Logging and privacy (3 items)
  - OpenBSD hardening (4 items, platform-conditional)
  - Monitoring and alerting (3 items)
  - Operations and incident response (3 items)
  - Sign-off table

### Documentation structure

```
docs/
  book.toml
  src/
    SUMMARY.md           ← 13 chapters, all links verified
    introduction.md
    getting-started.md
    faq.md
    guides/
      api-reference.md
      configuration.md
      status-tracking.md   ← new
      bulk-sending.md      ← new
    operations/
      security-checklist.md ← new (RFC 732)
      openbsd.md
      reverse-proxy.md      ← new
    development/
      architecture.md
      testing.md
      contributing.md       ← new
```

---

## [0.11.0] — 2026-05-10

### Theme: Hardening Correctness and Shared State

### Added

**OpenBSD SIGHUP `rpath` fix (RFC 721)**
- `rpath` is now always kept in the runtime pledge
- SIGHUP config reload works correctly on OpenBSD
- Security rationale: `unveil` already restricts readable paths to the config
  file only; keeping `rpath` does not expand the effective read surface
- Pledge sets updated: `stdio inet rpath` (smtp/memory), `stdio inet rpath wpath cpath` (sqlite)

**Redis/Valkey shared status store (RFC 722, `--features redis`)**
- `[status] store = "redis"` with `redis_url = "redis://host:port/db"`
- Multi-instance deployments share a single status view
- Key schema: `rele:s:{request_id}` → JSON, TTL via Redis native EXPIRE
- Degraded mode: Redis unavailability logs WARN, does not fail mail delivery
- `max_records` not enforced (use Redis `maxmemory-policy allkeys-lru`)
- `expire_old_records()` is a no-op (Redis handles TTL natively)
- Status lookup during Redis outage returns 404
- Unit tests: serialisation round-trip, key prefix format
- Integration tests: require `REDIS_TEST_URL` env; skipped when unset
- `redis = "0.25"` optional dependency; `[features] redis = ["dep:redis"]`

### Changed

- OpenBSD runtime pledge always includes `rpath` (was dropped in v0.10)
- `[status]` config now accepts `store = "redis"` with `redis_url`
- Store validation: `status.store` must be `memory`, `sqlite`, or `redis`
- Status store error message updated to mention all valid store options

### Docs

- `docs/configuration.md`: `store = "redis"` section added
- `docs/openbsd.md`: SIGHUP reload section added (rpath rationale)

### Test coverage

| Build | unit | integration | total |
|-------|------|-------------|-------|
| default | 81 | 89 | 170 |
| `--features redis` | 88 | — | 88 unit |

---

## [0.10.0] — 2026-05-10

### Theme: Performance and Transport Security

### Added

**Bounded-parallel SMTP submission for send-bulk (RFC 711)**
- Two-phase processing: sequential rate-limit/validate/build → parallel SMTP submit
- `[smtp].bulk_concurrency = 5` (default, 0 = unlimited)
- `tokio::task::JoinSet` + `tokio::sync::Semaphore` for bounded concurrency
- Response `results` always sorted by request index regardless of completion order
- All existing bulk tests pass unchanged; 2 new parallelism tests added

**HTTP server TLS / HTTPS (RFC 712, `--features tls`)**
- `[server].tls_cert` + `[server].tls_key` — PEM certificate and key paths
- `axum-server = "0.7"` with `tls-rustls` (pure-Rust TLS, no OpenSSL)
- Feature flag: `cargo build --release --features tls`
- OpenBSD: `unveil(tls_cert, "r")` + `unveil(tls_key, "r")` before runtime pledge
- Certs loaded into memory before pledge; no additional pledge promises required
- Config validation: both fields required together or neither
- Non-TLS build with `tls_cert` configured → startup error with build instruction
- 4 TLS config validation tests added

**`[rate_limit]` and `[logging]` sections now optional in config**
- Added `#[serde(default)]` + `Default` impls for `RateLimitConfig` and `LoggingConfig`
- Allows minimal TOMLs without explicitly spelling out default-only sections
- Backward-compatible (existing full configs continue to work)

### Changed

- `[smtp].bulk_concurrency` field added (default: 5)
- `[server].tls_cert`, `[server].tls_key` fields added (default: None)
- `axum-server = "0.7"` optional dependency (feature: `tls`)
- `[features] tls = ["dep:axum-server"]` added to Cargo.toml

### Test coverage

| Build | unit | integration | total |
|-------|------|-------------|-------|
| default | 81 | 85 | 166 |
| `--features tls` | 81 | 84 | 165 |

---

## [0.9.0] — 2026-05-10

### Theme: Bulk Submission

### Added

**`POST /v1/send-bulk` (RFC 701)**
- Accepts array of independent mail messages (same schema as `POST /v1/send` per element)
- Per-message pipeline: rate limit → validate → build → SMTP submit → status update
- 202 Accepted with per-message `results` array regardless of individual outcomes
- Partial success is supported: one failed message does not abort others
- Each message gets its own `request_id` and status record
- `bulk_request_id` in response for log correlation
- `GET /v1/submissions/{request_id}` works per-message

**Rate limiting per message (RFC 702)**
- Global, IP, and per-key limits decremented once per message
- Exhaustion mid-array: earlier messages unaffected, remainder rejected with `rate_limited`

**`[mail].max_bulk_messages`** (default: `10`) — cap on messages per bulk request

**`src/api/send_bulk.rs`** — new handler module

### Docs

- `docs/api.md`: `POST /v1/send-bulk` endpoint reference
- `docs/configuration.md`: `max_bulk_messages` field documented

### Test coverage

| Build | unit | integration | total |
|-------|------|-------------|-------|
| default | 81 | 79 | 160 |

---

## [0.8.0] — 2026-05-10

### Theme: SQLite Persistent Status Store

**SQLite status store (RFC 088, `--features sqlite`)**

Provides durable submission status records that survive application restarts.
Memory store remains the default. SQLite is an optional Cargo feature.

Status records are metadata-only in both stores: mail body, subject,
attachments, full recipient addresses, API keys, and SMTP credentials
are never stored.

#### Build

```sh
cargo build --features sqlite
cargo build --release --features sqlite
```

#### Configuration

```toml
[status]
store   = "sqlite"
db_path = "/var/db/http-smtp-rele/status.db"
```

The parent directory must exist before startup:
```sh
install -d -o _http_smtp_rele -m 750 /var/db/http-smtp-rele
```

#### Implementation

- `src/status_sqlite.rs` — `SqliteStatusStore` implementing `StatusStore` trait
- `migrations/001_initial.sql` — schema embedded via `include_str!`
- Single `Mutex<Connection>` with WAL mode
- Schema migration via `PRAGMA user_version` (version 1)
- Breaking migration clears all records and logs a `WARN` event
- Downgrade (newer DB than binary) → startup error
- `max_records` enforced at `put()` and `expire_old_records()`
- Lazy TTL expiry on `get()` + periodic background sweep

#### Security / OpenBSD

`store = "sqlite"` adds `rpath wpath cpath` to pledge and
`unveil(db_path, "rwc")`.  These are applied automatically before the main
pledge call. Use `store = "memory"` for maximum pledge hardening.

#### Non-SQLite builds

`store = "sqlite"` in config with a non-SQLite binary produces a clear
startup error:

```
fatal: status.store = "sqlite" is not available in this build.
       Rebuild with: cargo build --features sqlite
```

#### Dependency

`rusqlite = { version = "0.32", features = ["bundled"], optional = true }`

### Changed

- `security::apply_runtime_restrictions(mode, has_sqlite: bool)` — new signature
- `security::apply_sqlite_restrictions(db_path)` — new public function
- `StatusConfig` gains `db_path: Option<PathBuf>`
- `config::validate_config` checks `store`/`db_path` consistency

### Docs

- `docs/configuration.md` — `[status]` section with SQLite setup guide
- `docs/openbsd.md` — SQLite pledge/unveil additions
- `examples/http-smtp-rele.toml` — `[status]` section with commented `db_path`

---

## [0.12.0] — 2026-05-10

### Theme: Documentation and Security Checklist

No code changes. This release is documentation-only.

### Added

**mdbook documentation structure (RFC 731)**
- `docs/book.toml` — mdbook configuration
- `docs/src/SUMMARY.md` — persona-based navigation (13 chapters)
- `docs/src/introduction.md` — project overview and feature table
- `docs/src/guides/status-tracking.md` — end-to-end status tracking guide
- `docs/src/guides/bulk-sending.md` — bulk send guide with curl examples
- `docs/src/operations/reverse-proxy.md` — nginx / Caddy / relayd / HAProxy configs
- `docs/src/development/contributing.md` — RFC process, code style, test expectations
- Existing docs (`api.md`, `configuration.md`, `openbsd.md`, `security.md`,
  `architecture.md`, `testing.md`, `getting-started.md`, `faq.md`)
  copied into `docs/src/` hierarchy

**Security checklist (RFC 732)**
- `docs/src/operations/security-checklist.md` — 9-category pre-deployment checklist
  - Authentication and API keys (5 items)
  - Recipient and domain policy (3 items)
  - Rate limiting (3 items)
  - Network exposure (3 items)
  - TLS and transport security (2 items)
  - Logging and privacy (3 items)
  - OpenBSD hardening (4 items, platform-conditional)
  - Monitoring and alerting (3 items)
  - Operations and incident response (3 items)
  - Sign-off table

### Documentation structure

```
docs/
  book.toml
  src/
    SUMMARY.md           ← 13 chapters, all links verified
    introduction.md
    getting-started.md
    faq.md
    guides/
      api-reference.md
      configuration.md
      status-tracking.md   ← new
      bulk-sending.md      ← new
    operations/
      security-checklist.md ← new (RFC 732)
      openbsd.md
      reverse-proxy.md      ← new
    development/
      architecture.md
      testing.md
      contributing.md       ← new
```

---

## [0.11.0] — 2026-05-10

### Theme: Hardening Correctness and Shared State

### Added

**OpenBSD SIGHUP `rpath` fix (RFC 721)**
- `rpath` is now always kept in the runtime pledge
- SIGHUP config reload works correctly on OpenBSD
- Security rationale: `unveil` already restricts readable paths to the config
  file only; keeping `rpath` does not expand the effective read surface
- Pledge sets updated: `stdio inet rpath` (smtp/memory), `stdio inet rpath wpath cpath` (sqlite)

**Redis/Valkey shared status store (RFC 722, `--features redis`)**
- `[status] store = "redis"` with `redis_url = "redis://host:port/db"`
- Multi-instance deployments share a single status view
- Key schema: `rele:s:{request_id}` → JSON, TTL via Redis native EXPIRE
- Degraded mode: Redis unavailability logs WARN, does not fail mail delivery
- `max_records` not enforced (use Redis `maxmemory-policy allkeys-lru`)
- `expire_old_records()` is a no-op (Redis handles TTL natively)
- Status lookup during Redis outage returns 404
- Unit tests: serialisation round-trip, key prefix format
- Integration tests: require `REDIS_TEST_URL` env; skipped when unset
- `redis = "0.25"` optional dependency; `[features] redis = ["dep:redis"]`

### Changed

- OpenBSD runtime pledge always includes `rpath` (was dropped in v0.10)
- `[status]` config now accepts `store = "redis"` with `redis_url`
- Store validation: `status.store` must be `memory`, `sqlite`, or `redis`
- Status store error message updated to mention all valid store options

### Docs

- `docs/configuration.md`: `store = "redis"` section added
- `docs/openbsd.md`: SIGHUP reload section added (rpath rationale)

### Test coverage

| Build | unit | integration | total |
|-------|------|-------------|-------|
| default | 81 | 89 | 170 |
| `--features redis` | 88 | — | 88 unit |

---

## [0.10.0] — 2026-05-10

### Theme: Performance and Transport Security

### Added

**Bounded-parallel SMTP submission for send-bulk (RFC 711)**
- Two-phase processing: sequential rate-limit/validate/build → parallel SMTP submit
- `[smtp].bulk_concurrency = 5` (default, 0 = unlimited)
- `tokio::task::JoinSet` + `tokio::sync::Semaphore` for bounded concurrency
- Response `results` always sorted by request index regardless of completion order
- All existing bulk tests pass unchanged; 2 new parallelism tests added

**HTTP server TLS / HTTPS (RFC 712, `--features tls`)**
- `[server].tls_cert` + `[server].tls_key` — PEM certificate and key paths
- `axum-server = "0.7"` with `tls-rustls` (pure-Rust TLS, no OpenSSL)
- Feature flag: `cargo build --release --features tls`
- OpenBSD: `unveil(tls_cert, "r")` + `unveil(tls_key, "r")` before runtime pledge
- Certs loaded into memory before pledge; no additional pledge promises required
- Config validation: both fields required together or neither
- Non-TLS build with `tls_cert` configured → startup error with build instruction
- 4 TLS config validation tests added

**`[rate_limit]` and `[logging]` sections now optional in config**
- Added `#[serde(default)]` + `Default` impls for `RateLimitConfig` and `LoggingConfig`
- Allows minimal TOMLs without explicitly spelling out default-only sections
- Backward-compatible (existing full configs continue to work)

### Changed

- `[smtp].bulk_concurrency` field added (default: 5)
- `[server].tls_cert`, `[server].tls_key` fields added (default: None)
- `axum-server = "0.7"` optional dependency (feature: `tls`)
- `[features] tls = ["dep:axum-server"]` added to Cargo.toml

### Test coverage

| Build | unit | integration | total |
|-------|------|-------------|-------|
| default | 81 | 85 | 166 |
| `--features tls` | 81 | 84 | 165 |

---

## [0.9.0] — 2026-05-10

### Theme: Bulk Submission

### Added

**`POST /v1/send-bulk` (RFC 701)**
- Accepts array of independent mail messages (same schema as `POST /v1/send` per element)
- Per-message pipeline: rate limit → validate → build → SMTP submit → status update
- 202 Accepted with per-message `results` array regardless of individual outcomes
- Partial success is supported: one failed message does not abort others
- Each message gets its own `request_id` and status record
- `bulk_request_id` in response for log correlation
- `GET /v1/submissions/{request_id}` works per-message

**Rate limiting per message (RFC 702)**
- Global, IP, and per-key limits decremented once per message
- Exhaustion mid-array: earlier messages unaffected, remainder rejected with `rate_limited`

**`[mail].max_bulk_messages`** (default: `10`) — cap on messages per bulk request

**`src/api/send_bulk.rs`** — new handler module

### Docs

- `docs/api.md`: `POST /v1/send-bulk` endpoint reference
- `docs/configuration.md`: `max_bulk_messages` field documented

### Test coverage

| Build | unit | integration | total |
|-------|------|-------------|-------|
| default | 81 | 79 | 160 |

---

## [0.8.0] — 2026-05-10

### Theme: SQLite Persistent Status Store

Implements RFC 088 (SQLite backend) with an optional Cargo feature.
The default build remains unchanged; SQLite is opt-in.

### Added

**`--features sqlite` — optional SQLite status store (RFC 088)**
- `[status] store = "sqlite"` — persistent status records that survive restarts
- `[status] db_path = "/var/db/http-smtp-rele/status.db"` — required with sqlite
- `src/status_sqlite.rs`: `SqliteStatusStore` implementing `StatusStore` trait
- `migrations/001_initial.sql`: initial schema, embedded via `include_str!`
- WAL mode (`PRAGMA journal_mode=WAL`) for concurrent read performance
- Schema migration via `PRAGMA user_version` (runs in a transaction)
- Breaking schema changes clear all records and log `WARN` (data is TTL-bounded)
- Downgrade to older binary with newer DB schema: startup error

**Build commands:**
```sh
cargo build --release                    # default: memory store only
cargo build --release --features sqlite  # includes SQLite backend
```

**`AppState::new_with_store`** — constructor accepting pre-built `StatusStore`
for tests and shared-store scenarios.

**`config::load_from_str`** — parse and validate config from a TOML string
(used for feature availability checks in tests).

### Changed

- `security::apply_runtime_restrictions` gains `has_sqlite: bool` parameter
- `security::apply_sqlite_restrictions(db_path)` added (OpenBSD: unveil)
- OpenBSD pledge for SQLite build: adds `rpath wpath cpath`
- `[status].store` now validated: must be `"memory"` or `"sqlite"`
- `[status].db_path` validated: required when `store = "sqlite"`
- Non-SQLite build with `store = "sqlite"` → clear startup error message
- Dependencies: `rusqlite = "0.32"` (optional, bundled SQLite C library)
- Dev-dependencies: `tempfile = "3"`

### Docs

- `docs/getting-started.md`: SQLite build instructions
- `docs/configuration.md`: `[status]` SQLite section (already present; refined)
- `docs/openbsd.md`: pledge surface comparison for memory vs sqlite store

### Test coverage

| Build | unit | integration | total |
|-------|------|-------------|-------|
| default (no sqlite) | 81 | 69 | 150 |
| `--features sqlite` | 92 | 76 | 168 |

---

## [0.7.0] — 2026-05-10

### Theme: Observability and Operations

**Status store Prometheus metrics (RFC 601, implements RFC 089)**
- `rele_status_store_records_current` (Gauge) — live record count
- `rele_status_store_transitions_total{status, code}` (Counter) — per-transition counts
- `rele_status_store_expired_total` (Counter) — TTL expiry events
- `Arc<Metrics>` passed to `InMemoryStatusStore` at construction
- Label cardinality bounded: no `request_id`, `key_id`, `client_ip`, or `recipient_domain` labels

**Authenticated key info endpoint (RFC 602)**
- `GET /v1/keys/self` — returns non-secret config of the authenticated API key
- Fields: `id`, `enabled`, `description`, `allowed_recipient_domains`, `allowed_recipients`,
  `rate_limit_per_min`, `burst`, `mask_recipient`, `effective_rate_limit_per_min`, `effective_burst`
- `secret` is never returned
- 401/403 for missing/invalid auth

**Per-key `mask_recipient` override (RFC 603)**
- `ApiKeyConfig.mask_recipient: Option<bool>` — overrides `[logging].mask_recipient`
- `None` (default) — inherit global setting
- `Some(true)` — always mask `recipient_domain` in logs
- `Some(false)` — never mask, regardless of global setting
- Effective value resolved in `send_mail` handler for the `smtp_submitted` log event

### Changed

- `InMemoryStatusStore::new()` now takes `Arc<Metrics>` as second argument
- `NoopStatusStore` unchanged (no metrics to wire)

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
