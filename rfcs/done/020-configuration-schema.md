# RFC 020 — TOML Configuration Schema

**Status.** Implemented  
**Tracks.** Foundation  
**Touches.** `src/config.rs`, `examples/http-smtp-rele.toml`, `docs/configuration.md`

## Summary

Define the complete TOML configuration schema for `http-smtp-rele`, including all sections,
fields, types, defaults, and constraints. This schema is the single source of truth for
`AppConfig` and its sub-structs.

## Motivation

The configuration file is the primary operational interface. Every operator who deploys
`http-smtp-rele` reads this file. Ambiguous or undocumented fields lead to misconfiguration,
which in a security-sensitive relay can mean open relay or loss of service
(FR-090, FR-091, FR-092, NFR-AVL-003).

## Scope

- All TOML sections and fields.
- Default values.
- Valid value ranges and formats.
- Which fields are required vs. optional.
- Example config (`examples/http-smtp-rele.toml`).

## Non-goals

- Config loading and validation logic (RFC 021).
- Secret redaction in Debug output (RFC 022).
- Mail policy enforcement logic (RFC 052).
- Rate limiter implementation (RFC 071).

## Design

### Full schema

```toml
# http-smtp-rele.toml — example configuration
# All paths are relative to the system root unless absolute.

# ─────────────────────────────────────────────────────────
# [server] — HTTP server settings
# ─────────────────────────────────────────────────────────
[server]
# Bind address. Default: 127.0.0.1:8080
# Must be a socket address string: "IP:PORT"
# DO NOT bind to 0.0.0.0 unless behind a trusted reverse proxy.
bind_address = "127.0.0.1:8080"

# Trusted reverse proxy CIDRs.
# X-Forwarded-For is only trusted when the connecting IP is in this list.
# Default: ["127.0.0.1/32", "::1/128"]
trusted_proxy_cidrs = ["127.0.0.1/32"]

# Maximum request body size in bytes. Default: 1048576 (1 MiB)
max_request_body_bytes = 1048576

# Request read timeout in seconds. Default: 10
request_timeout_seconds = 10

# Maximum concurrent requests. Default: 64
concurrency_limit = 64

# Graceful shutdown timeout in seconds. Default: 30
shutdown_timeout_seconds = 30

# ─────────────────────────────────────────────────────────
# [security] — Access control settings
# ─────────────────────────────────────────────────────────
[security]
# Whether authentication is required. Default: true
# Setting to false is dangerous and intended only for local testing.
require_auth = true

# Source IP allowlist. Empty list = allow all (use with caution).
# Default: ["127.0.0.1/32", "::1/128"]
allowed_source_cidrs = ["127.0.0.1/32"]

# Reject any request that contains a 'headers' field. Default: true
reject_raw_headers = true

# Allow multiple recipients per request. Default: false
allow_multiple_recipients = false

# Maximum recipients per request when allow_multiple_recipients = true. Default: 1
max_recipients = 1

# ─────────────────────────────────────────────────────────
# [rate_limit] — Rate limiting settings
# ─────────────────────────────────────────────────────────
[rate_limit]
# Global rate limit: max requests per minute across all keys and IPs.
# Default: 120
global_per_minute = 120

# Global burst allowance. Default: 20
global_burst = 20

# Per API-key rate limit (per minute). Default: 60
# Overridden per key by api_keys[].rate_limit_per_minute
per_key_per_minute = 60

# Per API-key burst. Default: 10
per_key_burst = 10

# Per source-IP rate limit (per minute). Default: 30
per_ip_per_minute = 30

# Per source-IP burst. Default: 5
per_ip_burst = 5

# ─────────────────────────────────────────────────────────
# [mail] — Mail construction policy
# ─────────────────────────────────────────────────────────
[mail]
# Default From address. Required. Must be a valid RFC 5321 address.
default_from = "noreply@example.com"

# Display name for the From header. Optional.
default_from_name = "Notification Service"

# Maximum subject length in characters. Default: 255
max_subject_chars = 255

# Maximum body size in bytes. Default: 1048576 (1 MiB)
# Must be <= server.max_request_body_bytes
max_body_bytes = 1048576

# Allowed recipient domains. Empty = allow all (use with caution).
# Example: ["example.com", "example.org"]
allowed_recipient_domains = []

# Mask recipient address in logs (log domain only). Default: true
mask_recipient_in_logs = true

# ─────────────────────────────────────────────────────────
# [smtp] — SMTP relay settings
# ─────────────────────────────────────────────────────────
[smtp]
# SMTP submission mode. "smtp" or "pipe". Default: "smtp"
mode = "smtp"

# SMTP server host. Default: "127.0.0.1"
host = "127.0.0.1"

# SMTP server port. Default: 25
port = 25

# SMTP connection timeout in seconds. Default: 10
timeout_seconds = 10

# SMTP hello hostname (EHLO). Default: "localhost"
helo_name = "localhost"

# TLS mode: "none", "starttls", "tls". Default: "none" for localhost.
tls = "none"

# Sendmail binary path. Used when mode = "pipe". Default: "/usr/sbin/sendmail"
# pipe_command = "/usr/sbin/sendmail"

# ─────────────────────────────────────────────────────────
# [logging] — Logging settings
# ─────────────────────────────────────────────────────────
[logging]
# Log level: "error", "warn", "info", "debug", "trace". Default: "info"
level = "info"

# Emit logs as JSON. Default: false
json = false

# Log request body. Default: false. MUST remain false in production.
log_request_body = false

# Log API key secret. Default: false. MUST remain false in production.
log_api_key = false

# ─────────────────────────────────────────────────────────
# [[api_keys]] — API key definitions (one per entry)
# ─────────────────────────────────────────────────────────
[[api_keys]]
# Log-safe identifier. Required.
key_id = "service-a"

# The bearer token value. Required. Treat as a secret.
secret = "change-this-to-a-random-secret"

# Whether this key is active. Default: true
enabled = true

# Human-readable description. Optional.
description = "Web application notification sender"

# Allowed recipient domains for this key. Overrides [mail].allowed_recipient_domains.
# Empty = use global policy.
allowed_recipient_domains = []

# Allowed recipient addresses for this key. Strict list; empty = no per-key restriction.
allowed_recipients = []

# Per-key rate limit (per minute). Overrides [rate_limit].per_key_per_minute.
# 0 = use global per-key default.
rate_limit_per_minute = 0

# Per-key burst. 0 = use global per-key default.
burst = 0

[[api_keys]]
key_id = "service-b"
secret = "another-random-secret"
enabled = false
description = "Disabled service"
```

