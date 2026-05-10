# RFC 087 — In-memory Submission Status Store

**Status.** Proposed  
**Tracks.** T5 — Abuse / Audit  
**Touches.** `src/status_memory.rs`, `src/config.rs`, `src/lib.rs`, `crates/cli/src/main.rs`

## Summary

This RFC implements the MVP status store as an in-memory, metadata-only, TTL-bound store.
It is intentionally non-durable. Restarting http-smtp-rele clears all status records.
This keeps the MVP simple, avoids storing mail content, and preserves strong OpenBSD hardening.

## Configuration

```toml
[status]
enabled                  = true
store                    = "memory"
ttl_seconds              = 3600
max_records              = 10000
cleanup_interval_seconds = 60
```

SIGHUP-reloadable: `ttl_seconds`, `max_records`, `cleanup_interval_seconds`.  
Restart required: `enabled`, `store`.

## Implementation

```rust
pub struct InMemoryStatusStore {
    records:       RwLock<HashMap<String, SubmissionStatusRecord>>,
    config:        ArcSwap<StatusConfig>,
    expired_total: AtomicU64,
}
```

## TTL cleanup: hybrid strategy

**Lazy expiry** — `get()` checks `expires_at`; if expired, deletes and returns 404.  
**Periodic cleanup** — background tokio task runs every `cleanup_interval_seconds`.

Cleanup order when over `max_records`:
1. Delete expired records first.
2. If still over limit, evict oldest by `created_at`.

## Lifecycle

- Background task spawned at startup only when `enabled = true`.
- Task stops on graceful shutdown.
- Cleanup failures are logged; they do not affect the send API.
- On restart, all records are lost (documented behaviour).

## OpenBSD pledge

In-memory store requires no additional pledge promises or unveil paths.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-087-01 | TTL cleanup removes expired records. |
| AC-087-02 | `max_records` prevents unbounded growth. |
| AC-087-03 | `reload_config()` applies updated TTL/max_records. |
| AC-087-04 | Restart clears all records (documented). |
| AC-087-05 | Records are metadata-only. |
