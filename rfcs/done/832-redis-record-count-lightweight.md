# RFC 832 — record_count() Lightweight for Redis

**Status.** Proposed  
**Tracks.** T5 — Status Tracking / Performance  
**Touches.** `src/status_redis.rs`

## Problem

`record_count()` is called by the background metrics task and by Prometheus
scrape. The Redis implementation uses `SCAN` to count all matching keys:

```rust
redis::cmd("SCAN").arg(cursor).arg("MATCH").arg(&pattern).arg("COUNT").arg(100)
```

This is O(N) over all keys in the database. At scale (tens of thousands of
records), each Prometheus scrape triggers a full keyspace scan.

## Fix

Maintain an explicit counter in Redis using `INCR`/`DECR`:

```rust
// put()
conn.incr::<_, _, ()>(&counter_key, 1)?;

// on TTL expiry (lazy, approximation): decrement when key is found missing
```

Because Redis TTL expiry is asynchronous, the counter is an approximation
(best-effort). This is acceptable for gauge metrics.

Alternative: skip `record_count()` for Redis entirely (return 0 / -1 as
sentinel) and rely on the `rele_status_store_transitions_total` counter
for throughput visibility instead.

Document which approach is chosen and its accuracy guarantees.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-832-01 | `record_count()` in Redis does not perform `SCAN` on the full keyspace. |
| AC-832-02 | The implementation's accuracy characteristics are documented. |
