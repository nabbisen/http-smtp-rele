# RFC 011 — Application State and Request Context

**Status.** Implemented  
**Tracks.** Foundation  
**Touches.** `src/context.rs`, `src/app.rs`, `src/api/extractors.rs`

## Summary

Define `AppState` (the shared, immutable application state) and `RequestContext` (per-request
context carrying `request_id`, resolved `client_ip`, and `key_id` after authentication).

## Motivation

Every handler needs access to config, rate limiters, and the SMTP handle. Every audit log entry
needs a `request_id` to correlate with the HTTP response. Defining these types explicitly and
centrally prevents ad-hoc field passing and makes the data flow easy to audit (FR-001, NFR-SEC-007).

## Scope

- `AppState`: fields, derivations, and `Arc` wrapping policy.
- `RequestContext`: fields, construction, and how it flows through the middleware stack.
- `request_id` generation: format, generation point, and propagation.
- Axum extractor for `RequestContext` so handlers receive it as a typed argument.
- `client_ip` resolution: where the forwarded IP vs. socket IP decision lives (full logic in RFC 041).

## Non-goals

- Authentication logic (RFC 040–044) — `key_id` is populated by auth middleware, not here.
- Rate limiter implementation (RFC 070–071) — `AppState.rate_limiter` is a type placeholder.
- SMTP transport implementation (RFC 061) — `AppState.smtp` is a type placeholder.
- Trusted proxy evaluation (RFC 041) — context carries the resolved IP; resolution logic is elsewhere.

## Design

### `AppState`

```rust
/// Shared, read-only application state.
///
/// Wrapped in `Arc` by the router builder; all fields must be `Send + Sync`.
#[derive(Clone)]
pub struct AppState {
    /// Loaded and validated configuration. Immutable after startup.
    pub config: Arc<AppConfig>,

    /// In-memory rate limiter. Shared across all requests.
    pub rate_limiter: Arc<RateLimiter>,

    /// SMTP transport handle. Used by the send handler.
    pub smtp: Arc<SmtpHandle>,
}
```

`AppState` is constructed once in `main.rs` and passed to `Router::with_state`. Cloning is `O(1)`
(only reference counts are incremented).

### `RequestContext`

```rust
/// Per-request context. Built at the start of each request.
///
/// Populated by `RequestContextLayer` before any handler runs.
#[derive(Clone, Debug)]
pub struct RequestContext {
    /// Server-generated opaque identifier for this request.
    /// Included in every response and every log event.
    pub request_id: String,

    /// Resolved client IP (after trusted proxy evaluation).
    /// Used for IP-based rate limiting and access control.
    pub client_ip: IpAddr,

    /// Set by auth middleware after successful authentication.
    /// Empty string before auth; set to `key_id` on success.
    pub key_id: String,
}
```

`RequestContext` is inserted into the request's extension map by a Tower middleware layer and
retrieved via an Axum extractor.

### `request_id` format

Use UUIDv4 for MVP (widely recognized, easy to grep, no external crate beyond `uuid`). A 32-char
lowercase hex string without hyphens is also acceptable to reduce line length in logs.

Format: `xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx`

Generated at the very start of the request, before any handler or middleware runs, using
`uuid::Uuid::new_v4().to_string()`.

### Axum extractor

```rust
#[async_trait]
impl<S> FromRequestParts<S> for RequestContext
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<RequestContext>()
            .cloned()
            .ok_or(AppError::Internal("missing request context"))
    }
}
```

### Middleware layer

A Tower `Layer` wraps every request:

```rust
pub struct RequestContextLayer {
    trusted_proxy_cidrs: Arc<Vec<IpNet>>,
}

impl<S> Service<Request> for RequestContextService<S> {
    fn call(&mut self, mut req: Request) -> Self::Future {
        let request_id = Uuid::new_v4().to_string();
        let client_ip = resolve_client_ip(&req, &self.trusted_proxy_cidrs);
        req.extensions_mut().insert(RequestContext {
            request_id,
            client_ip,
            key_id: String::new(),
        });
        self.inner.call(req)
    }
}
```

`key_id` is populated by the auth middleware (RFC 040) after this layer runs.

### Response header

Every response must include `X-Request-Id: <request_id>`. Applied by the middleware layer before
returning the response, so even error responses carry it.

## Implementation Plan

1. Define `AppState` in `src/app.rs`.
2. Define `RequestContext` in `src/context.rs`.
3. Implement `request_id` generation using `uuid`.
4. Implement `RequestContextLayer` in `src/context.rs`.
5. Implement `FromRequestParts<S> for RequestContext`.
6. Add `RequestContextLayer` to the router in `src/app.rs`.
7. Add `X-Request-Id` response header in the layer.
8. Write unit tests.

## Test Plan

### Unit Tests

- `RequestContext` is present in extension map after `RequestContextLayer` runs.
- `request_id` is a valid UUID v4 string.
- `client_ip` is the socket peer address when no trusted proxy header is present (full logic in RFC 041).
- Two sequential requests produce different `request_id` values.

### Integration Tests

- Every response (including 400, 401, 415, 429, 502) includes `X-Request-Id`.
- The `request_id` in the response matches the one in the corresponding log event.

### Operational Tests

- `AppState` can be constructed in a test with minimal config.

## Security Considerations

- `key_id` starts empty and is set only after auth succeeds. It must never be set from
  untrusted request input. Auth middleware is the only writer.
- `client_ip` resolution must not trust `X-Forwarded-For` from untrusted peers (enforcement
  in RFC 041, but the contract is established here: `client_ip` is always the *resolved* IP,
  never the raw header value).
- `request_id` is server-generated and must not be taken from request headers. A client-supplied
  ID in `metadata.request_id` is for client-side correlation only and must not overwrite the
  server's `request_id`.

## Operational Considerations

- The `request_id` appears in both the response header and every log event for this request.
  Operators can correlate a reported error with its full log trace.
- `AppState` is `Clone` to satisfy Axum's `Service` requirements; the `Arc` fields ensure
  sharing is reference-counted, not copied.

## Documentation Changes

- Document `X-Request-Id` in `docs/api.md`.
- Document `key_id` correlation in `docs/security.md`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-011-01 | `RequestContext` is inserted into every request's extension map before handlers run. |
| AC-011-02 | `request_id` is a UUIDv4 string unique per request. |
| AC-011-03 | Every HTTP response includes `X-Request-Id` matching the request's `request_id`. |
| AC-011-04 | `key_id` starts as empty string and is set only by auth middleware. |
| AC-011-05 | `FromRequestParts` for `RequestContext` returns `AppError::Internal` if the context is missing. |

## Open Questions

- Whether to use ULID (sortable by creation time) instead of UUIDv4. Leaning toward UUID for
  familiarity and to avoid an extra dependency. Will finalize in RFC 035.
