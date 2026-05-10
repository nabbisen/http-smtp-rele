# Configuration Reference

The configuration file is TOML. The default path is `/etc/http-smtp-rele.toml`;
override with `--config <path>`.

Invalid configuration causes immediate process exit with a clear error message (fail-fast).

---

## [server]

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `bind_address` | string | `"127.0.0.1:8080"` | TCP address to listen on. **Never bind to `0.0.0.0` without a firewall in front.** |
| `max_request_body_bytes` | integer | `1048576` | Maximum HTTP request body in bytes. Requests over this limit receive 413. |
| `request_timeout_seconds` | integer | `30` | Wall-clock timeout for the full request cycle. |

---

## [security]

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `trust_proxy_headers` | bool | `false` | Read `X-Forwarded-For` for client IP resolution. Applies only when the peer IP is in `trusted_source_cidrs`. |
| `trusted_source_cidrs` | string[] | `[]` | CIDRs whose `X-Forwarded-For` headers may be trusted. Used only when `trust_proxy_headers = true`. |
| `allowed_source_cidrs` | string[] | `[]` | CIDRs from which connections are permitted at all. Empty = allow all source IPs. Applied after IP resolution; independent of proxy header trust. |

**`trusted_source_cidrs` vs. `allowed_source_cidrs`:** These serve distinct purposes.
`trusted_source_cidrs` controls proxy header trust for IP resolution.
`allowed_source_cidrs` controls which resolved client IPs may connect at all.
An IP can be in one list without the other.

If `trust_proxy_headers = true` and the peer is in `trusted_source_cidrs`,
`http-smtp-rele` uses `X-Forwarded-For` to resolve the client IP. Otherwise, proxy
headers are ignored and the socket peer IP is used directly.

---

## [[api_keys]]

Repeat this section for each API key. At least one enabled key is required.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | ✓ | Non-secret key identifier. In TOML: `id`. In logs and request context: `key_id` (same value, renamed for clarity in log output). |
| `secret` | string | ✓ | The bearer token secret. Generate with `openssl rand -base64 32`. |
| `enabled` | bool | ✓ | Set to `false` to revoke without removing. |
| `description` | string | — | Free-text note (not logged by default). |
| `allowed_recipient_domains` | string[] | — | Additional domain restriction for this key. Empty = inherit global policy. |
| `rate_limit_per_min` | integer | — | Per-key rate limit override. `0` = use global default. |

> **Key rotation:** Add the new key, deploy, then set `enabled = false` on the old key and
> restart. Zero downtime rotation with two restart cycles.

---

## [rate_limit]

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `global_per_min` | integer | `60` | Maximum requests per minute across all clients. |
| `per_ip_per_min` | integer | `20` | Maximum requests per minute from a single source IP. |
| `burst_size` | integer | `5` | Token bucket burst capacity. A fresh bucket starts at this size. |

> **In-memory limitation:** Rate limit state resets on process restart. See [security.md](security.md).

---

## [mail]

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `default_from` | string | — | **Required.** `From` address for all outgoing mail. Clients cannot override. |
| `default_from_name` | string | — | Display name for `From`. Client's `from_name` takes precedence. |
| `allowed_recipient_domains` | string[] | `[]` | Allowlist of recipient domains. **Empty = allow any domain (open relay risk).** Always set in production. |
| `max_subject_chars` | integer | `255` | Maximum subject length in UTF-8 characters. |
| `max_body_bytes` | integer | `524288` | Maximum body size in bytes. Must be ≤ `server.max_request_body_bytes`. |
| `mask_recipient` | bool | `true` | Log only recipient domain, not full address. Recommended for privacy. |

---

## [smtp]

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `host` | string | `"127.0.0.1"` | SMTP server hostname or IP. On OpenBSD with OpenSMTPD: `127.0.0.1`. |
| `port` | integer | `25` | SMTP port. |
| `connect_timeout_seconds` | integer | `5` | TCP connect timeout. |
| `submission_timeout_seconds` | integer | `30` | Full SMTP session timeout (connect + EHLO + DATA + QUIT). |

> **OpenBSD note:** Use an IP address for `host`, not a hostname. With `pledge("stdio inet")`,
> DNS resolution is not available after startup hardening. See [openbsd.md](openbsd.md).

---

