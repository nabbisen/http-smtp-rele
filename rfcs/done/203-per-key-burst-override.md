# RFC 203 — Per-Key Burst Override

**Status.** Implemented  
**Tracks.** Security / Config  
**Touches.** `src/config.rs`, `src/rate_limit.rs`

## Summary

Add `burst: u32` to `ApiKeyConfig` so individual keys can override the default per-key
burst capacity.

## Motivation

Trusted internal services that send in bursts need a larger burst capacity than the global
default, without changing the sustained rate. Confirmed as v0.2 scope by architect review.

## Design

```toml
[[api_keys]]
id     = "batch-service"
secret = "..."
enabled = true
rate_limit_per_min = 120
burst  = 30   # new field; 0 = inherit per_key_burst from [rate_limit]
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-203-01 | `ApiKeyConfig.burst > 0` overrides `per_key_burst` for that key. |
| AC-203-02 | `ApiKeyConfig.burst = 0` (or absent) inherits `per_key_burst`. |
