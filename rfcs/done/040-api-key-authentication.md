# RFC 040 — API Key Authentication Model

**Status.** Implemented  
**Tracks.** Security  
**Touches.** `src/auth.rs`, `src/api/extractors.rs`, `src/context.rs`

## Summary

Define the API key authentication model: how tokens are extracted from request headers,
how they are compared against the configured key store using constant-time comparison,
and how the resulting `AuthContext` is propagated to handlers.

## Motivation

Every mail submission request must be authenticated. Authentication must resist timing attacks
and must not reveal whether a key exists or is disabled. The auth logic must be centralized
so that it cannot be accidentally bypassed by a handler that forgets to check (FR-010,
FR-011, FR-012, FR-013, NFR-SEC-001, AC-002, AC-003).

## Scope

- `AuthContext` — the result of successful authentication.
- Bearer token extraction from `Authorization` header.
- `X-API-Key` fallback header.
- Lookup and constant-time comparison against `ApiKeyConfig` (RFC 022).
- Authentication as an Axum extractor that rejects the request on failure.
- `key_id` propagation to `RequestContext`.

## Non-goals

- Per-key permission policy (RFC 042).
- Rate limiting (RFC 070).
- Constant-time comparison implementation (RFC 043).
- Timing safety of the full iteration (covered in RFC 022).

## Design

### `AuthContext`

```rust
/// Result of successful API key authentication.
///
/// Carried as an Axum extractor; handlers that require auth declare it as a parameter.
#[derive(Clone, Debug)]
pub struct AuthContext {
    /// The authenticated key configuration.
    pub key: ApiKeyConfig,
}
```

`AuthContext` is also an Axum extractor. If the request is unauthenticated or authentication
fails, the extractor returns an `AppError` which Axum converts to an error response.

### Token extraction

Priority:
1. `Authorization: Bearer <token>` (preferred).
2. `X-API-Key: <token>` (compatibility fallback).

If both headers are present, `Authorization` takes precedence.

```rust
fn extract_token(headers: &HeaderMap) -> Option<&str> {
    if let Some(auth) = headers.get("authorization") {
        let s = auth.to_str().ok()?;
        if let Some(token) = s.strip_prefix("Bearer ") {
            return Some(token.trim());
        }
    }
    headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
}
```

### Authentication logic

```rust
pub fn authenticate<'a>(
    token: Option<&str>,
    api_keys: &'a [ApiKeyConfig],
) -> Result<&'a ApiKeyConfig, AppError> {
    let token = token.ok_or(AppError::Unauthorized)?;
    // Trim leading/trailing whitespace; reject if empty after trim.
    let token = token.trim();
    if token.is_empty() {
        return Err(AppError::Unauthorized);
    }

    // Iterate ALL keys to prevent timing-based key enumeration.
    let mut matched: Option<&ApiKeyConfig> = None;
    for key in api_keys {
        if key.secret.constant_time_eq(token) && key.enabled {
            matched = Some(key);
            // Do NOT break — continue to prevent timing variance.
        }
    }

    matched.ok_or(AppError::Forbidden)
}
```

Error codes:
- No header / empty token → `AppError::Unauthorized` (401).
- Token present but no match / key disabled → `AppError::Forbidden` (403).

### Axum extractor

```rust
#[async_trait]
impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync + HasApiKeys,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_token(&parts.headers);
        let key = authenticate(token, state.api_keys())?;

        // Propagate key_id to RequestContext for logging.
        if let Some(ctx) = parts.extensions.get_mut::<RequestContext>() {
            ctx.key_id = key.key_id.clone();
        }

        Ok(AuthContext { key: key.clone() })
    }
}
```

The `HasApiKeys` bound provides access to the key list from `AppState`. Alternatively,
`State<AppState>` can be accessed directly within `FromRequestParts` using the state type.

### Handler signature

```rust
async fn send_mail(
    State(state): State<AppState>,
    ctx: RequestContext,
    auth: AuthContext,      // ← auth is required; missing → 401/403 automatically
    StrictJson(payload): StrictJson<MailRequest>,
) -> Result<Json<SendResponse>, AppError> {
    // ...
}
```

If `AuthContext` extraction fails, Axum calls `IntoResponse` on the `AppError` rejection
and the handler body never runs.

## Implementation Plan

1. Define `AuthContext` in `src/auth.rs`.
2. Implement `extract_token`.
3. Implement `authenticate` (full-iteration, no early break).
4. Implement `FromRequestParts for AuthContext`.
5. Add `AuthContext` parameter to the send handler.
6. Write tests.

## Test Plan

### Unit Tests

- `extract_token` returns the bearer token from `Authorization: Bearer <tok>`.
- `extract_token` returns the value from `X-API-Key` when `Authorization` is absent.
- `extract_token` prefers `Authorization` when both headers present.
- `extract_token` returns `None` when neither header is present.
- `authenticate` returns `Ok` for a valid, enabled key.
- `authenticate` returns `Err(Forbidden)` for an incorrect token.
- `authenticate` returns `Err(Forbidden)` for a disabled key with correct secret.
- `authenticate` returns `Err(Unauthorized)` when token is `None`.
- `authenticate` returns `Err(Unauthorized)` when token is empty string.
- `authenticate` iterates all keys when first key matches (timing property).

### Integration Tests

- Request with no `Authorization` header → 401 JSON.
- Request with `Authorization: Bearer wrong` → 403 JSON.
- Request with correct bearer token → reaches handler.
- Request with `X-API-Key: correct` → reaches handler.
- `key_id` appears in log events after auth.

### Security Tests

- Disabled key with correct secret → 403 (not 200).
- Empty bearer token → 401.
- Bearer token with surrounding spaces → trimmed and evaluated correctly.
- Auth failure response does not reveal whether the key exists.

## Security Considerations

- The full-iteration design in `authenticate` is essential. Breaking on the first match would
  allow an attacker to measure the time difference between "matched the first key" and
  "matched the last key," revealing the position of a valid key.
- `AppError::Unauthorized` vs. `AppError::Forbidden`: the distinction is based on whether
  credentials were provided, not on whether the specific key was found. This prevents
  enumeration: an attacker cannot distinguish "no such key" from "key disabled."
- Token extraction must not log the extracted token value at any level.

## Operational Considerations

- Both `Authorization: Bearer` and `X-API-Key` are supported for compatibility with different
  HTTP client libraries. Document the preferred form (`Authorization: Bearer`).
- `key_id` in logs allows operators to trace which key was used for each request.

## Documentation Changes

- Document auth headers in `docs/api.md`.
- Document key management in `docs/configuration.md`.
- Document the security model in `docs/security.md`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-040-01 | Missing auth header returns 401. |
| AC-040-02 | Invalid token returns 403. |
| AC-040-03 | Disabled key with valid secret returns 403. |
| AC-040-04 | Valid key returns 200 (or appropriate success code). |
| AC-040-05 | `key_id` is set in `RequestContext` after successful auth. |
| AC-040-06 | `authenticate` iterates all keys without early termination. |

## Open Questions

None.
