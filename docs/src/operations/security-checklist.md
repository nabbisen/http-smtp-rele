# Security Checklist

A pre-deployment checklist for operators running `http-smtp-rele` in production.
Work through each section before accepting external traffic.

Print this page or copy it to your deployment runbook.

---

## 1. Authentication and API Keys

### 1.1 Authentication is always required

- [ ] `[security] require_auth = true` (this is the default; verify it is not overridden)
- [ ] No unauthenticated access path exists to `/v1/send` or `/v1/send-bulk`

> **Risk if skipped:** An open relay will relay mail for any caller.
> `require_auth = false` is only appropriate for local loopback-only deployments
> with network-level access control.

### 1.2 API key secrets are cryptographically strong

- [ ] Each `secret` is generated with a cryptographically random source:
  ```sh
  openssl rand -base64 32
  ```
- [ ] No example or default secrets from documentation are in use
- [ ] No secret is shorter than 32 bytes of entropy

### 1.3 API key hygiene

- [ ] Each integration or calling service has its own key (`id` is unique per caller)
- [ ] Keys that are no longer in use have `enabled = false`
- [ ] The config file is readable only by the service user:
  ```sh
  chmod 600 /etc/http-smtp-rele.toml
  chown _http_smtp_rele /etc/http-smtp-rele.toml
  ```
