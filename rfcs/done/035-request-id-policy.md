# RFC 035 — Request ID Response Policy

**Status.** Implemented  
**Tracks.** API / Foundation  
**Touches.** `src/context.rs`, `src/api/responses.rs`, `docs/api.md`

## Summary

Establish that every HTTP response — success, error, and framework-level rejection — carries
a server-generated `request_id` both in the `X-Request-Id` response header and in the JSON
response body.

## Motivation

Operators correlate client error reports with server logs using the `request_id`. If any
response class (413 from body limit, 404 from unknown path, 500 from internal error) omits
the `request_id`, the correlation chain breaks (FR-032 concept, NFR-OPS-004).

## Scope

- `X-Request-Id` response header present on all responses.
- `request_id` field present in all JSON response bodies (success and error).
- Fallback `request_id` generation for cases where `RequestContextLayer` did not run.
- Format: UUIDv4 string.

## Non-goals

- Client-supplied request IDs (not trusted; see RFC 031 `metadata` semantics).
- Tracing propagation headers (W3C `traceparent`); deferred to v0.2.

## Design

### Header injection

`RequestContextLayer` (RFC 011) generates the `request_id` and inserts `X-Request-Id` on the
response before it is sent:

```rust
fn add_request_id_header(response: &mut Response, request_id: &str) {
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(request_id).unwrap_or_else(|_| HeaderValue::from_static("unknown")),
    );
}
```

This runs in the middleware layer's response path, after the handler.

### JSON body injection

`SendResponse` (RFC 031) and `ErrorResponse` (RFC 032) both contain `request_id: String`.
The value is taken from `RequestContext.request_id`.

### Fallback for framework-level rejections

When Axum's built-in extractors reject a request (413 from body limit, 405 from wrong method),
the `RequestContextLayer` may or may not have run before the rejection. To ensure coverage:

1. `RequestContextLayer` runs outermost (before body limit and method checks).
2. If the `request_id` is not in the extension map, the fallback JSON response handler
   generates a new UUID for that response.

### Format

UUIDv4 lowercase with hyphens: `xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx`.

## Implementation Plan

1. Add response header injection to `RequestContextLayer`.
2. Verify `SendResponse` and `ErrorResponse` include `request_id` (done in RFC 011, 031, 032).
3. Add a fallback 404/405 handler that generates a `request_id` if none exists.
4. Write integration tests.

## Test Plan

### Integration Tests

- Every successful response has `X-Request-Id` header.
- Every error response has `X-Request-Id` header.
- The `X-Request-Id` header value matches `request_id` in the JSON body.
- 413, 415, 404, 405 responses all include `X-Request-Id`.

## Security Considerations

- The `request_id` is server-generated. It must not be taken from client headers.
- The `request_id` does not expose any server state; UUIDv4 is unpredictable.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-035-01 | Every response has `X-Request-Id` header. |
| AC-035-02 | The header value matches `request_id` in the JSON body. |
| AC-035-03 | 413, 415, 404, 405 responses include `X-Request-Id`. |
| AC-035-04 | `request_id` is a valid UUIDv4 string. |

## Open Questions

None.
