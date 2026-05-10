# RFC 814 — M-02: StatusStore Backend Error Observability

**Status.** Proposed  
**Tracks.** T5 — Status Tracking  
**Touches.** `src/status_sqlite.rs`, `src/status_redis.rs`, `src/metrics.rs`

## Problem

`StatusStore::get()` returns `None` on both "record not found" and "backend
error". Callers (and the HTTP API) cannot distinguish between a normal 404
and a database failure. This makes operational diagnosis difficult.

## Fix

Minimum viable fix: Log backend errors at `ERROR` level with a structured
field `event = "status_store_error"`, distinct from the expected not-found
path. Also increment a new Prometheus counter:

```
rele_status_store_errors_total{operation="get"|"put"|"update"}
```

This does not change the API response (still 404) but makes failures visible
in metrics and logs.

Long-term (future RFC): Change `get()` to `Result<Option<...>, StatusStoreError>`
so callers can return 503 instead of 404 on backend error.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-814-01 | Backend errors are logged at ERROR with `event=status_store_error`. |
| AC-814-02 | `rele_status_store_errors_total` counter is incremented on backend error. |
| AC-814-03 | Not-found path does not log an error. |
