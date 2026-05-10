# RFC 817 — M-05: HTTP 400 / 422 Status Code Unification

**Status.** Proposed  
**Tracks.** T2 — HTTP API  
**Touches.** `src/error.rs`, `docs/api.md`, `docs/src/guides/api-reference.md`

## Problem

`AppError::Validation` maps to `422 Unprocessable Entity`, but some API
documentation shows `400 Bad Request` for validation failures. The contract
is inconsistent between docs and implementation.

## Decision

Adopt the semantic split:

| Scenario | HTTP status |
|----------|------------|
| Malformed JSON / unknown field / parse failure | 400 Bad Request |
| Semantic validation failure (domain not allowed, subject too long, etc.) | 422 Unprocessable Entity |

This requires:
1. A separate `AppError::BadRequestParsing` or re-use of `AppError::BadRequest`
   for deserialization failures after the RFC 805 raw-body change.
2. `AppError::Validation` remains 422.
3. Docs updated to match.

If this split is too complex, a simpler alternative: map all validation
failures to `400 Bad Request` and document it clearly. Either is acceptable
as long as docs and implementation agree.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-817-01 | Docs and implementation agree on status code for each error case. |
| AC-817-02 | No test expects 400 but receives 422 or vice versa. |
| AC-817-03 | The chosen scheme is documented in the API reference. |
