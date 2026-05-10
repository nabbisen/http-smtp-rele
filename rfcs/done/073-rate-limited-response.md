# RFC 073 — Rate Limited Response and Retry-After

**Status.** Implemented  
**Tracks.** Security  
**Touches.** `src/error.rs`, `src/api/responses.rs`

## Summary

When a rate limit is exceeded, return `429 Too Many Requests` with a `Retry-After` response
header when the estimated wait time is calculable.

## Motivation

Clients that implement retry logic need to know how long to wait. A `Retry-After` header
with a positive integer (seconds) enables standards-compliant retry behavior (HTTP RFC 7231,
FR-033 concept).

## Scope

- `Retry-After` header injection in `AppError::RateLimited::into_response`.
- Log event structure for rate limit hits.
- `retry_after_secs: Option<u64>` in the error variant.

## Non-goals

- Rate limit algorithm (RFC 071).
- Pipeline ordering (RFC 072).

## Design

### `AppError::RateLimited` shape

```rust
RateLimited { retry_after_secs: Option<u64> },
```

`retry_after_secs` is `Some(n)` when the token bucket can estimate the wait (RFC 071 returns
this value). It is `None` when the wait is not calculable (should not occur in practice with
the lazy-refill bucket, but handled defensively).

### `IntoResponse`

```rust
AppError::RateLimited { retry_after_secs } => {
    let body = Json(ErrorResponse {
        status: "error",
        code: "rate_limited",
        message: "Rate limit exceeded".into(),
        request_id: /* from extension */,
    });

    let mut response = (StatusCode::TOO_MANY_REQUESTS, body).into_response();

    if let Some(secs) = retry_after_secs {
        if let Ok(val) = HeaderValue::try_from(secs.to_string()) {
            response.headers_mut().insert("retry-after", val);
        }
    }

    response
}
```

### Log event

```
warn event=rate_limited tier=global request_id=... client_ip=1.2.3.4
warn event=rate_limited tier=ip     request_id=... client_ip=1.2.3.4
warn event=rate_limited tier=key    request_id=... key_id=service-a
```

No secret values are logged. Only the tier name, IP, and key_id (not secret).

## Test Plan

### Integration Tests

- Rate limited response returns HTTP 429.
- Response body has `code: "rate_limited"`.
- `Retry-After` header is present with a positive integer value.
- `Retry-After` value is approximately correct (within 2 seconds of expected).

## Security Considerations

- `Retry-After` does not expose any sensitive information.
- A client could use the `Retry-After` value to calibrate the exact rate limit. This is
  acceptable; the rate limit configuration is not secret.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-073-01 | Rate limited response has HTTP status 429. |
| AC-073-02 | Response body matches `ErrorResponse` shape with `code: "rate_limited"`. |
| AC-073-03 | `Retry-After` header is present when retry estimate is available. |
| AC-073-04 | Rate limit log event includes tier name and context (not secrets). |

## Open Questions

None.
