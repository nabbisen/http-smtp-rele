# RFC 838 — AppError and ErrorCode Unification

**Status.** Proposed  
**Tracks.** T7 — Maintainability  
**Touches.** `src/error.rs`, `src/status.rs`

## Problem

`AppError` and `status::ErrorCode` are separate enums that express
overlapping concepts. `ErrorCode::SmtpRejected` exists but `AppError`
has no corresponding variant — when SMTP rejection is classified (RFC 810),
the mapping must be done manually in two places.

Any new error category must be added to both enums and kept in sync by
convention, not by the compiler.

## Design

Establish `ErrorCode` as the single source of truth for external error
classification. `AppError` maps to `ErrorCode` via a method:

```rust
pub enum ErrorCode {
    ValidationFailed,
    PayloadTooLarge,
    RateLimited,
    Unauthorized,
    Forbidden,
    NotFound,
    SmtpUnavailable,
    SmtpRejected,        // RFC 810
    InternalError,
    FeatureDisabled,     // RFC 823
    UnsupportedMediaType,
}

impl AppError {
    pub fn error_code(&self) -> ErrorCode { ... }
    pub fn http_status(&self) -> StatusCode { ... }
}
```

HTTP status codes and error code strings derive from `AppError::error_code()`,
removing the current duplicated match arms in `into_response()`.

`StatusUpdate` and `SubmissionStatusRecord` already use `ErrorCode` —
this change makes `AppError → ErrorCode` the single mapping path.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-838-01 | `AppError::error_code()` covers all `AppError` variants. |
| AC-838-02 | `AppError::into_response()` delegates to `error_code()` / `http_status()`. |
| AC-838-03 | No duplicate error-code string literals in `error.rs`. |
| AC-838-04 | All existing tests pass. |