- [ ] Key rotation procedure is documented in your runbook (see
  [Security Guide](./security-checklist.md#key-rotation))

### 1.4 Key rotation procedure is ready

- [ ] New key entry is added and deployed first
- [ ] Client is updated to use new key before old key is disabled
- [ ] Old key is set `enabled = false` in a subsequent deploy

---

## 2. Recipient and Domain Policy

### 2.1 Recipient domain allowlist is configured

- [ ] `[mail] allowed_recipient_domains` is **non-empty** in production
- [ ] Each domain in the list is intentional and owned or authorised by you
- [ ] No wildcard entries (e.g., `*.com`) appear in the list

> **Risk if skipped:** An empty `allowed_recipient_domains` creates an open relay
> that will forward to any address.

### 2.2 Per-key domain restrictions are reviewed

- [ ] For keys that should only send to a subset of domains, `allowed_recipient_domains`
  is set at the key level in `[[api_keys]]`
- [ ] Keys with no per-key restriction inherit the global list (verify this is intentional)

### 2.3 `From` address is correct

- [ ] `[mail] default_from` is a real address owned by your organisation
- [ ] The address passes SPF/DKIM/DMARC for your domain
- [ ] `default_from_name` is set to a recognisable display name

---

## 3. Rate Limiting

### 3.1 Global limits are set

- [ ] `[rate_limit] global_per_min` is set to a value appropriate for your expected volume
- [ ] `[rate_limit] global_burst` is not excessively large (burst ≤ 2× sustained rate)

### 3.2 Per-source limits are set

- [ ] `per_ip_per_min` is configured to limit individual callers
- [ ] Per-key limits (`rate_limit_per_min` in `[[api_keys]]`) are set for
  high-volume or untrusted integrations

### 3.3 Rate limit state and limitations are understood

- [ ] Rate limit state is **in-memory only** — it resets on restart
- [ ] Multiple relay instances do **not** share rate limit state (use an application
  load balancer with sticky sessions, or accept the limitation)
- [ ] Exceeded rate limits return `429`; they do not block the IP at the firewall
  (add firewall rules for persistent abuse sources)

---

## 4. Network Exposure

### 4.1 Bind address is loopback (or intended interface only)

- [ ] `[server] bind_address` is `127.0.0.1:PORT` (loopback) for proxy deployments
- [ ] If binding to `0.0.0.0`, a firewall restricts external access to known sources
- [ ] `/readyz` and `/metrics` are not publicly accessible (see [Reverse Proxy](./reverse-proxy.md))

### 4.2 SMTP server access is restricted

- [ ] The SMTP server (`[smtp] host`) is not publicly exposed
- [ ] SMTP auth credentials (`auth_user`, `auth_password`) are present if the SMTP
  server requires authentication
- [ ] SMTP TLS is enabled where the connection is not local loopback:
  ```toml
  [smtp]
  tls = "starttls"   # or "tls"
  ```

### 4.3 Source IP allowlist is configured for high-security deployments

If callers are known fixed IPs:

```toml
[security]
allowed_source_cidrs = ["203.0.113.0/24"]
```

- [ ] `allowed_source_cidrs` is set if callers are a known fixed set

---

## 5. TLS and Transport Security

### 5.1 Client-to-relay transport is encrypted

Choose one:

**Option A: Reverse proxy handles TLS** (recommended)

- [ ] Proxy (nginx, Caddy, relayd) terminates TLS
- [ ] Relay binds to loopback; proxy accepts external HTTPS
- [ ] See [Reverse Proxy Setup](./reverse-proxy.md)

**Option B: Built-in TLS** (direct-expose deployments)

- [ ] Built with `--features tls`
- [ ] `[server] tls_cert` and `tls_key` point to valid PEM files
- [ ] Certificate is from a trusted CA (not self-signed for production)
- [ ] Certificate renewal is automated (e.g., `acme-client`, `certbot`)

### 5.2 TLS configuration is correct

- [ ] TLS 1.2 minimum (proxy or OS configuration)
- [ ] Weak cipher suites are disabled
- [ ] HSTS header is set at the proxy level for HTTPS deployments

---

## 6. Logging and Privacy

### 6.1 Recipient addresses are masked in logs

- [ ] `[logging] mask_recipient = true` (default) is in effect, or
- [ ] Per-key masking is explicitly reviewed for each `[[api_keys]]` entry
  with `mask_recipient = false`

> **Risk if skipped:** Recipient email addresses appear in structured log output,
> which may violate GDPR/privacy regulations depending on jurisdiction.

### 6.2 Log output does not contain secrets

- [ ] API key secrets (`SecretString`) are confirmed redacted: the `Debug` impl
  outputs `"[REDACTED]"` and the field is excluded from tracing spans
- [ ] No custom log sinks capture raw HTTP request bodies

### 6.3 Log retention policy is defined

- [ ] Logs are retained for a defined period appropriate for your audit requirements
- [ ] Old logs are deleted or archived on schedule

---

## 7. OpenBSD Hardening (OpenBSD deployments only)

### 7.1 `pledge` and `unveil` are active

- [ ] Process is running on OpenBSD (verify with `uname -s`)
- [ ] `pledge` and `unveil` restrictions are confirmed in startup log:
  ```
  INFO app.started version=... bind_address=...
  ```
  (the process would have exited with `Abort trap` if pledge/unveil failed)

### 7.2 File permissions are tight

- [ ] Config file: `chmod 600`, owned by service user
- [ ] SQLite database directory (if used): `chmod 750`, owned by service user
- [ ] TLS key file (if used): `chmod 600`, owned by service user

### 7.3 `rc.d` service is configured

- [ ] Service runs as a dedicated low-privilege user (`_http_smtp_rele`)
- [ ] `rc.conf.local` has `http_smtp_rele=YES`
- [ ] Service starts on boot and restarts on failure

### 7.4 Unveil paths are reviewed

Expected unveiled paths (verify with `ktrace` or source review):

| Path | Permission | When |
|------|-----------|------|
| Config file | `r` | Always |
| SQLite db file | `rwc` | `store = "sqlite"` |
| TLS cert | `r` | `--features tls` + cert configured |
| TLS key | `r` | `--features tls` + key configured |
| Sendmail binary | `x` | `smtp.mode = "pipe"` |

---

## 8. Monitoring and Alerting

### 8.1 Prometheus scrape is configured

- [ ] `/metrics` is scraped by your monitoring system
- [ ] Scrape interval is ≤ 60 s
- [ ] `/metrics` endpoint is network-restricted (not public)

### 8.2 Alerts are defined for key failure modes

Define alerts on the following:

| Metric | Condition | Meaning |
|--------|-----------|---------|
| `rele_auth_failures_total` rate | > X/min sustained | Credential brute-force or misconfigured client |
| `rele_rate_limited_total` rate | Spike > baseline | Abuse or misconfigured client |
| `rele_smtp_submissions_total{result="error"}` | > 0 sustained | SMTP server issue |
| `rele_requests_total{status="5xx"}` | > 0 | Internal errors |

### 8.3 Health checks are integrated

- [ ] Load balancer polls `GET /healthz` (liveness)
- [ ] Monitoring polls `GET /readyz` and alerts on non-200 (SMTP unreachable)

---

## 9. Operations and Incident Response

### 9.1 Config backup

- [ ] The config file is version-controlled or backed up
- [ ] API key secrets are stored in a secrets manager (not only in the config file)
- [ ] Backup includes: config, TLS cert/key, SQLite DB (if used)

### 9.2 Incident response is documented

Document the following in your runbook:

- [ ] How to immediately disable a compromised API key (`enabled = false` + SIGHUP or restart)
- [ ] How to block an abusive IP at the firewall or proxy
- [ ] How to inspect recent audit events in the structured log
- [ ] Who to contact if SMTP delivery problems are suspected

### 9.3 SIGHUP reload is tested

- [ ] SIGHUP causes config to reload without restart:
  ```sh
  kill -HUP $(cat /var/run/http-smtp-rele.pid)
  ```
- [ ] Reload is confirmed in the log:
  ```
  INFO event=config_reloaded
  ```
- [ ] On OpenBSD: SIGHUP reload is working (requires `rpath` in pledge — default since v0.11)

---

## Sign-off

| Category | Reviewer | Date | Notes |
|----------|----------|------|-------|
| Authentication | | | |
| Recipient policy | | | |
| Rate limiting | | | |
| Network exposure | | | |
| TLS | | | |
| Logging / privacy | | | |
| OpenBSD hardening | | | |
| Monitoring | | | |
| Operations | | | |
