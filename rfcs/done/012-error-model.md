# RFC 012 — Error Model and HTTP Response Mapping

**Status.** Implemented  
**Tracks.** Foundation  
**Touches.** `src/error.rs`, `src/api/responses.rs`

## Summary

Define the `AppError` enum, its mapping to HTTP status codes and JSON error bodies, and the
policy that internal details are logged — never returned to clients.

## Motivation

A uniform error model prevents accidental information disclosure (NFR-SEC-001), ensures every
error response is a valid JSON object with a stable `code` field (FR-071, FR-072), and makes
the behavior of every handler predictable. Without a central error type, handlers diverge in
what they return, making the API contract impossible to pin down.

## Scope

- `AppError` enum with all error variants used by the MVP.
- `AppError → (StatusCode, ErrorResponse)` mapping.
- `ErrorResponse` JSON shape.
- Axum `IntoResponse` implementation for `AppError`.
- Policy: internal error messages are logged at `error` level, not returned.
- `request_id` is always present in error responses.

## Non-goals

- Logging implementation (RFC 013 / RFC 080).
- Specific validation error messages (RFC 050).
- Rate limit error format (RFC 073).

## Design

### `AppError` variants

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    // ── 400 ─────────────────────────────────────────────────────────────────
    #[error("invalid request payload")]
    BadRequest,

    #[error("validation failed: {0}")]
    Validation(String),

    // ── 401 ─────────────────────────────────────────────────────────────────
    #[error("missing or malformed authorization")]
    Unauthorized,

    // ── 403 ─────────────────────────────────────────────────────────────────
    #[error("authentication failed or insufficient permissions")]
    Forbidden,

    // ── 413 ─────────────────────────────────────────────────────────────────
    #[error("request payload too large")]
    PayloadTooLarge,

    // ── 415 ─────────────────────────────────────────────────────────────────
    #[error("unsupported media type")]
    UnsupportedMediaType,

    // ── 429 ─────────────────────────────────────────────────────────────────
    #[error("rate limit exceeded")]
    RateLimited { retry_after_secs: Option<u64> },

    // ── 502 ─────────────────────────────────────────────────────────────────
    #[error("SMTP server unavailable or rejected the message")]
    SmtpUnavailable,

    // ── 500 ─────────────────────────────────────────────────────────────────
    #[error("internal error")]
    Internal(&'static str),
}
```

### HTTP status mapping

| Variant | Status | JSON `code` |
|---------|--------|-------------|
| `BadRequest` | 400 | `bad_request` |
| `Validation(_)` | 400 | `validation_failed` |
| `Unauthorized` | 401 | `unauthorized` |
| `Forbidden` | 403 | `forbidden` |
| `PayloadTooLarge` | 413 | `payload_too_large` |
| `UnsupportedMediaType` | 415 | `unsupported_media_type` |
| `RateLimited { .. }` | 429 | `rate_limited` |
| `SmtpUnavailable` | 502 | `smtp_unavailable` |
| `Internal(_)` | 500 | `internal_error` |

### `ErrorResponse` shape

```rust
#[derive(Serialize)]
pub struct ErrorResponse {
    pub status: &'static str,   // always "error"
    pub code: &'static str,     // stable machine-readable code
    pub message: String,        // human-readable; sanitized
    pub request_id: String,
}
```

JSON example:

```json
{
  "status": "error",
  "code": "validation_failed",
  "message": "Invalid email address in 'to' field",
  "request_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

### Information disclosure policy

- `message` for `AppError::Internal` is always the fixed string `"An internal error occurred"` —
  the internal detail (from the `&'static str` field) is logged, never returned.
- `message` for `AppError::Validation` includes the field name and a safe description, but
  never the raw value (which could contain injected content or sensitive data).
- `message` for authentication errors is intentionally generic — the server must not reveal
  whether a key exists or is disabled (see FR-072 error code table).

### `IntoResponse` implementation

```rust
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let code = self.error_code();

        // Log internal details before sanitizing.
        if status.is_server_error() {
            tracing::error!(error = %self, "internal error");
        }

        // Build sanitized JSON body.
        let body = Json(ErrorResponse {
            status: "error",
            code,
            message: self.safe_message(),
            request_id: // extracted from response extensions or generated
        });

        (status, body).into_response()
    }
}
```

The `request_id` is injected from `RequestContext` (RFC 011). The exact mechanism (response
extension vs. middleware-level injection) is resolved during M1 implementation.

## Implementation Plan

1. Define `AppError` in `src/error.rs`.
2. Implement `status_code()`, `error_code()`, `safe_message()` methods.
3. Define `ErrorResponse` in `src/api/responses.rs`.
4. Implement `IntoResponse for AppError`.
5. Write unit tests for every variant's mapping.

## Test Plan

### Unit Tests

- Each `AppError` variant maps to the correct `StatusCode`.
- Each variant produces the correct `code` string.
- `AppError::Internal` never exposes the internal detail in `safe_message()`.
- `AppError::Validation("to field: CR/LF detected")` produces code `validation_failed`.

### Integration Tests

- A handler returning `AppError::Unauthorized` yields `{"status":"error","code":"unauthorized",...}`.
- A handler returning `AppError::Internal(...)` yields `{"code":"internal_error","message":"An internal error occurred",...}`.
- All error responses include `request_id`.

### Security Tests

- Internal error messages are not present in the JSON response body (check `message` field).
- Auth failure messages do not reveal whether the key exists (`forbidden` vs. `unauthorized`
  semantics are preserved).

## Security Considerations

- The distinction between `Unauthorized` (401) and `Forbidden` (403) is intentional:
  - 401 = no credentials provided.
  - 403 = credentials provided but invalid or key disabled.
  - Neither response reveals anything about the key store.
- Internal errors must never leak stack traces, file paths, or Rust error chains to the client.
- `message` for validation errors must not echo raw user input (risk of reflected injection).

## Operational Considerations

- The `code` strings are part of the public API contract and must not change without a
  breaking-change notice (see RFC 030).
- Internal errors are logged at `error` level; client errors at `warn` or `info` depending on
  severity (full policy in RFC 080).

## Documentation Changes

- Document all error codes in `docs/api.md`.
- Document the information disclosure policy in `docs/security.md`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-012-01 | All `AppError` variants have a defined HTTP status, JSON code, and safe message. |
| AC-012-02 | `AppError::Internal` never exposes the internal detail in the response body. |
| AC-012-03 | All error responses are `Content-Type: application/json`. |
| AC-012-04 | All error responses include a `request_id` field. |
| AC-012-05 | Error code strings are stable across minor versions. |

## Open Questions

- Whether to include a `details` field for validation errors listing all failing fields at once.
  Deferred: for MVP, a single error per response is sufficient.