### Rust struct mapping

```rust
#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub security: SecurityConfig,
    pub rate_limit: RateLimitConfig,
    pub mail: MailConfig,
    pub smtp: SmtpConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub api_keys: Vec<ApiKeyConfig>,
}
```

All sub-structs use `#[serde(default)]` on optional fields so that a minimal config
(with only required fields) is accepted.

### Required fields

| Section | Required field | Notes |
|---------|---------------|-------|
| `[mail]` | `default_from` | Must be valid email |
| `[[api_keys]]` | `key_id` | Must be unique |
| `[[api_keys]]` | `secret` | Must not be empty |

All other fields have defaults (RFC 021 documents the defaults and validates them).

## Implementation Plan

1. Define all config structs in `src/config.rs` with serde derives and `#[serde(default)]`.
2. Write `examples/http-smtp-rele.toml` matching the schema above.
3. Write `docs/configuration.md` documenting every field.
4. Confirm `toml::from_str` parses the example config without error.

## Test Plan

### Unit Tests

- Example config file parses without error.
- A minimal config (only required fields) parses successfully.
- An empty `[[api_keys]]` section parses as an empty vec.

## Security Considerations

- `secret` fields in `ApiKeyConfig` must never appear in Debug output (RFC 022).
- `log_api_key = true` and `log_request_body = true` are documented as "MUST remain false in
  production" and should emit a `warn!` log event at startup if enabled.
- `require_auth = false` is dangerous; emit `warn!` at startup.

## Operational Considerations

- File path: `/etc/http-smtp-rele.toml` by default; overridable with `--config`.
- File permissions: `640`, owner `root`, group `_http_smtp_rele`.
- Do not hot-reload config; a restart is required for config changes.

## Documentation Changes

- Create `examples/http-smtp-rele.toml`.
- Create `docs/configuration.md` with the full field reference.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-020-01 | Example config parses without error. |
| AC-020-02 | All fields have documented defaults. |
| AC-020-03 | All struct fields are covered by the example config. |
| AC-020-04 | `docs/configuration.md` documents every field. |

## Open Questions

None.
