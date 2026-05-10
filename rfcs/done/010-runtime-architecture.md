# RFC 010 — Runtime Architecture and Crate Structure

**Status.** Implemented  
**Tracks.** Foundation  
**Touches.** `Cargo.toml`, `src/main.rs`, `src/app.rs`, all `src/` modules

## Summary

Define the Rust crate structure, module boundaries, async runtime, and HTTP framework for
`http-smtp-rele`. This is the skeleton that all subsequent RFCs build upon.

## Motivation

Establishing clear module boundaries early prevents the entangling of concerns (auth inside
handlers, validation inside SMTP code) that is difficult to untangle later. The choice of
async runtime and HTTP framework also determines what middleware patterns are available for
rate limiting, body size enforcement, and request tracing (FR-001, NFR-MNT-001, NFR-MNT-002).

## Scope

- `Cargo.toml` with edition 2024, initial dependencies, and release profile.
- `src/main.rs` — entry point: parse CLI, load config, set up runtime, run app.
- `src/app.rs` — builds the Axum router, attaches middleware, returns a `Server` future.
- Module stubs for: `config`, `error`, `context`, `logging`, `auth`, `validation`, `sanitize`,
  `mail`, `smtp`, `rate_limit`, `policy`, `security`, `api` (with submodules).
- `AppState` — the shared application state passed to handlers via `axum::extract::State`.
- Async runtime: Tokio, `#[tokio::main]`.
- HTTP framework: Axum 0.8.

## Non-goals

- Actual implementation of any module beyond stubs.
- Configuration loading (RFC 020–025).
- Authentication logic (RFC 040–044).
- SMTP transport (RFC 060–062).
- OpenBSD pledge/unveil (RFC 090–091).

## Design

### Crate layout

Single Cargo package. No workspace for MVP.

```
http-smtp-rele/
├── Cargo.toml
├── Cargo.lock          ← committed to VCS
└── src/
    ├── main.rs         ← entry point
    ├── app.rs          ← router + middleware builder
    ├── api.rs          ← module declaration for api/
    ├── api/
    │   ├── handlers.rs ← request handlers (thin)
    │   ├── routes.rs   ← route definitions
    │   ├── responses.rs← response types
    │   └── extractors.rs← custom axum extractors
    ├── auth.rs
    ├── config.rs
    ├── context.rs
    ├── error.rs
    ├── logging.rs
    ├── mail.rs
    ├── policy.rs
    ├── rate_limit.rs
    ├── sanitize.rs
    ├── security.rs
    ├── smtp.rs
    └── validation.rs
```

No `mod.rs` files. Submodule files use `parent.rs` + `parent/child.rs` pattern (Rust 2018+).

### Module responsibilities (summary)

| Module | Responsibility |
|--------|----------------|
| `config` | Load and validate TOML; produce immutable `AppConfig`. |
| `error` | `AppError` enum; mapping to HTTP status + JSON body. |
| `context` | `RequestContext` carrying `request_id`, `client_ip`, `key_id`. |
| `logging` | `tracing` subscriber initialization; audit event helpers. |
| `auth` | Parse auth header; constant-time comparison; produce `AuthContext`. |
| `validation` | Validate `MailRequest`; produce `ValidatedMailRequest`. |
| `sanitize` | Detect CR/LF and control characters in header-bound fields. |
| `mail` | Build `lettre::Message` from `ValidatedMailRequest`. |
| `smtp` | Initialize SMTP transport; submit message; map errors. |
| `rate_limit` | Global, IP, and key token buckets. |
| `policy` | Recipient domain policy; API-key-specific permissions. |
| `security` | `pledge` / `unveil` on OpenBSD; no-op on other platforms. |
| `api` | Routes, handlers, responses, extractors. |

### `AppState`

Shared state, cheaply cloneable via `Arc`:

```rust
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub rate_limiter: Arc<RateLimiter>,
    pub smtp: Arc<SmtpTransportHandle>,
}
```

Passed to handlers via `axum::extract::State<AppState>`.

### `main.rs` structure

```rust
#[tokio::main]
async fn main() {
    // 1. Parse CLI args (--config path)
    // 2. Initialize logging (before anything else that might log)
    // 3. Load and validate config (fail fast)
    // 4. Apply OpenBSD security restrictions
    // 5. Build AppState
    // 6. Build and run Axum server with graceful shutdown
}
```

Config loading and security application happen before the async runtime starts request handling,
so errors abort with a clear message rather than panicking in a handler.

### Cargo.toml initial dependencies

```toml
[package]
name = "http-smtp-rele"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"  # Rust 2024 edition minimum

[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["limit", "timeout", "trace"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
lettre = { version = "0.11", default-features = false, features = ["smtp-transport", "tokio1-native-tls", "builder"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
thiserror = "2"
clap = { version = "4", features = ["derive"] }
ipnet = "2"
subtle = "2"
uuid = { version = "1", features = ["v4"] }

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"

[profile.dev]
panic = "abort"
```

OpenBSD platform bindings (via `libc`) are conditionally compiled in `security.rs`.

## Implementation Plan

1. Run `cargo new http-smtp-rele --bin`.
2. Set edition to `"2024"` in `Cargo.toml`.
3. Add all listed dependencies.
4. Create all module stub files (empty or with `pub mod` declarations).
5. Write `src/main.rs` with the five-step skeleton (stubs for each step).
6. Write `src/app.rs` returning a placeholder router with `/healthz`.
7. Confirm `cargo build` and `cargo clippy --all-targets -- -D warnings` pass.
8. Confirm `cargo test` passes (no tests yet, just compilation).

## Test Plan

### Unit Tests

- Compilation succeeds: `cargo build`.
- No clippy warnings: `cargo clippy --all-targets -- -D warnings`.
- No formatting issues: `cargo fmt --check`.

### Integration Tests

- The binary starts and `/healthz` returns 200 (minimal test; full handler in RFC 034).

### Operational Tests

- The binary exits non-zero on `--help` (clap default behavior).
- The binary starts without a config file and immediately prints a clear error.

## Security Considerations

- `AppState` must not expose mutable references to the config after initialization.
- No `unwrap()` or `expect()` in request handling paths (enforced by clippy in later RFCs).
- The module structure isolates security-sensitive logic (`auth`, `sanitize`, `security`) from
  generic infrastructure. This boundary must be maintained as the codebase grows.

## Operational Considerations

- `Cargo.lock` is committed to ensure reproducible builds.
- `rust-version` in `Cargo.toml` documents the minimum supported Rust version.
- `panic = "abort"` in both dev and release profiles prevents stack unwinding, which simplifies
  OpenBSD pledge restrictions.

## Documentation Changes

- Create `README.md` with a placeholder.
- Create `CHANGELOG.md` skeleton.
- Create `ROADMAP.md` skeleton.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-010-01 | `cargo build` passes. |
| AC-010-02 | `cargo clippy --all-targets -- -D warnings` passes. |
| AC-010-03 | `cargo fmt --check` passes. |
| AC-010-04 | All module stubs exist as listed in the crate layout. |
| AC-010-05 | No `mod.rs` files are present. |
| AC-010-06 | `AppState` is defined in `app.rs` or `context.rs` with `Arc<AppConfig>`. |
| AC-010-07 | `main.rs` follows the five-step startup sequence (even if steps are stubs). |

## Open Questions

- Whether to use `ulid` instead of `uuid` for request IDs. ULIDs are lexicographically sortable,
  which aids log analysis. Decision deferred to RFC 035 (Request ID policy).
