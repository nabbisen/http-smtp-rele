# RFC 031 — Request and Response JSON Contract

**Status.** Implemented  
**Tracks.** API  
**Touches.** `src/api/handlers.rs`, `src/api/responses.rs`, `docs/api.md`

## Summary

Define the exact JSON shapes for the `POST /v1/send` request body and success response,
including all fields, their types, required/optional status, and the stability guarantee.

## Motivation

The JSON contract is the interface external systems depend on. Ambiguity in field types,
optional vs. required status, or behavior of extra fields leads to integration bugs. Defining
the contract explicitly and enforcing `deny_unknown_fields` makes the behavior predictable
and prevents accidentally accepting fields that could introduce security issues (FR-002, FR-003).

## Scope

- `MailRequest` request DTO struct.
- `SendResponse` success response struct.
- `deny_unknown_fields` policy.
- Field presence rules (required, optional, forbidden).
- The `metadata` field semantics.

## Non-goals

- Error response shape (RFC 032).
- Validation rules on field values (RFC 050, 051).
- Authentication headers (RFC 040).
- Content-Type enforcement (RFC 033).

## Design

### `MailRequest`

```rust
/// Request body for POST /v1/send.
///
/// Unknown fields are rejected (deny_unknown_fields) to prevent clients
/// from sending unsupported options and assuming they have effect.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MailRequest {
    /// Recipient email address. Required.
    pub to: String,

    /// Email subject line. Required.
    pub subject: String,

    /// Email body. Plain text only. Required.
    pub body: String,

    /// Display name for the From header. Optional.
    /// The From address is always the configured default_from.
    pub from_name: Option<String>,

    /// Reply-To address. Optional. Must be a valid email address.
    pub reply_to: Option<String>,

    /// Client-supplied metadata. Optional.
    /// Used for client-side request correlation only.
    /// Not reflected in the email or its headers.
    pub metadata: Option<serde_json::Value>,
}
```

### Explicitly excluded fields

The following fields are explicitly NOT accepted:

| Field | Why excluded |
|-------|-------------|
| `from` | Client-controlled From is a spoofing/injection risk |
| `cc` | Not in MVP |
| `bcc` | Not in MVP; Bcc header injection risk |
| `headers` | Arbitrary header injection risk |
| `attachments` | Not in MVP |
| `html_body` | Not in MVP |

Because `deny_unknown_fields` is set, any request containing these fields will receive `400
Bad Request`. The error message will indicate an unknown field was present, but not the
specific field name (to avoid leaking internal API expectations; revisit this decision if
operator debugging trumps security here).

### `SendResponse`

```rust
#[derive(Debug, Serialize)]
pub struct SendResponse {
    /// Always "accepted" on success.
    pub status: &'static str,

    /// Server-generated request identifier.
    /// Matches the X-Request-Id response header.
    pub request_id: String,
}
```

Example:

```json
{
  "status": "accepted",
  "request_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

HTTP status: `202 Accepted` (SMTP server has accepted the message for delivery; final
delivery is not guaranteed).

### `metadata` semantics

- Accepted as a JSON object (or any JSON value).
- Stored in `RequestContext` for log correlation.
- Never reflected in the email (subject, body, or headers).
- `metadata.request_id` from the client is for the client's tracking only; it does not affect
  the server's `request_id`.
- Maximum size of `metadata` is bounded by `max_request_body_bytes` (shared with the full request).

### Minimal valid request

```json
{
  "to": "recipient@example.com",
  "subject": "Hello",
  "body": "This is a message."
}
```

### Full request with all optional fields

```json
{
  "to": "recipient@example.com",
  "subject": "Hello",
  "body": "This is a message.",
  "from_name": "Example Service",
  "reply_to": "support@example.com",
  "metadata": {
    "request_id": "client-req-001",
    "source": "billing-service"
  }
}
```

### Content-Type requirement

Requests must have `Content-Type: application/json`. Requests with any other Content-Type are
rejected with `415 Unsupported Media Type` before the body is deserialized (RFC 033).

## Implementation Plan

1. Define `MailRequest` in `src/api/handlers.rs` or a dedicated `src/api/dto.rs`.
2. Define `SendResponse` in `src/api/responses.rs`.
3. Implement the `POST /v1/send` handler stub that deserializes `MailRequest` and returns
   `SendResponse` with a generated `request_id`.
4. Write tests.

## Test Plan

### Unit Tests

- Minimal request (to, subject, body) deserializes correctly.
- Full request with all optional fields deserializes correctly.
- Request with `from` field present → 400 (unknown field).
- Request with `headers` field present → 400 (unknown field).
- Request with `cc` field present → 400 (unknown field).

### Integration Tests

- Valid minimal request returns 202 with `{"status":"accepted","request_id":"..."}`.
- Response `request_id` matches `X-Request-Id` header.
- Request with extra field returns 400.

### Security Tests

- Request containing `from` field is rejected before reaching SMTP.
- Request containing `bcc` field is rejected before reaching SMTP.
- `metadata` content is not reflected in the email.

## Security Considerations

- `deny_unknown_fields` is the primary defense against undocumented field injection.
  It must never be removed without a security review.
- `metadata` must not be reflected in any email header or body, regardless of content.
  A future extension that uses `metadata` fields in email must be gated on explicit config.
- The `from_name` field is only used as the display name in the `From` header; the actual
  From address is always the configured `default_from`. A long or CR/LF-containing
  `from_name` is rejected by sanitization (RFC 051).

## Operational Considerations

- The `202 Accepted` status signals that the SMTP server has accepted the message, not that
  delivery has completed. Clients should use delivery notification mechanisms (e.g., bounce
  emails) if confirmation is critical.
- The `request_id` in the response allows operators to correlate a client report with server
  logs.

## Documentation Changes

- Document the full request/response schema in `docs/api.md`.
- Include examples (minimal and full request).
- Document `metadata` semantics and limitations.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-031-01 | `MailRequest` uses `deny_unknown_fields`. |
| AC-031-02 | Requests with `from`, `headers`, `cc`, `bcc` fields return 400. |
| AC-031-03 | `SendResponse` contains `status: "accepted"` and a `request_id`. |
| AC-031-04 | HTTP status is 202, not 200, on success. |
| AC-031-05 | `metadata` is accepted but never reflected in the email. |

## Open Questions

- Whether to include the client's `metadata.request_id` in the response for round-trip
  correlation. Deferred: the server `request_id` is sufficient; clients can maintain their
  own mapping.
