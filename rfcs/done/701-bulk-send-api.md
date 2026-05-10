# RFC 701 — POST /v1/send-bulk

**Status.** Proposed  
**Tracks.** T2 — HTTP API  
**Touches.** `src/api/send_bulk.rs`, `src/api/mod.rs`, `src/config.rs`

## Summary

Add `POST /v1/send-bulk` accepting an array of independent mail messages.
Each message goes through the same pipeline as `POST /v1/send`.
Results are returned per-message in the response body.

## Request

```http
POST /v1/send-bulk
Authorization: Bearer <token>
Content-Type: application/json
```

```json
{
  "messages": [
    {
      "to": "alice@example.com",
      "subject": "Hello",
      "body": "Hello Alice."
    },
    {
      "to": ["bob@example.com", "carol@example.org"],
      "cc": "dave@example.com",
      "subject": "Greetings",
      "body": "Hello team.",
      "html_body": "<p>Hello team.</p>"
    }
  ]
}
```

Each element has the same schema as `POST /v1/send`.

## Response — 202 Accepted

Returned when the payload is valid and auth passes.
Per-message outcomes are in `results`.

```json
{
  "bulk_request_id": "req_01HX...",
  "total":    2,
  "accepted": 1,
  "rejected": 1,
  "results": [
    {
      "index":      0,
      "request_id": "req_01HX...",
      "status":     "accepted"
    },
    {
      "index":      1,
      "request_id": "req_01HX...",
      "status":     "rejected",
      "code":       "validation_failed",
      "message":    "Recipient domain not allowed: example.org"
    }
  ]
}
```

The top-level HTTP status is 202 regardless of individual message outcomes,
provided auth passed and the payload structure is valid.

## Error responses

| Condition | Status | `code` |
|-----------|--------|--------|
| Missing/invalid auth | 401/403 | `unauthorized` / `forbidden` |
| Empty `messages` array | 400 | `bad_request` |
| `messages` exceeds `max_bulk_messages` | 400 | `payload_too_large` |
| Malformed JSON | 400 | `bad_request` |

## Configuration

```toml
[mail]
max_bulk_messages = 10   # default; maximum messages per bulk request
```

## Processing pipeline per message (sequential, v0.9)

1. Rate limit check (global, IP, per-key) — counted per message.
2. Validate and sanitize.
3. Build lettre `Message`.
4. Submit to SMTP.
5. Update status store.
6. Record per-message result.

SMTP submissions are sequential in v0.9. Bounded parallelism is a v1.0 optimisation.

## Status tracking

Each message receives its own `request_id` and status record.
The `bulk_request_id` in the response identifies the outer request for log correlation;
it does not appear in the status store.

`GET /v1/submissions/{request_id}` works per-message using the individual
`request_id` values from `results`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-701-01 | Array of messages → 202, per-message results. |
| AC-701-02 | Empty array → 400. |
| AC-701-03 | Array exceeding `max_bulk_messages` → 400. |
| AC-701-04 | One invalid message does not fail others. |
| AC-701-05 | Each message has distinct `request_id`. |
| AC-701-06 | Per-message `request_id` queryable via `GET /v1/submissions/`. |
| AC-701-07 | Audit log entry per message. |
