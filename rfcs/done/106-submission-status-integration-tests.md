# RFC 106 — Submission Status API Integration Tests

**Status.** Proposed  
**Tracks.** T6 — Testing  
**Touches.** `tests/integration_tests.rs`

## Summary

Integration tests covering the full Submission Status API pipeline.

## Required test cases

| ID | Scenario | Expected |
|----|----------|----------|
| STS-001 | Valid send → `GET` same key | `smtp_accepted` |
| STS-002 | Validation error after auth → status | `rejected / validation_failed` |
| STS-003 | Rate limit after auth → status | `rejected / rate_limited` |
| STS-004 | SMTP unavailable → status | `smtp_failed / smtp_unavailable` |
| STS-005 | Different API key → `GET` | 404 `submission_not_found` |
| STS-006 | Unknown `request_id` → `GET` | 404 `submission_not_found` |
| STS-007 | Response contains no body/subject/token/full-address | assert absent |
| STS-008 | `request_id` in response matches `X-Request-Id` header | exact match |
| STS-009 | `enabled = false` → `GET` always 404 | 404 |
| STS-010 | `max_records` eviction under load | store size bounded |

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-106-01 | All STS-001 through STS-010 exist and pass. |
| AC-106-02 | Tests run under `cargo test --test integration_tests`. |
