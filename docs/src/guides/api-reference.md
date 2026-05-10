# API Reference

## Authentication

All requests to `/v1/send` must include an API key.

**Preferred form:**
```
Authorization: Bearer <secret>
```

**Alternative (compatibility):**
```
X-API-Key: <secret>
```

When both headers are present, `Authorization` takes precedence.

---

## POST /v1/send

Submit a mail message for relay.

### Request

**Content-Type:** `application/json` (required)

**Body fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `to` | string | ✓ | Recipient email address (RFC 5321 format) |
| `subject` | string | ✓ | Email subject. Max length: `mail.max_subject_length` chars |
| `body` | string | ✓ | Plain text body. Max size: `mail.max_body_length` bytes |
| `from_name` | string | — | Display name for the `From` header. Max 128 chars |
| `reply_to` | string | — | `Reply-To` address (RFC 5321 format) |
| `metadata` | object | — | Opaque client data. Logged for correlation; not sent in mail |

**Unknown fields are rejected.** The `from`, `cc`, `bcc`, and `headers` fields are
intentionally absent; the relay does not accept them.

**Example (minimal):**
```json
{
  "to": "user@example.com",
  "subject": "Hello",
  "body": "This is the message body."
}
```

**Example (full):**
```json
{
  "to": "user@example.com",
  "subject": "Hello",
  "body": "This is the message body.",
  "from_name": "My Application",
  "reply_to": "support@example.com",
  "metadata": {"client_request_id": "req-12345"}
}
```

### Response — 202 Accepted

```json
{
  "status": "accepted",
  "request_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

Use `request_id` to correlate with server logs when reporting issues.

### Response headers

| Header | Description |
|--------|-------------|
| `X-Request-Id` | Same value as `request_id` in the response body |
| `Retry-After` | Seconds to wait before retrying (present on 429 responses) |

---

## GET /healthz

Liveness probe. Returns 200 as long as the process is running.
No authentication required.

```json
{"status": "ok", "version": "0.1.0"}
```

---

## GET /readyz

Readiness probe. Returns 200 when the configured SMTP server is reachable via TCP.

**200 OK:**
```json
{"status": "ready", "smtp": "ok"}
```

**503 Service Unavailable:**
```json
{"status": "not_ready", "smtp": "unavailable"}
```

> **Security:** Restrict external access to `/readyz` at the reverse proxy layer.
> See [security.md](security.md).

---

## Error Codes

All error responses follow this shape:

```json
{
  "status": "error",
  "code": "<code>",
  "message": "<human-readable description>",
  "request_id": "<uuid>"
}
```

| HTTP | `code` | Cause |
|------|--------|-------|
| 400 | `validation_failed` | Field validation failed (bad address, CR/LF injection, size limit, unknown field) |
| 401 | `unauthorized` | No `Authorization` header or malformed token |
| 403 | `forbidden` | Token not matched or key disabled; also: source IP not allowed |
| 413 | `payload_too_large` | Request body exceeds `server.max_request_body_bytes` |
| 415 | `unsupported_media_type` | `Content-Type` is not `application/json` |
| 429 | `rate_limited` | Rate limit exceeded (global, IP, or per-key) |
| 500 | `internal_error` | Unexpected server error |
| 502 | `smtp_unavailable` | SMTP server is unreachable or rejected the message |
| 503 | `smtp_unavailable` | SMTP not reachable (readyz only) |
| 408 | `request_timeout` | Request processing exceeded `server.request_timeout_seconds` |

---

## Using `request_id`

Every response — success and error — includes a `request_id` (UUIDv4) in both the JSON
body and the `X-Request-Id` response header.

When the relay's logs are in JSON format, you can correlate the server-side log entry:

```sh
grep '"request_id":"550e8400..."' /var/log/http-smtp-rele.log
```

## POST /v1/send-bulk

Submit an array of independent mail messages in one request.
Each message is processed through the same validation and SMTP pipeline as `POST /v1/send`.

**Authentication:** Required (`Authorization: Bearer <token>`)

### Request body

```json
{
  "messages": [
    {
      "to": "alice@example.com",
      "subject": "Hello Alice",
      "body": "Hello."
    },
    {
      "to": ["bob@example.com", "carol@example.org"],
      "cc": "dave@example.com",
      "subject": "Hello team",
      "body": "Hello.",
      "html_body": "<p>Hello.</p>"
    }
  ]
}
```

Each element in `messages` has the same schema as `POST /v1/send`.
Maximum array length is controlled by `[mail].max_bulk_messages` (default: 10).

### Response — 202 Accepted

Returned when auth passes and the payload structure is valid.
Per-message outcomes are in `results`; partial success is normal.

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
      "message":    "Recipient domain not allowed."
    }
  ]
}
```

The `bulk_request_id` identifies the outer request for log correlation.
Each `results[].request_id` is queryable via `GET /v1/submissions/{request_id}`.

### Rate limiting

Rate limits are counted **per message**, not per bulk request.
A bulk request of 10 messages consumes 10 units from each applicable bucket.
If a rate limit is exhausted mid-array, earlier messages are unaffected;
remaining messages are rejected with `code = "rate_limited"`.

### Error responses

| Condition | Status |
|-----------|--------|
| Empty `messages` array | 400 |
| `messages` exceeds `max_bulk_messages` | 413 |
| Unauthenticated | 401 |
| Invalid API key | 403 |

