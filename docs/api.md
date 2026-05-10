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
