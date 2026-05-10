# RFC 103 — E2E Test Scenarios

**Status.** Proposed  
**Tracks.** Testing  
**Touches.** `tests/e2e_tests.rs`

## Summary

Define end-to-end scenarios that exercise the complete request pipeline — HTTP → auth →
validation → SMTP stub — and confirm the correct response and audit log events for each.

## Design

### Scenarios

| Scenario | Input | Expected HTTP | Expected log event |
|----------|-------|--------------|-------------------|
| E2E-001 | Valid minimal request, valid auth | 202 accepted | `smtp_submitted` |
| E2E-002 | Valid request, SMTP stub down | 502 smtp_unavailable | `smtp_failure` |
| E2E-003 | Valid request, SMTP stub rejects | 502 smtp_unavailable | `smtp_failure` |
| E2E-004 | `/healthz` while SMTP is down | 200 ok | — |
| E2E-005 | `/readyz` while SMTP is up | 200 ok | — |
| E2E-006 | `/readyz` while SMTP is down | 503 smtp_unavailable | — |
| E2E-007 | Valid request, rate limit not reached | 202 accepted | `smtp_submitted` |
| E2E-008 | Valid request, global rate limit exceeded | 429 rate_limited | `rate_limited tier=global` |
| E2E-009 | Wrong Content-Type | 415 unsupported_media_type | — |
| E2E-010 | Oversized body | 413 payload_too_large | — |
| E2E-011 | request_id in response matches X-Request-Id header | 202 | — |

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-103-01 | All E2E-001 through E2E-011 scenarios exist as tests and pass. |
| AC-103-02 | SMTP stub is used for E2E-001, E2E-002, E2E-003. |
| AC-103-03 | Tests run as part of `cargo test`. |

## Open Questions

None.
