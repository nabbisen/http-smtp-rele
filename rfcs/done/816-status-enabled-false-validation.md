# RFC 816 — M-04: status.enabled=false Skips Backend Config Validation

**Status.** Proposed  
**Tracks.** T2 — Configuration  
**Touches.** `src/config/validate.rs`

## Problem

`validate_config` validates `status.store`, `status.db_path`, and
`status.redis_url` regardless of whether `status.enabled` is `false`.

Setting `status.enabled = false` with `store = "redis"` but no `redis_url`
causes a startup failure even though the store will never be used.

## Fix

Wrap the backend-specific validation in an `enabled` guard:

```rust
if config.status.enabled {
    // validate store, db_path, redis_url
}
```

Other `[status]` validation (range checks, etc.) may continue unconditionally.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-816-01 | `status.enabled = false` with any `store` value starts without error. |
| AC-816-02 | `status.enabled = true` with `store = "sqlite"` and no `db_path` still fails. |
| AC-816-03 | Config validation test covers both cases. |
