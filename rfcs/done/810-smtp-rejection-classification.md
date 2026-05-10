# RFC 810 — H-04: SMTP Rejection vs Unavailability Classification

**Status.** Proposed  
**Tracks.** T4 — SMTP / T5 — Status Tracking  
**Touches.** `src/smtp.rs`, `src/error.rs`, `src/api/send.rs`, `src/api/send_bulk.rs`

## Problem

`smtp::submit()` maps all lettre errors to `AppError::SmtpUnavailable`.
SMTP 4xx/5xx rejections (e.g., relay denied, mailbox full, policy reject)
are indistinguishable from connection failures.

`ErrorCode::SmtpRejected` exists in the enum but is never set.

## Fix

Classify lettre errors:

| lettre error | AppError | ErrorCode |
|-------------|----------|-----------|
| Connection failure / timeout | `SmtpUnavailable` | `smtp_unavailable` |
| SMTP 4xx response | `SmtpRejected` | `smtp_rejected` |
| SMTP 5xx response | `SmtpRejected` | `smtp_rejected` |

Add `AppError::SmtpRejected` to the error enum if not already present.
Update status store writes in `send_mail` and `send_bulk` to use the correct
code.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-810-01 | TCP connection failure → `smtp_unavailable`. |
| AC-810-02 | SMTP 5xx rejection → `smtp_rejected` in status store and response. |
| AC-810-03 | Integration test covers each classification. |
