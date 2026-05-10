# RFC 834 — /v1/keys/self Information Exposure Control

**Status.** Proposed  
**Tracks.** T1 — Security / T2 — HTTP API  
**Touches.** `src/api/keys.rs`, `src/config.rs`

## Problem

`GET /v1/keys/self` returns the authenticated key's full policy:

```json
{
  "id": "service-a",
  "allowed_recipient_domains": ["example.com", "partner.org"],
  "allowed_recipients": ["admin@example.com"],
  "rate_limit_per_min": 30,
  "mask_recipient": false
}
```

If a token is compromised, an attacker learns the relay's full recipient
allowlist and rate policy. `allowed_recipients` in particular reveals
specific addressable targets.

## Decision

Add a config flag to control endpoint availability:

```toml
[server]
enable_keys_self_endpoint = true   # default: true (preserves current behaviour)
```

And limit the default response to non-sensitive fields:

```json
{
  "id": "service-a",
  "enabled": true
}
```

Sensitive fields (`allowed_recipient_domains`, `allowed_recipients`,
`rate_limit_per_min`) are excluded from the default response. Operators
who need these for client self-inspection can set:

```toml
[server]
keys_self_expose_policy = true   # default: false
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-834-01 | Default response does not include `allowed_recipients`. |
| AC-834-02 | `enable_keys_self_endpoint = false` → endpoint returns 404. |
| AC-834-03 | `keys_self_expose_policy = true` restores the current full response. |
