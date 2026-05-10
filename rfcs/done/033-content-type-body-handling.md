# RFC 033 — Request Body and Content-Type Handling

**Status.** Implemented  
**Tracks.** API  
**Touches.** `src/app.rs`, `src/api/handlers.rs`, `src/api/extractors.rs`

## Summary

Enforce `Content-Type: application/json` and the configured body size limit as middleware
layers applied before JSON deserialization, ensuring malformed or oversized requests are
rejected early with a consistent JSON error response.

## Motivation

Without an explicit Content-Type check, Axum's JSON extractor may attempt to parse
non-JSON bodies and produce a confusing error. Without an early body size limit, a client
can stream an arbitrarily large body before receiving a rejection, consuming memory and CPU
(FR-040, requirement §6.5, NFR-PERF-003).

## Scope

- Content-Type check: reject non-`application/json` with 415.
- Body size limit: enforce at the middleware layer, before body is buffered.
- JSON deserialization error mapping: ensure Axum's default JSON extractor errors map to
  `AppError::BadRequest` or `AppError::Validation`.
- Strict JSON: no trailing content, no comments.

## Non-goals

- Semantic validation of JSON fields (RFC 050).
- Sanitization of field values (RFC 051).
- Authentication (RFC 040).

## Design

### Content-Type enforcement

Implemented as a middleware layer applied to `/v1/send` only (not health endpoints):

```rust
async fn check_content_type(
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let content_type = req
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !content_type.starts_with("application/json") {
        return Err(AppError::UnsupportedMediaType);
    }

    Ok(next.run(req).await)
}
```

This middleware runs before the JSON extractor, so the 415 response is produced before any
body reading occurs.

Accepted `Content-Type` values:
- `application/json`
- `application/json; charset=utf-8`

Any other value → 415.

### Body size limit

Provided by `tower_http::limit::RequestBodyLimitLayer` (configured in RFC 024). The limit
applies to the entire request body, not just the JSON portion.

When the limit is exceeded, Axum's `PayloadTooLarge` rejection is returned. A custom rejection
handler (RFC 032) maps this to `AppError::PayloadTooLarge` → 413 JSON response.

### JSON extraction and error mapping

Axum's built-in `Json<T>` extractor is used. Its rejection types:

| Axum rejection | Mapping |
|---------------|---------|
| `JsonDataError` | `AppError::Validation` |
| `JsonSyntaxError` | `AppError::BadRequest` |
| `MissingJsonContentType` | Handled before this point by Content-Type middleware |
| `BytesRejection` | `AppError::PayloadTooLarge` or `AppError::Internal` |

Custom extractor wrapping `axum::Json` to produce `AppError`:

```rust
pub struct StrictJson<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for StrictJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(axum::Json(value)) => Ok(StrictJson(value)),
            Err(rejection) => Err(map_json_rejection(rejection)),
        }
    }
}

fn map_json_rejection(rejection: JsonRejection) -> AppError {
    match rejection {
        JsonRejection::JsonDataError(e) => AppError::Validation(e.to_string()),
        JsonRejection::JsonSyntaxError(_) => AppError::BadRequest,
        JsonRejection::MissingJsonContentType(_) => AppError::UnsupportedMediaType,
        JsonRejection::BytesRejection(_) => AppError::PayloadTooLarge,
        _ => AppError::BadRequest,
    }
}
```

### Unknown fields

The `deny_unknown_fields` attribute on `MailRequest` (RFC 031) causes serde to return a
deserialization error for unknown fields. This is caught by `StrictJson` and mapped to
`AppError::Validation("unknown field: ...")`.

The error message for unknown fields must not include the field name verbatim if the name
could be user-controlled in a way that enables injection. For MVP, field names in JSON keys
are safe to echo in error messages (they are not email content), but this should be reviewed.

## Implementation Plan

1. Implement `check_content_type` middleware function.
2. Apply it to the `/v1/send` route in `routes.rs`.
3. Implement `StrictJson<T>` extractor in `src/api/extractors.rs`.
4. Replace `Json<MailRequest>` with `StrictJson<MailRequest>` in the send handler.
5. Verify the custom rejection handlers for 413 and 415 are in place (RFC 032).
6. Write tests.

## Test Plan

### Unit Tests

- `check_content_type` allows `application/json`.
- `check_content_type` allows `application/json; charset=utf-8`.
- `check_content_type` rejects `text/plain` with 415.
- `check_content_type` rejects absent Content-Type with 415.
- `StrictJson` maps `JsonSyntaxError` to `AppError::BadRequest`.
- `StrictJson` maps `JsonDataError` (unknown field) to `AppError::Validation`.

### Integration Tests

- `POST /v1/send` with `Content-Type: text/plain` → 415 JSON.
- `POST /v1/send` with no `Content-Type` → 415 JSON.
- `POST /v1/send` with body `{invalid json` → 400 JSON.
- `POST /v1/send` with body `{"unknown_field":"x","to":"a@b.com","subject":"s","body":"b"}` → 400 JSON.
- `POST /v1/send` with body larger than limit → 413 JSON.
- `GET /healthz` with any `Content-Type` → 200 (Content-Type check not applied to health).

### Security Tests

- Oversized body is rejected before body is fully buffered.
- Content-Type check runs before JSON deserialization (order matters for performance).

## Security Considerations

- The body size limit enforced by `RequestBodyLimitLayer` prevents memory exhaustion. The
  limit must be applied before any buffering, which `tower-http` guarantees.
- Rejecting non-JSON Content-Type prevents clients from accidentally sending binary data or
  form-encoded data that could confuse the deserializer.
- `deny_unknown_fields` (RFC 031) closes the "extra field injection" vector; this RFC
  ensures unknown fields are caught and produce a `400` before they can propagate.

## Operational Considerations

- The body size limit is configured in `[server].max_request_body_bytes`. Mail body is
  separately limited by `[mail].max_body_bytes` at validation time (RFC 053).
- If a client sends `Content-Type: application/json; charset=latin-1`, the middleware accepts
  it (starts with `application/json`). Serde will attempt UTF-8 parsing; if the body is not
  valid UTF-8, `JsonSyntaxError` results in 400.

## Documentation Changes

- Document accepted Content-Type values in `docs/api.md`.
- Document body size limits in `docs/configuration.md`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-033-01 | `Content-Type: application/json` is required for `POST /v1/send`. |
| AC-033-02 | Non-JSON Content-Type returns 415 with JSON error body. |
| AC-033-03 | Malformed JSON returns 400 with JSON error body. |
| AC-033-04 | Oversized body returns 413 with JSON error body. |
| AC-033-05 | Unknown JSON fields return 400 with JSON error body. |
| AC-033-06 | Content-Type check does not apply to `/healthz` or `/readyz`. |

## Open Questions

None.
