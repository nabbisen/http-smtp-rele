# RFC 839 — StatusStore Async Trait (Long-term)

**Status.** Proposed  
**Tracks.** T8 — Extensibility  
**Touches.** `src/status.rs`, all StatusStore implementations

## Background

RFC 821 addresses the immediate concern (blocking I/O in async handlers) via
`spawn_blocking`. This RFC specifies the long-term target.

## Problem

The current `StatusStore` trait is synchronous. Every I/O backend must bridge
sync → async through `spawn_blocking` or similar, adding latency and
complexity that is invisible to the trait.

## Target design (Rust 2024 + async trait)

```rust
pub trait StatusStore: Send + Sync {
    async fn put(&self, record: SubmissionStatusRecord);
    async fn update_status(&self, request_id: &RequestId, key_id: &str, update: StatusUpdate);
    async fn get(&self, request_id: &RequestId, key_id: &str)
        -> Result<Option<SubmissionStatusRecord>, StatusStoreError>;
    async fn expire_old_records(&self);
    async fn record_count(&self) -> usize;
    fn reload_config(&self, config: &StatusConfig);
}
```

Rust 2024 edition supports async fn in traits natively (no `async-trait` crate
needed). All call sites in handlers are already `async`, so `await` is trivial.

## Migration plan

1. RFC 821 wraps SQLite with `spawn_blocking` (sync trait retained)
2. This RFC adds `async` keyword to the trait and all implementations
3. `InMemoryStatusStore` wraps trivial operations in `async` blocks
4. `SqliteStatusStore` uses `tokio-rusqlite` or a dedicated worker task
5. `RedisStatusStore` uses `redis::aio::ConnectionManager`

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-839-01 | `StatusStore` trait methods are `async`. |
| AC-839-02 | SQLite implementation does not block Tokio worker threads. |
| AC-839-03 | Redis implementation uses an async connection. |
| AC-839-04 | All tests pass. |
