# RFC 806 — RB-06: request_id in All Error Response JSON Bodies

**Status.** Proposed  
**Tracks.** T2 — HTTP API  
**Touches.** `src/error.rs`, `src/api/submissions.rs`, `src/auth.rs`

## Problem

`AppError::into_response()` produces:

```json
{ "status": "error", "code": "...", "message": "..." }
```

There is no `request_id`. Clients cannot correlate an error response with
logs or status store entries using only the body.

`X-Request-Id` header carries the ID, but the API contract expects it in
the body too.

Additionally, `GET /v1/submissions/{request_id}` not-found response contains:

```json
{
  "request_id": "<target being looked up>",
  "lookup_request_id": "<new ID for this GET request>"
}
```

The `lookup_request_id` field does not match the `X-Request-Id` header on the
response, which is confusing.

## Decision

### All error responses include the current request's ID

```json
{
  "status": "error",
  "code": "validation_failed",
  "message": "...",
  "request_id": "req_01HX..."
}
```

`request_id` is the ID of **this request** (the one that returned the error),
matching `X-Request-Id`.

### Status lookup not-found uses distinct field names

```json
{
  "status": "error",
  "code": "submission_not_found",
  "message": "Submission status was not found or has expired.",
  "request_id":        "req_01LOOKUP...",
  "target_request_id": "req_01TARGET..."
}
```

`request_id` = the GET request's own ID (matches `X-Request-Id`).
`target_request_id` = the ID that was looked up and not found.

## Implementation approach

Since `AppError::into_response()` does not have access to the `request_id`
extension, the injection must happen in a middleware layer or in a wrapper:

Option A (recommended): Add an error-enriching middleware that reads the
`RequestId` extension from the response extensions and patches the JSON body.

Option B: Wrap all handler error returns via a helper that builds the enriched
body before returning.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-806-01 | All `AppError` JSON responses contain a `request_id` field. |
| AC-806-02 | `request_id` in the body matches `X-Request-Id` header. |
| AC-806-03 | Status lookup not-found uses `request_id` + `target_request_id`. |
| AC-806-04 | Auth failure responses (401/403) include `request_id`. |
| AC-806-05 | Integration tests verify `request_id` presence in error bodies. |
