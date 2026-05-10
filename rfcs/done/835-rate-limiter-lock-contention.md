# RFC 835 — Rate Limiter Global Lock: Documentation and Improvement Path

**Status.** Proposed  
**Tracks.** T7 — Performance  
**Touches.** `src/rate_limit.rs`, docs

## Problem

Every `/v1/send` and `/v1/send-bulk` message acquires a `Mutex` on the
global token bucket:

```rust
self.global.lock().unwrap().try_consume()
```

Under high concurrency, all requests serialize at this lock. The critical
section is tiny (token arithmetic), so in practice contention is low.
However, it is a single bottleneck with no sharding.

IP and per-key buckets use a `RwLock<HashMap<...>>` which also serializes
writes when a new IP/key is first seen.

## Decision

### v0.15: Document the limitation

Add a note in `docs/src/guides/configuration.md` and the architecture docs:

> The rate limiter uses in-process mutex-based token buckets. All rate limit
> state is lost on restart. For high-concurrency deployments (thousands of
> requests/second), consider sharded or external rate limiting.

### Future (separate RFC): replace with `governor` or atomic-based limiter

The `governor` crate provides a `GCRA`-based, `DashMap`-backed limiter
that eliminates the global mutex bottleneck.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-835-01 | Rate limiter limitations are documented in configuration reference. |
| AC-835-02 | No behaviour change in this RFC. |
