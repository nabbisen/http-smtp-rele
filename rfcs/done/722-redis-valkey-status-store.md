# RFC 722 — Redis/Valkey Shared Status Store

**Status.** Proposed  
**Tracks.** T5 — Abuse / Audit  
**Touches.** `src/status_redis.rs`, `src/config.rs`, `src/lib.rs`, `Cargo.toml`

## Summary

Implement a Redis/Valkey-backed `StatusStore` as an optional Cargo feature.
Enables multi-instance deployments where all instances share a common status
view. Records stored as JSON with native TTL (`EXPIRE`).

## Feature flag

```
cargo build --release --features redis
```

## Configuration

```toml
[status]
store     = "redis"
redis_url = "redis://127.0.0.1:6379/0"
# or: redis_url = "redis+unix:///var/run/redis/redis.sock?db=0"
```

`redis_url` required when `store = "redis"`. Restart required to change.

## Key schema

```
Key:   rele:s:{request_id}
Value: JSON of SubmissionStatusRecord
TTL:   set to ttl_seconds on PUT; refreshed on UPDATE
```

Prefix `rele:s:` is short (6 chars) to minimise memory overhead.

## Degraded-mode behaviour

Redis unavailability (connection failure, timeout) does not fail mail delivery.

| Operation | On Redis error |
|-----------|---------------|
| `put()` | log WARN, discard |
| `update_status()` | log WARN, discard |
| `get()` | log WARN, return None (→ 404) |
| `expire_old_records()` | no-op (Redis TTL handles expiry) |

## Differences from in-memory and SQLite stores

| Capability | memory | sqlite | redis |
|-----------|--------|--------|-------|
| Survives restart | no | yes | yes |
| Multi-instance | no | no | yes |
| `max_records` enforcement | yes | yes | no (use `maxmemory-policy`) |
| TTL expiry | lazy+background | lazy+background | native EXPIRE |
| Background cleanup task | yes | yes | no (no-op) |

## OpenBSD pledge

Redis uses TCP (`inet` promise). No new pledge promises required.

## Testing

Unit tests cover serialisation round-trips and error handling.
Integration tests require a running Redis and use the `REDIS_TEST_URL`
environment variable; tests are skipped (not failed) when it is unset.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-722-01 | `store = "redis"` persists and retrieves records. |
| AC-722-02 | Records expire via Redis TTL. |
| AC-722-03 | Redis unavailability: mail delivery succeeds, status returns 404. |
| AC-722-04 | Non-redis build rejects `store = "redis"` at startup. |
| AC-722-05 | Missing `redis_url` with `store = "redis"` fails startup. |
