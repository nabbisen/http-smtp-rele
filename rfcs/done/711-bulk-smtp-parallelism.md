# RFC 711 — Bulk SMTP Parallelism

**Status.** Proposed  
**Tracks.** T4 — SMTP / Delivery  
**Touches.** `src/api/send_bulk.rs`, `src/config.rs`

## Summary

Replace the sequential SMTP submission loop in `POST /v1/send-bulk` with
two-phase processing: sequential validation followed by bounded-parallel
SMTP submission using `tokio::task::JoinSet` and `tokio::sync::Semaphore`.

## Two-phase design

```
Phase 1 (sequential):
  for each message:
    rate_limit_check()    ← order matters for fairness
    validate()            ← fast, CPU-bound
    build_message()       ← fast, CPU-bound
    → PreparedMessage or rejected result

Phase 2 (parallel, bounded by semaphore):
  JoinSet::spawn per PreparedMessage:
    acquire semaphore permit
    smtp::submit()        ← I/O-bound; parallelised
    update status_store()

merge phase1_rejections + phase2_results → sort by index → response
```

Rate limit checks remain sequential; only the SMTP I/O is parallelised.

## Configuration

```toml
[smtp]
bulk_concurrency = 5   # 0 = unlimited; default 5
```

SIGHUP-reloadable: no (restart required — changing concurrency mid-flight is complex).

## Result ordering

Results in the response body are always in request index order, regardless of
SMTP completion order. Achieved by storing `index` in each result and sorting
before serialisation.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-711-01 | Bulk of N messages issues at most `bulk_concurrency` simultaneous SMTP connections. |
| AC-711-02 | Response `results[i].index == i` always. |
| AC-711-03 | Phase 1 rejection (validation fail) does not affect Phase 2 submissions. |
| AC-711-04 | All existing bulk tests pass unchanged. |
