# RFC 021 — Configuration Loading and Fail-Fast Validation

**Status.** Implemented  
**Tracks.** Foundation  
**Touches.** `src/config.rs`, `src/main.rs`

## Summary

Define `config::load(path)` — the function that reads, parses, applies defaults, and validates
the TOML configuration — and the fail-fast policy: any validation error aborts the process
with a clear log message before the HTTP server starts.

## Motivation

A misconfigured relay is a security hazard. An invalid `default_from` could cause mail delivery
failures. An empty `api_keys` list means all requests are unauthenticated. A negative rate limit
could panic at runtime. Catching all of these at startup, before accepting any connection,
implements `NFR-AVL-003` and prevents silent misconfiguration (FR-092, AC-012).

## Scope

- `config::load(path: &Path) -> Result<AppConfig, ConfigError>`.
- Validation rules for every field.
- Default value application.
- `--config` CLI flag plumbing.
- Startup log messages for dangerous settings.

## Non-goals

- Hot-reload (not in MVP).
- Environment variable interpolation.
- Secret decryption or vault integration.
- Schema migration between versions.

## Design

### Loading pipeline

```
read file
  -> toml::from_str::<AppConfig>(content)
  -> apply_defaults(&mut config)
  -> validate(&config) -> Result<(), ConfigError>
  -> return Ok(config)
```

Each step can fail with a distinct error message:

| Stage | Error type | Example message |
|-------|-----------|-----------------|
| File not found | `ConfigError::NotFound` | "Config file not found: /etc/http-smtp-rele.toml" |
| File read error | `ConfigError::Io` | "Could not read config file: permission denied" |
| TOML parse error | `ConfigError::Parse` | "Config parse error at line 12: ..." |
| Validation error | `ConfigError::Validation` | "Config invalid: default_from is not a valid email address" |

### Validation rules

#### `[server]`

| Field | Rule |
|-------|------|
| `bind_address` | Must parse as a `SocketAddr` |
| `trusted_proxy_cidrs` | Each entry must parse as a CIDR (`ipnet::IpNet`) |
| `max_request_body_bytes` | Must be > 0 and ≤ 10 MiB |
| `request_timeout_seconds` | Must be > 0 |
| `concurrency_limit` | Must be > 0 |
| `shutdown_timeout_seconds` | Must be > 0 |

#### `[security]`

| Field | Rule |
|-------|------|
| `allowed_source_cidrs` | Each entry parses as a CIDR; empty list is allowed (emit warn) |
| `require_auth = false` | Emit `warn!("require_auth is false; all requests will be unauthenticated")` |
| `max_recipients` | Must be ≥ 1; if `allow_multiple_recipients = false`, max_recipients is forced to 1 |

#### `[rate_limit]`

| Field | Rule |
|-------|------|
| `global_per_minute` | Must be > 0 |
| `global_burst` | Must be ≥ 1 |
| `per_key_per_minute` | Must be > 0 |
| `per_ip_per_minute` | Must be > 0 |

#### `[mail]`

| Field | Rule |
|-------|------|
| `default_from` | Must parse as a valid RFC 5321 address (use `lettre::Address::from_str` or similar) |
| `max_subject_chars` | Must be > 0 |
| `max_body_bytes` | Must be > 0 and ≤ `server.max_request_body_bytes` |
| `allowed_recipient_domains` | Each entry must be a syntactically valid domain name |

#### `[smtp]`

| Field | Rule |
|-------|------|
| `mode` | Must be `"smtp"` or `"pipe"` |
| `host` | Must not be empty |
| `port` | Must be 1–65535 |
| `timeout_seconds` | Must be > 0 |

#### `[[api_keys]]`

| Field | Rule |
|-------|------|
| `key_id` | Must not be empty; must be unique across all keys |
| `secret` | Must not be empty |
| `api_keys` (list) | Must contain at least one entry when `require_auth = true` |

#### Dangerous setting warnings

Emit `warn!` at startup for any of:
- `require_auth = false`
- `allowed_source_cidrs = []` (empty = allow all)
- `allowed_recipient_domains = []` (global, means no restriction)
- `log_request_body = true`
- `log_api_key = true`
- Any key with `enabled = false` (informational, not a warning)

### `ConfigError`

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("config file not found: {0}")]
    NotFound(PathBuf),

    #[error("could not read config file: {0}")]
    Io(#[from] std::io::Error),

    #[error("config parse error: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("config validation failed: {0}")]
    Validation(String),
}
```

### CLI integration

```rust
#[derive(clap::Parser)]
struct Cli {
    /// Path to the TOML configuration file.
    #[arg(long, default_value = "/etc/http-smtp-rele.toml")]
    config: PathBuf,
}
```

In `main`:
```rust
let cli = Cli::parse();
let config = config::load(&cli.config).unwrap_or_else(|e| {
    tracing::error!(error = %e, "startup failed");
    std::process::exit(1);
});
```

## Implementation Plan

1. Add `ConfigError` to `src/config.rs`.
2. Write `config::load(path)` with the four-stage pipeline.
3. Implement `validate(&config)` checking all rules above.
4. Write dangerous-setting warning emitters.
5. Add `Cli` struct with clap.
6. Wire into `main.rs`.
7. Write tests.

## Test Plan

### Unit Tests

- Valid example config loads without error.
- Missing config file → `ConfigError::NotFound`.
- Invalid TOML → `ConfigError::Parse`.
- Empty `api_keys` with `require_auth = true` → `ConfigError::Validation`.
- Invalid `default_from` → `ConfigError::Validation`.
- Invalid CIDR in `trusted_proxy_cidrs` → `ConfigError::Validation`.
- Invalid SMTP port 0 → `ConfigError::Validation`.
- `max_body_bytes > max_request_body_bytes` → `ConfigError::Validation`.
- Duplicate `key_id` → `ConfigError::Validation`.
- `require_auth = false` → loads OK but emits a warning.

### Integration Tests

- Binary exits 1 with a logged error when given a nonexistent config file.
- Binary exits 1 with a logged error when config has a validation failure.
- Binary starts normally with the example config.

## Security Considerations

- Validation ensures `secret` is never empty; an empty secret would accept any token.
- Duplicate `key_id` would make rate limit isolation impossible; must be rejected.
- Dangerous setting warnings at startup give operators an explicit signal when the relay is
  in a less-safe configuration.
- Config load happens before `pledge`; after `pledge("stdio inet")`, re-reading the config
  is not possible, so the config must be fully loaded before security restrictions are applied.

## Operational Considerations

- Config reload requires a process restart (`rcctl restart`).
- The `--config` flag can be set in the rc.d `daemon_flags`.
- Validation error messages should be specific enough to guide the operator to the fix.

## Documentation Changes

- Document all validation rules in `docs/configuration.md`.
- Document exit codes in `docs/openbsd.md`.
- Document the `--config` flag in `README.md`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-021-01 | `config::load` returns `Ok` for the example config. |
| AC-021-02 | Empty `api_keys` with `require_auth = true` returns `Err`. |
| AC-021-03 | Invalid `default_from` returns `Err`. |
| AC-021-04 | Duplicate `key_id` returns `Err`. |
| AC-021-05 | `require_auth = false` emits a `warn!` log but does not return `Err`. |
| AC-021-06 | `main` exits with code 1 on any `ConfigError`. |

## Open Questions

- Whether to support a `--validate` flag that runs config loading and exits 0 or 1 without
  starting the server. Useful for CI config checks. Deferred to v0.2.
