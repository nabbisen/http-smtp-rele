# RFC 821 — StatusStore Blocking I/O in Async Handlers

**Status.** Proposed  
**Tracks.** T5 — Status Tracking / Performance  
**Touches.** `src/status.rs`, `src/status_sqlite.rs`, `src/status_redis.rs`

## Problem

`StatusStore` is a synchronous trait. SQLite uses `Mutex<Connection>` and
`rusqlite`, and Redis uses `redis::Commands` (synchronous). Both are called
directly from Axum async handlers, blocking a Tokio worker thread on I/O.

Effects:
- Tokio worker stall during SQLite write lock contention or Redis latency
- Degraded request throughput under concurrent load
- Redis latency spikes cause HTTP latency spikes

In-memory store is unaffected (no I/O).

## Decision

### Short-term (v0.15): spawn_blocking for SQLite

Wrap SQLite operations in `tokio::task::spawn_blocking`. The synchronous trait
is retained; the store implementation calls `Handle::current().block_on(spawn_blocking(...))`.

The simpler, safer approach is to have the store acquire the `Mutex` and perform
I/O inside a `spawn_blocking` closure launched by the caller. Since the trait
is sync, the bridge is unavoidable without a trait change.

### Medium-term (RFC 839): async StatusStore trait

A future RFC will make `StatusStore` async, allowing proper async SQLite
(via `tokio-rusqlite`) and async Redis (via `redis::aio`).

### Redis: mark as experimental

Document that `store = "redis"` with sync client has blocking I/O implications
and should not be used in high-throughput production until RFC 839 is complete.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-821-01 | SQLite store operations do not block the Tokio executor thread. |
| AC-821-02 | docs warn that Redis store is experimental pending async migration. |
| AC-821-03 | In-memory store behavior unchanged. |
