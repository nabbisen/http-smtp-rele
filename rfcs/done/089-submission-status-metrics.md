# RFC 089 — Submission Status Store Metrics

**Status.** Proposed  
**Tracks.** T5 — Abuse / Audit  
**Touches.** `src/metrics.rs`, `src/status_memory.rs`

## Summary

Add Prometheus metrics for the submission status store.
This RFC is separate from RFC 086/087 so that store correctness and security are
defined independently of observability instrumentation.

## Metrics

### `rele_status_store_records_current` (Gauge)

Current number of records held in the store.

### `rele_status_store_transitions_total{status, code}` (Counter)

Cumulative count of status updates.  
`status`: `received | rejected | smtp_submission_started | smtp_accepted | smtp_failed`  
`code`: low-cardinality enum only (`none | validation_failed | rate_limited | smtp_unavailable | ...`)

### `rele_status_store_expired_total` (Counter)

Records removed by TTL expiration (lazy + periodic combined).

## Label policy

The following must NOT be used as Prometheus labels (high cardinality):
`request_id`, `key_id`, `recipient_domain`, `client_ip`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-089-01 | `records_current` gauge reflects live store size. |
| AC-089-02 | `transitions_total` increments on each status update. |
| AC-089-03 | `expired_total` increments on TTL deletion. |
| AC-089-04 | No high-cardinality labels. |
