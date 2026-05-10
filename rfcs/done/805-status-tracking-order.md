# RFC 805 — RB-05: Status Tracking Order — received Before Rate Limit

**Status.** Proposed  
**Tracks.** T5 — Status Tracking  
**Touches.** `src/api/send.rs`, `src/api/send_bulk.rs`

## Problem

In `send_mail`, `status_store.put(received)` happens **after** all three
rate limit checks. Rate-limited requests are therefore never recorded.

Additionally, because the Axum `Json<MailRequest>` extractor runs before
the handler body, failures from:
- Invalid JSON body
- Unknown JSON fields
- Unsupported `Content-Type`
- Body deserialization failure

... never reach the handler at all, and cannot be recorded in the status store.

`send_bulk` calls `store_rejected()` for rate-limited messages, creating
a behavioral inconsistency with single send.

## Decision

### Single send: move received creation before rate limit checks

```text
auth success
→ status = received  (created here)
→ check global rate limit → rejected/rate_limited
→ check IP rate limit    → rejected/rate_limited
→ check key rate limit   → rejected/rate_limited
→ validate
→ rejected/validation_failed
→ smtp_submission_started
→ smtp_accepted / smtp_failed
```

### Single send: raw body extraction for full coverage

Replace `Json(payload): Json<MailRequest>` with manual extraction to capture
pre-deserialization failures:

```rust
pub async fn send_mail(
    State(state):                  State<Arc<AppState>>,
    ExtractRequestId(request_id):  ExtractRequestId,
    auth:                          AuthContext,
    headers:                       axum::http::HeaderMap,
    body:                          axum::body::Bytes,
) -> Result<(StatusCode, Json<Value>), AppError>
```

Inside the handler:
1. `status = received` created immediately after auth
2. Content-Type checked → `rejected/unsupported_media_type`
3. Body size checked → `rejected/payload_too_large`
4. `serde_json::from_slice` → `rejected/bad_request`
5. `validate_mail_request` → `rejected/validation_failed`
6. Normal SMTP pipeline

### Align send_bulk with send

`send_bulk` already creates `received` before rate limit for each message,
which is the correct behavior. After this RFC, both paths are consistent.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-805-01 | Rate-limited single sends appear in status store as `rejected/rate_limited`. |
| AC-805-02 | Validation-failed single sends appear as `rejected/validation_failed`. |
| AC-805-03 | `send_mail` and `send_bulk` status store behavior is consistent. |
| AC-805-04 | Integration tests cover rate limit and validation failure status records. |
