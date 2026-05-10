# RFC 206 — IP Bucket LRU Eviction

**Status.** Implemented  
**Tracks.** Security  
**Touches.** `src/rate_limit.rs`

## Summary

Cap the per-IP `HashMap` at a configurable entry count using LRU eviction to prevent
unbounded memory growth during IP-diversity attacks.

## Design

When the `by_ip` map exceeds `rate_limit.ip_table_size` entries (default: 10 000), evict
the least-recently-used entry before inserting the new one. Use `linked-hash-map` or
a manual LRU structure.

`rate_limit.ip_table_size = 0` disables eviction (unbounded; not recommended).

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-206-01 | Map does not exceed `ip_table_size` entries under flood conditions. |
| AC-206-02 | Evicted IPs get a fresh bucket on next request. |
