# RFC 500 — v0.5 Development Plan

**Status.** Proposed  
**Tracks.** Governance

## Theme: Architecture Maturity and API Completion

v0.5 completes the initial design surface and matures the project structure:

| RFC | Feature | Rationale |
|-----|---------|-----------|
| 501 | Cargo workspace split | Clean lib/bin separation; library embeddable |
| 502 | Attachment support | Completes the mail content model |
| 503 | `reply_to` array | API consistency with `to`/`cc` (RFC 302/404) |
| 504 | Prometheus: full instrumentation | Auth/rate/validation counters wired |

## Implementation order

1. RFC 501 — Workspace split (structural; no logic changes)
2. RFC 503 — reply_to array (small; builds on Recipients pattern)
3. RFC 504 — Prometheus full instrumentation (wires existing metrics)
4. RFC 502 — Attachment support (largest; new dep, new validation)

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-500-01 | `cargo build` succeeds from workspace root. |
| AC-500-02 | Binary crate produces the `http-smtp-rele` executable. |
| AC-500-03 | All existing tests pass unchanged. |
| AC-500-04 | Attachments appear in SMTP DATA output from the stub. |
| AC-500-05 | Auth/rate metrics increment correctly in integration tests. |
