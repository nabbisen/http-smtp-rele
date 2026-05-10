# Contributing

Contributions to `http-smtp-rele` are welcome. This page covers the
development setup, RFC process, and pull request guidelines.

---

## Development setup

### Requirements

- Rust 1.91 (via system package `rustc-1.91` on Debian/Ubuntu)
- `cargo-1.91`

```sh
# Verify
rustc-1.91 --version
cargo-1.91 --version
```

### Build commands

```sh
# Set toolchain pointers
export RUSTC=/usr/bin/rustc-1.91
export RUSTDOC=/usr/bin/rustdoc-1.91
CARGO=/usr/bin/cargo-1.91

# Default build
$CARGO build

# With optional features
$CARGO build --features sqlite
$CARGO build --features tls
$CARGO build --features redis

# Release
$CARGO build --release --workspace
```

### Running tests

```sh
# All gates (check + lib + integration + RFC integrity)
make gate

# Unit tests only
$CARGO test --lib

# Integration tests (starts an in-process SMTP stub)
$CARGO test --test integration_tests

# With Redis (requires a running Redis)
REDIS_TEST_URL=redis://127.0.0.1:6379/0 $CARGO test --features redis
```

---

## Project layout

```
src/
  lib.rs            — AppState, public exports
  config.rs         — TOML configuration types and validation
  auth.rs           — API key authentication extractor
  error.rs          — AppError → HTTP response mapping
  mail.rs           — lettre Message construction
  smtp.rs           — SMTP submission (direct and pipe modes)
  rate_limit.rs     — token bucket rate limiter
  validation.rs     — MailRequest validation and sanitization
  sanitize.rs       — header injection protection
  security.rs       — pledge/unveil (OpenBSD) + no-op stubs
  metrics.rs        — Prometheus counters and histograms
  logging.rs        — tracing subscriber initialisation
  request_id.rs     — RequestId newtype (req_ + ULID)
  status.rs         — StatusStore trait + shared types
  status_memory.rs  — in-memory StatusStore implementation
  status_sqlite.rs  — SQLite StatusStore (--features sqlite)
  status_redis.rs   — Redis StatusStore (--features redis)
  api/
    mod.rs          — Axum router, request_id_layer middleware
    send.rs         — POST /v1/send
    send_bulk.rs    — POST /v1/send-bulk
    submissions.rs  — GET /v1/submissions/{request_id}
    keys.rs         — GET /v1/keys/self
    health.rs       — GET /healthz, GET /readyz
    metrics_handler.rs — GET /metrics
  tests.rs          — unit tests for core modules
crates/cli/
  src/main.rs       — binary entry point
tests/
  integration_tests.rs — full HTTP stack integration tests
  common.rs         — shared test helpers
  smtp_stub.rs      — in-process SMTP server stub
migrations/
  001_initial.sql   — SQLite schema
docs/
  src/              — mdbook source (this directory)
rfcs/
  done/             — implemented RFCs
  proposed/         — RFCs under review
```

---

## RFC process

Design decisions are captured in RFC files before implementation.

1. Create `rfcs/proposed/NNN-short-name.md` using the RFC template.
2. Discuss in the pull request; revise as needed.
3. Implement the feature referencing the RFC in code comments.
4. Move the file to `rfcs/done/NNN-short-name.md`.
5. Register the RFC in `rfcs/README.md`.
6. Run `make check-rfcs` to verify integrity.

**RFC numbering convention:**

| Range | Track |
|-------|-------|
| 001–009 | T0 Governance |
| 010–029 | T1 Security |
| 030–059 | T2 HTTP API / T3 Rate Limiting |
| 060–089 | T4 SMTP / T5 Observability |
| 090–129 | T6 Testing / Documentation |
| 200–599 | Version-specific features (200s = v0.2, etc.) |
| 600+    | Cross-cutting / operational |

---

## Pull request guidelines

- Keep PRs focused: one RFC per PR where possible.
- Ensure `make gate` passes before opening.
- Update `CHANGELOG.md` with a concise entry.
- Add or update tests to cover the new behaviour.
- Documentation updates are required for user-visible changes.

### Code style

- Document public items with `///` doc comments.
- English for all comments, docs, and identifiers.
- `#[serde(deny_unknown_fields)]` on request DTOs.
- Never store secrets in `String`; use `SecretString`.
- Sensitive debug output must redact via `SecretString`'s `Debug` impl.

### Test coverage expectations

| Change type | Required tests |
|-------------|---------------|
| New endpoint | Integration test covering happy path + error cases |
| New config field | Config validation test + `Default` coverage |
| Security fix | Test that exercises the fixed behaviour |
| Bug fix | Regression test |
| Refactor | Existing tests must continue to pass |
