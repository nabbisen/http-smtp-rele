# RFC 504 — Prometheus: Full Instrumentation

**Status.** Proposed  
**Tracks.** Ops

## Summary

Wire auth failure, rate limit, and validation failure counters to their
respective code paths (auth.rs, send.rs) so all six metric families increment.

## Changes

| Location | Event | Metric |
|----------|-------|--------|
| `auth.rs` | Missing `Authorization` header | `rele_auth_failures_total{reason="missing_token"}` |
| `auth.rs` | Unknown / disabled key | `rele_auth_failures_total{reason="invalid_token"}` |
| `send.rs` | Global rate limit hit | `rele_rate_limited_total{tier="global"}` |
| `send.rs` | IP rate limit hit | `rele_rate_limited_total{tier="ip"}` |
| `send.rs` | Key rate limit hit | `rele_rate_limited_total{tier="key"}` |
| `send.rs` | Validation failure | `rele_validation_failures_total{field="<field>"}` |
| `send.rs` | 4xx responses | `rele_requests_total{status="4xx"}` |

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-504-01 | `rele_auth_failures_total` increments on 401/403. |
| AC-504-02 | `rele_rate_limited_total` increments by tier on 429. |
| AC-504-03 | Integration test verifies counter values via `/metrics`. |
