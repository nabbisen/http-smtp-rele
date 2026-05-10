# RFC 202 — Default Per-Key Rate in [rate_limit]

**Status.** Implemented  
**Tracks.** Security / Config  
**Touches.** `src/config.rs`, `src/rate_limit.rs`

## Summary

Add `per_key_per_min` to `RateLimitConfig` as the default rate for API keys that do not
set their own `rate_limit_per_min`. Avoids duplicating the same value across every key.

## Motivation

In v0.1, per-key rate inherits `per_ip_per_min` when `ApiKeyConfig.rate_limit_per_min` is
absent — which is semantically wrong. Confirmed as v0.2 scope by architect review.

## Design

```toml
[rate_limit]
per_key_per_min = 30  # new field; default 30
```

`RateLimiter::check_key` uses `per_key_per_min` as the default, not `per_ip_per_min`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-202-01 | `[rate_limit].per_key_per_min` sets the default per-key rate. |
| AC-202-02 | `ApiKeyConfig.rate_limit_per_min` overrides the default when set. |
