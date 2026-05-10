# RFC 200 — v0.2 Development Plan

**Status.** Implemented  
**Tracks.** Governance  
**Touches.** All

## Summary

Define the scope, RFC assignments, and milestone structure for the v0.2 release.

## Scope

v0.2 adds two categories of work:

### Category A — Test infrastructure (deferred from v0.1)

RFC 100–103 were designed in v0.1 but not implemented. They are the first priority
in v0.2 because they unlock reliable end-to-end testing for all Category B features.

| RFC | Title |
|-----|-------|
| 100 | Integration Test Harness |
| 101 | SMTP Stub Server |
| 102 | Security Regression Test Suite (complete SEC-013, SEC-015, SEC-016) |
| 103 | E2E Test Scenarios |

### Category B — Architect-confirmed feature additions

Confirmed during v0.1 architecture review. New RFCs 201–206.

| RFC | Feature |
|-----|---------|
| 201 | Per-tier burst configuration (`global_burst`, `per_ip_burst`, `per_key_burst`) |
| 202 | Default per-key rate limit in `[rate_limit]` (`per_key_per_min`) |
| 203 | Per-key burst override (`ApiKeyConfig.burst`) |
| 204 | Per-address recipient allowlist (`ApiKeyConfig.allowed_recipients`) |
| 205 | Concurrency limit (`server.concurrency_limit`) |
| 206 | IP bucket LRU eviction |

### Category C — Original v0.2 items (subset for this release)

| RFC | Feature |
|-----|---------|
| 210 | Sendmail pipe mode (`smtp.mode = "pipe"`) — implements RFC 064 |
| 211 | Signal-based config reload (`SIGHUP`) |

Items deferred to v0.3: SMTP AUTH, W3C Forwarded header, HTML body, multi-recipient.

## Implementation order

```
1. RFC 101 — SMTP Stub Server          (enables all E2E tests)
2. RFC 100 — Integration Test Harness  (test scaffolding)
3. RFC 102 — SEC-013, 015, 016         (complete security test matrix)
4. RFC 103 — E2E Scenarios             (pipeline correctness proof)
5. RFC 201 — Per-tier burst config     (config schema first; rate_limit uses it)
6. RFC 202 — Default per-key rate      (completes rate_limit config)
7. RFC 203 — Per-key burst override    (ApiKeyConfig extension)
8. RFC 205 — Concurrency limit         (independent Tower layer)
9. RFC 206 — IP bucket LRU eviction    (rate_limit.rs internal)
10. RFC 204 — Per-address allowlist    (validation pipeline)
11. RFC 210 — Sendmail pipe mode       (smtp.rs; security.rs pledge update)
12. RFC 211 — SIGHUP reload            (main.rs, signal handling)
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-200-01 | All SEC-001 through SEC-017 tests pass. |
| AC-200-02 | E2E tests pass with SMTP stub (full pipeline: HTTP → auth → validation → SMTP). |
| AC-200-03 | Per-tier burst and per-key rate config are TOML-documented and tested. |
| AC-200-04 | `cargo test` passes with 0 failures. |
| AC-200-05 | `make gate` passes. |

## Open Questions

None.
