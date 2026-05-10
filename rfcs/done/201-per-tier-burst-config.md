# RFC 201 — Per-Tier Burst Configuration

**Status.** Implemented  
**Tracks.** Security / Config  
**Touches.** `src/config.rs`, `src/rate_limit.rs`, `docs/configuration.md`

## Summary

Add separate burst settings for each rate limit tier (`global_burst`, `per_ip_burst`,
`per_key_burst`) so operators can tune burst tolerance independently per tier, rather than
sharing a single `burst_size`.

## Motivation

The shared `burst_size` from v0.1 prevents fine-grained control. A legitimate internal
service may need a large per-key burst for batch processing, while the global and IP tiers
should remain tight. Confirmed as v0.2 scope by architect review (D-05).

## Design

### `RateLimitConfig`

```toml
[rate_limit]
global_per_min  = 60
global_burst    = 10
per_ip_per_min  = 20
per_ip_burst    = 5
# Default for per-key tier when ApiKeyConfig.burst is not set
per_key_per_min = 30
per_key_burst   = 5
```

Migration: `burst_size` is deprecated in favour of the per-tier values.
A `burst_size` key still parses and sets all three burst fields to the same value,
with a startup warning.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-201-01 | `[rate_limit]` accepts `global_burst`, `per_ip_burst`, `per_key_burst`. |
| AC-201-02 | Absent fields fall back to individual defaults. |
| AC-201-03 | Legacy `burst_size` still parses and emits a deprecation warning. |
