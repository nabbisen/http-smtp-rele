# RFC 072 — Rate Limit Pipeline Ordering

**Status.** Implemented  
**Tracks.** Security  
**Touches.** `src/app.rs`, `src/api/handlers.rs`

## Summary

Fix the order of rate limit checks in the request pipeline to maximize security: global and
IP limits before auth, key limit after auth.

## Motivation

Placing global/IP limits before auth prevents unauthenticated floods from consuming auth CPU.
Placing the key limit after auth ensures it uses the correct per-key configuration
(FR-030, FR-031, FR-032).

## Scope

- Middleware layer order for global and IP rate limits.
- Handler-level call order for key rate limit.
- Rationale for the before-auth vs. after-auth split.

## Non-goals

- Token bucket implementation (RFC 071).
- `Retry-After` response (RFC 073).

## Design

### Pipeline

```
RequestContextLayer
    → GlobalRateLimitLayer  (Tower middleware, before auth)
    → IpRateLimitLayer      (Tower middleware, before auth)
    → Auth extractor        (AppError::Unauthorized/Forbidden if fails)
    → KeyRateLimitCheck     (handler-level, after auth)
    → ContentTypeCheck
    → StrictJson extractor
    → validation, policy, SMTP
```

### Axum middleware vs. in-handler

Global and IP: Tower middleware layers, applied to all routes via `layer()`.

```rust
let app = Router::new()
    .route("/v1/send", post(handle_send))
    .layer(IpRateLimitLayer::new(state.rate_limiter.clone()))
    .layer(GlobalRateLimitLayer::new(state.rate_limiter.clone()))
    .layer(RequestContextLayer::new(...));
```

Key limit: called explicitly inside the handler, after `AuthContext` is resolved.

```rust
async fn send_mail(
    State(state): State<AppState>,
    ctx: RequestContext,
    auth: AuthContext,
) -> Result<Json<SendResponse>, AppError> {
    state.rate_limiter
        .check_key(&auth.key.key_id, &auth.effective_rate_limits)
        .map_err(|secs| AppError::RateLimited { retry_after_secs: Some(secs) })?;
    // ...
}
```

## Implementation Plan

1. Implement `GlobalRateLimitLayer` and `IpRateLimitLayer` as Tower middleware.
2. Apply them to the router in the correct order.
3. Add key rate limit check inside the send handler.
4. Write integration tests verifying order.

## Test Plan

### Integration Tests

- A flood from an unauthenticated request hits the global/IP limit and returns 429 without
  reaching the auth step.
- A valid key that has exceeded its limit returns 429 (not 200).
- Key A's limit exhaustion does not affect key B.

## Security Considerations

- Order is critical: global/IP before auth prevents CPU exhaustion from high-volume invalid-key
  attacks.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-072-01 | Global limit fires before auth middleware. |
| AC-072-02 | IP limit fires before auth middleware. |
| AC-072-03 | Key limit fires after auth succeeds. |

## Open Questions

None.
