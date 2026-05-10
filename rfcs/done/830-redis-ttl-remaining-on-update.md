# RFC 830 — Redis TTL: Use Remaining TTL on Update, Not Full Reset

**Status.** Proposed  
**Tracks.** T5 — Status Tracking  
**Touches.** `src/status_redis.rs`

## Problem

`update_status()` calls `SET EX ttl`, resetting the TTL to the full
`ttl_seconds` on every status transition:

```rust
conn.set_ex::<_, _, ()>(&k, &json, ttl)  // ttl = config.ttl_seconds
```

This means a record that transitions `received → smtp_accepted` 3599 seconds
after creation suddenly gets an extra `ttl_seconds` of lifetime, violating
the intended "record expires `ttl_seconds` after creation" contract.

## Fix

Compute the remaining TTL from `expires_at`:

```rust
let remaining_secs = record.expires_at
    .signed_duration_since(Utc::now())
    .num_seconds()
    .max(1) as u64;

conn.set_ex::<_, _, ()>(&k, &json, remaining_secs)?;
```

For Redis ≥ 7.4: use `SET … KEEPTTL` to preserve the existing TTL
without recalculating. Fall back to the remaining-seconds approach for
older versions.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-830-01 | After `update_status`, the Redis key TTL does not exceed the original `expires_at`. |
| AC-830-02 | Integration test (with `REDIS_TEST_URL`) verifies TTL is not extended. |
