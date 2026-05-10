# RFC 036 — Submission Status API

**Status.** Proposed  
**Tracks.** T2 — HTTP API  
**Touches.** `src/api/submissions.rs`, `src/api/mod.rs`

## Summary

Define the external HTTP API contract for querying submission status by `request_id`.
This is a metadata-only lookup endpoint; it does not expose mail content, trigger retries,
or represent final delivery state.

## Endpoint

```http
GET /v1/submissions/{request_id}
Authorization: Bearer <token>
```

The `request_id` path parameter must be the opaque application-owned identifier returned
in all `POST /v1/send` responses (and any rejection response) via the `X-Request-Id`
header and `request_id` JSON field.

## request_id external format

The canonical external format is `req_` followed by a ULID:

```text
req_01HX7Q9V6R6W9V8Y5E3E6E7M9A
```

Clients must treat `request_id` as an opaque string.
They must not parse the timestamp component or depend on internal format details.

## Success response (200 OK)

```json
{
  "request_id": "req_01HX7Q9V6R6W9V8Y5E3E6E7M9A",
  "status": "smtp_accepted",
  "code": null,
  "message": "The message was accepted by the configured SMTP server.",
  "recipient_domains": ["example.com"],
  "recipient_count": 1,
  "created_at": "2026-05-10T12:00:00Z",
  "updated_at": "2026-05-10T12:00:01Z",
  "expires_at": "2026-05-10T13:00:00Z"
}
```

Status values: `received`, `rejected`, `smtp_submission_started`, `smtp_accepted`, `smtp_failed`.

## Not found / expired (404 Not Found)

Returned for: unknown `request_id`, expired records, records created by a different API key.

```json
{
  "status": "error",
  "code": "submission_not_found",
  "message": "Submission status was not found or has expired.",
  "request_id": "req_..."
}
```

Responding 404 (not 403) for other-key records prevents existence enumeration.

## Authentication and access control

- A valid API key (`Authorization: Bearer`) is required.
- Status records are scoped by `key_id`.
- A key can only read records it created.
- Authentication failures return 401/403 as usual; they do not create status records.

## What this endpoint does not provide

- Final delivery state, bounce, or mailbox acknowledgement.
- Mail body, subject, full recipient addresses.
- Records for pre-auth rejections (no `key_id` to scope by).

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-036-01 | 200 response for a valid, in-scope `request_id`. |
| AC-036-02 | 404 for unknown, expired, or different-key `request_id`. |
| AC-036-03 | 401/403 for unauthenticated/unauthorised requests. |
| AC-036-04 | Response contains no mail body, subject, token, or full recipient address. |
