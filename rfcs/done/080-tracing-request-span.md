# RFC 080 — Tracing and Request Span Model

**Status.** Implemented  
**Tracks.** Ops  
**Touches.** `src/logging.rs`, `src/api/handlers.rs`, `src/context.rs`

## Summary

Define how `tracing` spans and fields are structured per request, so every log event within
a request automatically carries `request_id`, `client_ip`, and `key_id`.

## Motivation

Without a consistent request span, each log event must manually include correlation fields.
A `tracing` span created at the start of each request and extended as `key_id` becomes known
ensures automatic field propagation to all child events (NFR-SEC-007, NFR-OPS-004).

## Scope

- Per-request span creation in `RequestContextLayer`.
- Span field: `request_id`, `client_ip`.
- Span field extension: `key_id` added after auth.
- `#[instrument]` usage on handlers and major functions.
- Fields that must be skipped from `#[instrument]`.

## Non-goals

- Audit event taxonomy (RFC 081).
- Secret redaction (RFC 082).
- Log format (RFC 084).

## Design

### Request span

Created in `RequestContextLayer` before any handler runs:

```rust
let span = tracing::info_span!(
    "request",
    request_id = %ctx.request_id,
    client_ip = %ctx.client_ip,
    key_id = tracing::field::Empty,  // filled in after auth
    method = %req.method(),
    path = %req.uri().path(),
);
let _guard = span.enter();
```

`key_id` starts as `Empty`. It is recorded after auth:

```rust
// In auth extractor, after key is found:
tracing::Span::current().record("key_id", &key.key_id.as_str());
```

### `#[instrument]` convention

```rust
#[instrument(
    skip(state, payload),           // never instrument secrets or bodies
    fields(
        smtp_host = %state.config.smtp.host,  // safe config fields only
    )
)]
async fn send_mail(
    State(state): State<AppState>,
    ctx: RequestContext,
    auth: AuthContext,
    StrictJson(payload): StrictJson<MailRequest>,
) -> Result<Json<SendResponse>, AppError> { ... }
```

Rules:
- Always `skip` the full payload, body, and any auth headers.
- Include safe identifiers (`request_id`, `key_id`) via span fields, not `#[instrument]` fields.
- Do not instrument `SecretString` fields under any circumstances.

### Span structure for a successful request

```
SPAN request [request_id=abc, client_ip=127.0.0.1]
  EVENT info  config loaded
  EVENT info  auth succeeded [key_id=service-a]  ← key_id recorded in span
  EVENT info  validation passed
  EVENT info  smtp accepted  [recipient_domain=example.com]
```

### Span structure for a failure

```
SPAN request [request_id=abc, client_ip=1.2.3.4]
  EVENT warn  auth_failure [reason=invalid_token]
```

## Test Plan

### Integration Tests

- Every log event within a request includes `request_id`.
- `key_id` appears in log events after auth, not before.
- Payload body does not appear in any log event.

## Security Considerations

- `tracing::field::Empty` for `key_id` ensures the span does not carry the empty string before
  auth; the field is simply absent.
- `skip(payload)` on the handler ensures the entire `MailRequest` struct is never formatted
  into the span, even at `TRACE` level.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-080-01 | Every request has a `tracing` span with `request_id` and `client_ip`. |
| AC-080-02 | `key_id` is recorded in the span after successful auth. |
| AC-080-03 | `#[instrument]` on the send handler skips `payload`. |

## Open Questions

None.
