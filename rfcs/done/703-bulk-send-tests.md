# RFC 703 — Bulk Send Integration Tests

**Status.** Proposed  
**Tracks.** T6 — Testing  
**Touches.** `tests/integration_tests.rs`

## Test cases

| ID | Scenario | Expected |
|----|----------|----------|
| BULK-001 | Two valid messages → 202, both accepted | `accepted = 2` |
| BULK-002 | One valid, one invalid → 202, mixed results | `accepted = 1, rejected = 1` |
| BULK-003 | Empty messages array → 400 | `bad_request` |
| BULK-004 | Exceeds max_bulk_messages → 400 | `payload_too_large` |
| BULK-005 | Each message has unique request_id | all distinct |
| BULK-006 | Per-message request_id queryable via GET /v1/submissions/ | 200 + correct status |
| BULK-007 | Unauthenticated request → 401 | no messages processed |
| BULK-008 | Response contains no mail body / full addresses | assert absent |
| BULK-009 | `bulk_request_id` present in response | starts with `req_` |