## [logging]

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | string | `"info"` | Log level: `trace`, `debug`, `info`, `warn`, `error`. Overridden by `RUST_LOG`. |
| `format` | string | `"text"` | `"text"` for human-readable output; `"json"` for structured log aggregators. |
| `mask_recipient` | bool | `false` | Log only the recipient domain, not the full address. Recommended for privacy. |

---

## Dangerous settings

These settings reduce security and should be changed with care:

| Setting | Risk |
|---------|------|
| `bind_address = "0.0.0.0:..."` | Exposes the relay to all network interfaces |
| `allowed_recipient_domains = []` | Creates an open relay for any domain |
| `trust_proxy_headers = true` without `trusted_source_cidrs` | Allows IP spoofing via forged X-Forwarded-For |
| `mask_recipient_in_logs = false` | Stores recipient email addresses in logs |

---

## Example configuration

See [examples/http-smtp-rele.toml](../examples/http-smtp-rele.toml) for a fully-annotated
example configuration.

---

## `[status]` — Submission Status Tracking

Controls whether `http-smtp-rele` records per-request metadata in a status store.
This allows clients to query what the relay observed during request handling and
SMTP submission via `GET /v1/submissions/{request_id}`.

**Status records are metadata only.** Mail body, subject, attachments, full
recipient addresses, API keys, and SMTP credentials are never stored.

```toml
[status]
enabled                  = true    # false to disable entirely
store                    = "memory" # or "sqlite" (requires --features sqlite)
ttl_seconds              = 3600    # record lifetime; SIGHUP-reloadable
max_records              = 10000   # cap on live records; SIGHUP-reloadable
cleanup_interval_seconds = 60      # background sweep interval; SIGHUP-reloadable
```

### `store = "memory"` (default)

Non-durable in-process store. All records are lost on restart. No extra
dependencies or filesystem access. Recommended for most deployments and for
all security-sensitive environments.

### `store = "sqlite"` — persistent store

Survives application restarts. Suitable for single-host deployments where
clients frequently poll `GET /v1/submissions/{request_id}` after a restart.

**Requires:** binary built with `--features sqlite`.

```toml
[status]
store   = "sqlite"
db_path = "/var/db/http-smtp-rele/status.db"
```

**Preconditions:**

1. The **parent directory** must exist before startup; the application does not create it:
   ```sh
   install -d -o _http_smtp_rele -m 750 /var/db/http-smtp-rele
   ```
2. The SQLite **file** is created automatically on the first run.
3. `store` and `db_path` require a restart to take effect.

**Schema migration:**  
The schema version is tracked with `PRAGMA user_version`. Migrations run
automatically at startup and are embedded in the binary. A breaking schema
change (rare: major structural redesign only) clears all status records and
logs a `WARN` event — acceptable because records are TTL-bounded metadata.
Downgrading to an older binary version with a newer database triggers a
startup error with a clear message.

**OpenBSD pledge implications:**

| Store | pledge promises added |
|-------|----------------------|
| `memory` | *(none)* |
| `sqlite` | `rpath wpath cpath` |

SQLite mode increases the pledge surface. For maximum hardening on OpenBSD,
use `store = "memory"` and accept non-durable status records.


### `store = "redis"` — shared distributed store

Enables multi-instance deployments where all instances share a single status view.
Requires `--features redis` build.

```toml
[status]
store     = "redis"
redis_url = "redis://127.0.0.1:6379/0"
# or: redis_url = "redis+unix:///var/run/redis/redis.sock?db=0"
```

**Key schema:** `rele:s:{request_id}` → JSON, TTL set via Redis `EXPIRE`.

**`max_records` is not enforced.** Configure `maxmemory-policy allkeys-lru`
in Redis/Valkey instead.

**Degraded mode:** Redis unavailability logs a warning but does not fail mail
delivery. Status lookups return 404 while Redis is unavailable.

**OpenBSD:** no additional pledge promises required (Redis uses TCP, `inet` already present).

### `enabled = false`

Disables status tracking entirely. `request_id` is still issued and appears
in response headers and logs. `GET /v1/submissions/{request_id}` always
returns 404.

### SIGHUP-reloadable settings

| Setting | Reloadable |
|---------|-----------|
| `ttl_seconds` | ✓ SIGHUP |
| `max_records` | ✓ SIGHUP |
| `cleanup_interval_seconds` | ✓ SIGHUP |
| `enabled` | ✗ restart |
| `store` | ✗ restart |
| `db_path` | ✗ restart |
