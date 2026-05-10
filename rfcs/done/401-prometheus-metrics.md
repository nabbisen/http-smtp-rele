# RFC 401 — Prometheus /metrics Endpoint

**Status.** Implemented  
**Tracks.** Ops

## Summary

Add `GET /metrics` returning Prometheus text format with counters and histograms for
requests, SMTP submissions, auth failures, and rate limit events.

## Metrics

| Name | Type | Labels | Description |
|------|------|--------|-------------|
| `rele_requests_total` | Counter | `status` | Total HTTP requests by response status class |
| `rele_smtp_submissions_total` | Counter | `result` (ok/err) | SMTP submissions attempted |
| `rele_smtp_duration_seconds` | Histogram | — | SMTP session duration |
| `rele_auth_failures_total` | Counter | `reason` | Auth failures by reason |
| `rele_rate_limited_total` | Counter | `tier` | Rate limit hits by tier |
| `rele_validation_failures_total` | Counter | — | Validation failures |

## Design

Use the `prometheus` crate. Register metrics in a `Registry` stored in `AppState`.
The `/metrics` handler calls `prometheus::TextEncoder` to serialize.

Access restriction: document that `/metrics` should be restricted at the proxy layer
(same guidance as `/readyz`).

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-401-01 | `GET /metrics` returns `200 OK` with `text/plain; version=0.0.4` content-type. |
| AC-401-02 | `rele_requests_total` increments on each request. |
| AC-401-03 | `rele_smtp_submissions_total{result="ok"}` increments on successful send. |
