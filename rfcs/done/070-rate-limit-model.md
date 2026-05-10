# RFC 070 — Rate Limit Model

**Status.** Implemented  
**Tracks.** Security  
**Touches.** `src/rate_limit.rs`, `src/config.rs`

## Summary

Define the three-tier rate limit model (global, source IP, API key), the enforcement order,
and the in-memory storage structure. Full token bucket implementation is RFC 071.

## Motivation

Rate limiting is the primary defense against API key leakage being used for spam relay and
against DoS via high-volume requests. Without it, a single compromised key can saturate the
SMTP server (FR-030, FR-031, FR-032, NFR-SEC-001, AC-007).

## Scope

- Three rate limit tiers: global, per-source-IP, per-API-key.
- Enforcement order: global → IP → key (outer to inner).
- `RateLimiter` struct: owns all token buckets.
- In-memory only: state resets on restart (documented limitation).
- Interaction with authentication: IP limit applies before auth; key limit applies after auth.

## Non-goals

- Token bucket algorithm (RFC 071).
- `Retry-After` header (RFC 073).
- Persistent rate limit state (not in MVP).
- Distributed rate limiting (not in MVP).

## Design

### Tiers and enforcement order

```
Request arrives
    │
    ▼
[1] Global limiter          — all requests, regardless of IP or key
    │ 429 if exceeded
    ▼
[2] Source IP limiter       — per resolved client IP
    │ 429 if exceeded
    ▼
[3] Auth middleware          — 401/403 if auth fails
    │
    ▼
[4] API key limiter         — per key_id
    │ 429 if exceeded
    ▼
[5] Validation, policy, SMTP
```

Global and IP limits run before auth because they protect against unauthenticated flood
attacks. Applying auth first would waste CPU on invalid tokens at flood volume.

Key limit runs after auth because it requires knowing which key is in use.

### `RateLimiter`

```rust
pub struct RateLimiter {
    global: Mutex<TokenBucket>,
    by_ip: Mutex<HashMap<IpAddr, TokenBucket>>,
    by_key: Mutex<HashMap<String, TokenBucket>>,  // key: key_id
    ip_config: BucketConfig,
    key_config_default: BucketConfig,
}

impl RateLimiter {
    pub fn new(config: &RateLimitConfig) -> Self { ... }

    /// Check and consume one token from the global bucket.
    pub fn check_global(&self) -> Result<(), RateLimitError> { ... }

    /// Check and consume one token from the IP bucket.
    pub fn check_ip(&self, ip: IpAddr) -> Result<(), RateLimitError> { ... }

    /// Check and consume one token from the key bucket.
    pub fn check_key(&self, key_id: &str, effective: &EffectiveRateLimits) -> Result<(), RateLimitError> { ... }
}
```

`RateLimiter` is constructed once and stored in `AppState` behind `Arc`.

### `RateLimitError`

```rust
pub struct RateLimitError {
    /// Tier that was exceeded: "global", "ip", or "key".
    pub tier: &'static str,
    /// Estimated seconds until next token is available, if calculable.
    pub retry_after_secs: Option<u64>,
}
```

Maps to `AppError::RateLimited { retry_after_secs }`.

### In-memory limitation

Rate limit state lives in memory. On process restart, all buckets are reset. This means a
restart could temporarily allow burst traffic beyond the configured limit.

This limitation is:
1. Documented in `docs/configuration.md`.
2. Acceptable for the expected deployment (low-traffic relay, not high-concurrency SaaS).
3. Mitigated by the global limit, which provides a hard ceiling.

### IP bucket eviction

The `by_ip` map can grow unbounded in a high-IP-diversity attack. An LRU eviction policy is
applied: when the map exceeds a size threshold (e.g., 10,000 entries), oldest entries are
evicted. Evicted entries effectively get a fresh bucket on next access (restart-equivalent).

## Implementation Plan

1. Define `BucketConfig`, `RateLimitError` in `src/rate_limit.rs`.
2. Define `RateLimiter` struct.
3. Implement `check_global`, `check_ip`, `check_key` (stubs; token bucket in RFC 071).
4. Add IP bucket eviction.
5. Wire into the request pipeline in `app.rs`.
6. Write tests.

## Test Plan

### Unit Tests

- `check_global` returns `Ok` when global bucket has tokens.
- `check_global` returns `Err` when global bucket is empty.
- `check_ip` returns `Err` for an IP that has exceeded its limit.
- Two different IPs have independent buckets.
- `check_key` uses per-key config when `effective.per_minute > 0`.
- `check_key` uses global default when `effective.per_minute == 0`.

### Integration Tests

- Global limit exceeded → 429 for any authenticated and unauthenticated request.
- IP limit exceeded → 429 even with valid auth.
- Key limit exceeded → 429 (only for that key; other keys unaffected).
- Rate limits reset after restart (document; not a test assertion).

### Security Tests

- Exceeding key A's limit does not affect key B.
- IP flood is blocked by IP limit before auth is reached.

## Security Considerations

- Rate limiting before auth prevents DoS via unauthenticated high-volume requests.
- The IP limiter is only as strong as the IP resolution (RFC 041). A correctly configured
  trusted proxy setup ensures the real client IP is used.
- The in-memory-only limitation means a restart can temporarily lift rate limits. This is
  documented and accepted.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-070-01 | Global limit exceeded → 429 for any request. |
| AC-070-02 | IP limit exceeded → 429 regardless of auth status. |
| AC-070-03 | Key limit exceeded → 429 for that key only. |
| AC-070-04 | Two keys have independent rate limit buckets. |
| AC-070-05 | In-memory limitation is documented. |

## Open Questions

- LRU threshold value for `by_ip`: default 10,000 entries. Configurable? Decision: not in MVP.
