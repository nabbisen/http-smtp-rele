# RFC 829 — Mutex/RwLock Poison Handling

**Status.** Proposed  
**Tracks.** T7 — Stability  
**Touches.** `src/status_memory.rs`, `src/rate_limit.rs`

## Problem

`InMemoryStatusStore` and the rate limiter use `unwrap()` on lock acquisition:

```rust
self.records.write().unwrap()   // status_memory.rs
self.global.lock().unwrap()     // rate_limit.rs
```

If a thread panics while holding a lock, the lock becomes poisoned. Subsequent
`unwrap()` calls on poisoned locks panic again, cascading until the process
terminates. In `release` profile with `panic = "abort"` the process exits
immediately; under test or library use the panic propagates unexpectedly.

## Decision

### Option A (recommended for v0.15): `parking_lot`

Replace `std::sync::Mutex` and `std::sync::RwLock` with `parking_lot::Mutex`
and `parking_lot::RwLock`, which do not poison on panic.

```toml
parking_lot = "0.12"
```

This eliminates the entire poison class with minimal code change.

### Option B: explicit poison recovery

```rust
self.records.write().unwrap_or_else(|p| p.into_inner())
```

Recovers the lock by consuming the poison, treating the (potentially
inconsistent) state as still usable. Acceptable when corruption is unlikely.

### Document `panic = "abort"`

Regardless of choice, document in `Cargo.toml` that release builds use
`panic = "abort"`, which makes in-process recovery moot for library consumers.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-829-01 | No `unwrap()` on `Mutex`/`RwLock` lock acquisition in production code. |
| AC-829-02 | A panicking thread does not cause the next request to panic. |
