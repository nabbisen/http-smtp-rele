# RFC 809 — H-03: build_message Failure Sets Terminal Status in send_mail

**Status.** Proposed  
**Tracks.** T5 — Status Tracking  
**Touches.** `src/api/send.rs`

## Problem

In `send_mail`, if `mail::build_message()` fails, the `?` operator returns
the error immediately without updating the status store. The record remains
in `Received` state — a non-terminal state — until TTL expiry.

`send_bulk` correctly catches build errors and sets `Rejected/InternalError`.

## Fix

```rust
let message = mail::build_message(&validated, &cfg).map_err(|e| {
    state.status_store.update_status(&request_id, &auth.key_id, StatusUpdate {
        status:  SubmissionStatus::Rejected,
        code:    Some(ErrorCode::InternalError),
        message: Some("Failed to build mail message.".into()),
    });
    e
})?;
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-809-01 | A build failure in `send_mail` sets status to `Rejected/InternalError`. |
| AC-809-02 | `send_mail` and `send_bulk` handle build failures consistently. |
