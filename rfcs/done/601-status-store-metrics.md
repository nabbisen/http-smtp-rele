# RFC 601 — Status Store Prometheus Metrics

**Status.** Proposed  
**Tracks.** T5 — Ops  
**Touches.** `src/metrics.rs`, `src/status_memory.rs`, `src/lib.rs`

## Summary

Implements the metrics defined in RFC 089.
`Arc<Metrics>` is passed to `InMemoryStatusStore` at construction so the store
can increment counters directly. `NoopStatusStore` receives a no-op path.

## Metrics added

| Name | Type | Labels |
|------|------|--------|
| `rele_status_store_records_current` | Gauge | — |
| `rele_status_store_transitions_total` | Counter | `status`, `code` |
| `rele_status_store_expired_total` | Counter | — |

## Label cardinality

`status`: 5 values (received / rejected / smtp_submission_started / smtp_accepted / smtp_failed)  
`code`: low-cardinality enum — `none` + ErrorCode snake_case values  
Never use: `request_id`, `key_id`, `recipient_domain`, `client_ip`.

## Wiring

- `put()` → increment `transitions_total{status=received, code=none}` and `records_current`
- `update_status()` → increment `transitions_total{status, code}`
- `get()` lazy expiry → increment `expired_total`, decrement `records_current`
- `expire_old_records()` → increment `expired_total` × removed count, update `records_current`

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-601-01 | `GET /metrics` contains `rele_status_store_records_current`. |
| AC-601-02 | `rele_status_store_transitions_total` increments on each status change. |
| AC-601-03 | `rele_status_store_expired_total` increments on TTL deletion. |
| AC-601-04 | No high-cardinality labels. |
