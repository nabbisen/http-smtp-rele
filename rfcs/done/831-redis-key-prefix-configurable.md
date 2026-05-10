# RFC 831 — Redis Key Prefix Configurable

**Status.** Proposed  
**Tracks.** T5 — Status Tracking  
**Touches.** `src/status_redis.rs`, `src/config.rs`

## Problem

The Redis key prefix is hardcoded:

```rust
const KEY_PREFIX: &str = "rele:s:";
```

Deployments sharing a Redis instance across environments (staging, prod) or
across multiple relay instances with distinct namespaces will collide.

## Fix

Add a config field with a sane default:

```toml
[status]
# Key prefix for Redis entries. Change when sharing a Redis DB across deployments.
redis_key_prefix = "rele:s:"
```

Validation: must not be empty; must not contain whitespace.

`RedisStatusStore` reads the prefix at construction time and stores it as
a field. All key-building calls use `format!("{}{}",self.prefix, id)`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-831-01 | Two stores with different `redis_key_prefix` do not share keys. |
| AC-831-02 | Empty `redis_key_prefix` is rejected by config validation. |
| AC-831-03 | Default prefix is `"rele:s:"` for backward compatibility. |
