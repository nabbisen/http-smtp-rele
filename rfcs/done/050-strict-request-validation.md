# RFC 050 — Strict Request Validation

**Status.** Implemented  
**Tracks.** Security  
**Touches.** `src/validation.rs`, `src/api/handlers.rs`

## Summary

Define `validate_mail_request` — the function that takes a deserialized `MailRequest` and
produces a `ValidatedMailRequest`, or returns a `AppError::Validation` with a specific
message if any field fails validation.

## Motivation

The JSON deserializer catches structural errors (wrong type, missing field, unknown field).
Validation catches semantic errors (malformed email address, oversized subject, CR/LF in
header-bound fields). Without a dedicated validation step, these checks are scattered across
handlers, easy to forget, and hard to test in isolation (FR-040–FR-046, FR-050–FR-053).

## Scope

- `validate_mail_request(request: MailRequest, config: &MailConfig, auth: &AuthContext) → Result<ValidatedMailRequest, AppError>`.
- Validation rules for `to`, `subject`, `body`, `from_name`, `reply_to`.
- `ValidatedMailRequest` — the type that proves validation has been completed.
- Order of validation checks.
- Error messages: field name + reason, no raw value echo.

## Non-goals

- CR/LF detection (RFC 051 — called from this function but defined separately).
- Recipient domain policy check (RFC 023 — called after validation).
- SMTP submission (RFC 061).

## Design

### `ValidatedMailRequest`

```rust
/// A `MailRequest` that has passed all validation checks.
///
/// Construction is only possible through `validate_mail_request`.
/// Downstream code (mail construction, SMTP) receives this type,
/// not the raw `MailRequest`.
pub struct ValidatedMailRequest {
    pub to: String,
    pub subject: String,
    pub body: String,
    pub from_name: Option<String>,
    pub reply_to: Option<String>,
}
```

Private constructor; only `validate_mail_request` can create it.

### Validation pipeline

```rust
pub fn validate_mail_request(
    req: MailRequest,
    config: &MailConfig,
    auth: &AuthContext,
) -> Result<ValidatedMailRequest, AppError> {
    validate_email_address("to", &req.to)?;
    sanitize::reject_header_crlf("to", &req.to)?;
    policy::check_recipient(&req.to, &auth.effective_recipient_policy)?;

    validate_subject(&req.subject, config)?;
    sanitize::reject_header_crlf("subject", &req.subject)?;

    validate_body(&req.body, config)?;

    if let Some(ref name) = req.from_name {
        sanitize::reject_header_crlf("from_name", name)?;
        validate_max_chars("from_name", name, 128)?;
    }

    if let Some(ref reply_to) = req.reply_to {
        validate_email_address("reply_to", reply_to)?;
        sanitize::reject_header_crlf("reply_to", reply_to)?;
    }

    Ok(ValidatedMailRequest {
        to: req.to,
        subject: req.subject,
        body: req.body,
        from_name: req.from_name,
        reply_to: req.reply_to,
    })
}
```

### Validation functions

**`validate_email_address(field, value)`**

Use `lettre::Address::from_str` (or equivalent) to check RFC 5321 / RFC 5322 address syntax.
On failure: `AppError::Validation(format!("{field}: invalid email address"))`.

**`validate_subject(subject, config)`**

```
- Not empty / not whitespace-only → "subject: must not be empty"
- Length ≤ config.max_subject_chars → "subject: exceeds maximum length"
```

**`validate_body(body, config)`**

```
- Not empty → "body: must not be empty"
- No NUL bytes → "body: contains NUL character"
- Length ≤ config.max_body_bytes → "body: exceeds maximum size"
```

**`validate_max_chars(field, value, max)`**

Generic per-field character limit.

### Error messages

Messages include the field name and the reason:
- `"to: invalid email address"`
- `"subject: must not be empty"`
- `"subject: exceeds maximum length of 255 characters"`
- `"body: exceeds maximum size of 1048576 bytes"`

Messages must NOT include the raw value (which could contain injected content or sensitive data).

## Implementation Plan

1. Create `src/validation.rs` with all validation functions.
2. Define `ValidatedMailRequest` with private fields.
3. Implement `validate_mail_request` with the pipeline above.
4. Call `validate_mail_request` from the send handler.
5. Write unit tests for every validation rule.

## Test Plan

### Unit Tests

- Valid request → `Ok(ValidatedMailRequest)`.
- `to` = `"not-an-email"` → `Err(Validation("to: invalid email address"))`.
- `subject` = `""` → `Err(Validation("subject: must not be empty"))`.
- `subject` exceeding `max_subject_chars` → `Err(Validation(...))`.
- `body` exceeding `max_body_bytes` → `Err(Validation(...))`.
- `body` containing NUL byte → `Err(Validation(...))`.
- `reply_to` = `"invalid"` → `Err(Validation("reply_to: invalid email address"))`.
- `from_name` = `"Name\r\nX-Injected: hdr"` → `Err` (CR/LF; handled by RFC 051 call).

### Security Tests

- Validation error messages do not include the raw field value.
- A request with a `from` field is rejected at deserialization (RFC 031, deny_unknown_fields),
  not here.

## Security Considerations

- The `ValidatedMailRequest` type is a proof token: if a function receives one, it can trust
  that all validation has passed. No handler should accept a raw `MailRequest` for mail
  construction or SMTP submission.
- Validation runs after auth (avoids wasting CPU on unauthenticated requests) but before
  policy checks and SMTP.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-050-01 | `validate_mail_request` returns `Ok` for a valid request. |
| AC-050-02 | Invalid `to` address returns `Err`. |
| AC-050-03 | Empty or whitespace-only `subject` returns `Err`. |
| AC-050-04 | Body exceeding `max_body_bytes` returns `Err`. |
| AC-050-05 | `ValidatedMailRequest` cannot be constructed outside `validation.rs`. |
| AC-050-06 | Error messages include field name but not the raw value. |

## Open Questions

None.
