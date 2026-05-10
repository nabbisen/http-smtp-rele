# RFC 086 — Submission Status Store Abstraction

**Status.** Proposed  
**Tracks.** T5 — Abuse / Audit  
**Touches.** `src/status.rs`, `src/request_id.rs`

## Summary

This RFC defines a metadata-only submission status store abstraction.
It allows clients to query what http-smtp-rele observed during request handling
and SMTP submission. It does not make mail delivery asynchronous, does not
introduce a retry queue, and does not take ownership of SMTP delivery state.

## `RequestId` newtype

```rust
pub struct RequestId(String);  // "req_" + ULID
```

External format: `req_` followed by a ULID (26 uppercase alphanumeric chars).  
Must implement: `Display`, `FromStr`, `Serialize`, `Deserialize`, `Clone`, `Eq`, `Hash`.

## `Domain` newtype

```rust
pub struct Domain(String);  // lowercase domain extracted from email address
```

Stores only the domain part (after `@`). Full recipient addresses are never stored.

## `ErrorCode` enum

Reuses the same stable machine-readable strings as HTTP error responses.
No separate StatusStore-specific namespace.

```rust
pub enum ErrorCode {
    BadRequest, ValidationFailed, PayloadTooLarge, UnsupportedMediaType,
    RateLimited, Forbidden, SmtpUnavailable, SmtpRejected,
    InternalError, SubmissionNotFound,
}
```

External representation: `snake_case` string.

## `SubmissionStatus` enum

```rust
pub enum SubmissionStatus {
    Received, Rejected, SmtpSubmissionStarted, SmtpAccepted, SmtpFailed,
}
impl SubmissionStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Rejected | Self::SmtpAccepted | Self::SmtpFailed)
    }
}
```

`expired` is a lifecycle event, not a persisted status.

## `SubmissionStatusRecord`

```rust
pub struct SubmissionStatusRecord {
    pub request_id:        RequestId,
    pub key_id:            String,
    pub status:            SubmissionStatus,
    pub code:              Option<ErrorCode>,
    pub message:           Option<String>,
    pub recipient_domains: Vec<Domain>,
    pub recipient_count:   u32,
    pub created_at:        DateTime<Utc>,
    pub updated_at:        DateTime<Utc>,
    pub expires_at:        DateTime<Utc>,
}
```

`message` must contain only fixed/safe strings — never mail body or subject.

## `StatusStore` trait

```rust
pub trait StatusStore: Send + Sync {
    fn put(&self, record: SubmissionStatusRecord);
    fn update_status(&self, request_id: &RequestId, key_id: &str, update: StatusUpdate);
    fn get(&self, request_id: &RequestId, key_id: &str) -> Option<SubmissionStatusRecord>;
    fn expire_old_records(&self);
    fn record_count(&self) -> usize;
    fn reload_config(&self, config: &crate::config::StatusConfig);
}
```

## `enabled = false` behaviour

When disabled: no records are created; `GET /v1/submissions/` always returns 404.
`request_id` is still issued and logged.

## Data that must never be stored

Mail body, subject, raw SMTP message, attachments, API keys, Authorization headers,
full recipient addresses, or SMTP credentials.

## Terminal vs non-terminal states

| Status | Terminal |
|--------|----------|
| `received` | no |
| `smtp_submission_started` | no |
| `rejected` | yes |
| `smtp_accepted` | yes |
| `smtp_failed` | yes |

Non-terminal records from a crashed process are retained until TTL, then deleted.
Deletion returns 404; `expired` is not a stored status value.

## Security Considerations

The submission status store must remain metadata-only.  
Status values describe only what http-smtp-rele observed during HTTP request handling
and local SMTP submission. They must not be interpreted as final delivery, bounce,
retry, or mailbox state.
