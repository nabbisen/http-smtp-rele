# Security

## Open Relay Prevention

`http-smtp-rele` is designed to be an authenticated, domain-restricted relay — never an
open relay.

**Three layers of protection:**

1. **Authentication required.** Every submission request must present a valid API key.
   There is no anonymous submission path. A server with `require_auth = true` is the only
   supported configuration.

2. **`From` address is config-controlled.** Clients cannot set the `From` address. It is
   always taken from `mail.default_from`. This prevents impersonation of arbitrary senders.

3. **Recipient domain allowlist.** `mail.allowed_recipient_domains` restricts which domains
   the relay will deliver to. Leaving this empty creates an open relay — always set it in
   production.

---

## API Key Management

### Generating secrets

Use `openssl` to generate a cryptographically random secret:

```sh
openssl rand -base64 32
```

Store the result as `secret` in the `[[api_keys]]` section.

### Key rotation (zero downtime)

1. Add a new key entry with the new secret.
2. Deploy and restart the relay.
3. Update clients to use the new secret.
4. Set `enabled = false` on the old key entry.
5. Deploy and restart again.

### Key revocation

Set `enabled = false` for the key and restart. The key is checked but always rejected.
The revoked key remains in the config for audit purposes; remove it at the next planned
maintenance window.

### Per-key rate limits

Restrict high-risk integrations:
```toml
[[api_keys]]
id     = "untrusted-integration"
secret = "..."
enabled = true
rate_limit_per_min = 5
```

---

## Header Injection Protection

All string fields that appear in email headers (`to`, `subject`, `from_name`, `reply_to`)
are checked for CR (`\r`) and LF (`\n`) characters before mail construction. If detected,
the request is **rejected** with `400 validation_failed`. The attempt is logged as a
`header_injection_attempt` audit event.

Stripping (silently removing) the characters is intentionally not done — rejection makes
the attack visible.

---

## Timing Attack Resistance

API key comparison uses `subtle::ConstantTimeEq` from the `subtle` crate, which performs
byte-by-byte comparison in time that is independent of where the first differing byte
occurs. The authentication loop always iterates over **all** configured keys without
early termination, preventing enumeration of key position via response time.

---

## Logging and Privacy

### What is always logged

- `request_id`, `client_ip`, `key_id` (per-request)
- Auth failures with reason
- Rate limit events with tier
- Validation failures with field name (not value)
- SMTP submission result (domain only by default)

### What is never logged

- API key secrets (`SecretString` has a redacted `Debug` implementation)
- Request body content (excluded from tracing spans via `skip(payload)`)
- Bearer tokens from request headers
- Raw SMTP session transcripts

### Recipient masking

With `mail.mask_recipient = true` (the default), only the domain portion of the
recipient address is logged (e.g., `example.com` not `alice@example.com`).

---

## Rate Limiting

Three tiers are applied in order:

1. **Global** — all requests combined (before authentication)
2. **Per-IP** — by resolved client IP (before authentication)
3. **Per-key** — after authentication, by `key_id`

### Known limitations

- **In-memory only.** State resets on process restart. A restart allows a burst of up to
  `burst_size` tokens for every client immediately.
- **No distributed state.** Multiple relay instances do not share rate limit counters.
- **No automatic IP blocking.** Exceeded rate limits return 429; the IP is not blocked.

### Abuse response playbook

| Observation | Action |
|------------|--------|
| Auth failures from one IP | Block IP at firewall or reverse proxy |
| Key causing repeated 429s | Check if the key is legitimate; disable if not |
| `header_injection_attempt` logs | Block IP at firewall |
| Global limit hit during normal traffic | Raise `global_per_min`; investigate source |

---

## Reverse Proxy Guidance

### TLS

The relay itself does not terminate TLS. Place a TLS-terminating reverse proxy (nginx,
Caddy, relayd on OpenBSD) in front. The relay listens on loopback (`127.0.0.1`) and
the proxy handles external HTTPS.

### Restrict `/readyz`

`/readyz` reveals whether your SMTP server is reachable. Restrict external access:

**nginx:**
```nginx
location /readyz {
    allow 10.0.0.0/8;   # internal monitoring only
    deny all;
}
```

**relayd (OpenBSD):**
```
filter "block_readyz" { block }
```

### Trusted proxy headers

If the relay is behind a proxy and you use IP-based rate limiting or allowlisting, configure
both lists:

```toml
[security]
# Trust X-Forwarded-For from the local proxy only
trust_proxy_headers  = true
trusted_source_cidrs = ["127.0.0.1/32"]

# Optionally restrict which resolved client IPs may connect at all
# allowed_source_cidrs = ["10.0.0.0/8"]
```

If `trust_proxy_headers = true` and the peer IP is in `trusted_source_cidrs`,
`http-smtp-rele` uses `X-Forwarded-For` to resolve the client IP. Otherwise proxy
headers are ignored. `allowed_source_cidrs` controls which *resolved* client IPs are
permitted to proceed — it is independent of proxy header trust.

---

## OpenBSD Hardening

On OpenBSD, `pledge("stdio inet")` and `unveil(NULL, NULL)` are applied after config is
loaded. See [openbsd.md](openbsd.md) for details.
