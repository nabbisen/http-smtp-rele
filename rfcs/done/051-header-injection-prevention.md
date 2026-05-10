# RFC 051 — Header Injection Prevention

**Status.** Implemented  
**Tracks.** Security  
**Touches.** `src/sanitize.rs`, `src/validation.rs`

## Summary

Define `sanitize::reject_header_crlf` — a function that **rejects** (not strips) any string
containing `\r` or `\n`, applied to all fields that will appear in email headers.

## Motivation

Email header injection occurs when a client-controlled string containing `\r\n` is placed
into a header field. The injected bytes can add new headers (e.g., `Bcc:`, `Content-Type:`),
break the MIME structure, or enable spam. **Stripping** newlines silently accepts the injection
attempt; **rejecting** makes the attack visible in logs and forces the client to fix the request
(FR-050, FR-051, AC-004).

## Scope

- `sanitize::reject_header_crlf(field: &str, value: &str) → Result<(), AppError>`.
- Fields covered: `to`, `subject`, `from_name`, `reply_to`.
- Rejection (not stripping): the function returns an error; it does not modify the input.
- Integration point: called from `validate_mail_request` (RFC 050).

## Non-goals

- Other control character filtering (deferred; CR/LF is the primary injection vector).
- NUL byte handling (covered in RFC 050 body validation).
- Sanitizing `metadata` fields (not used in email construction).

## Design

### `reject_header_crlf`

```rust
/// Rejects any string that contains CR (`\r`) or LF (`\n`).
///
/// These characters are the primary vector for email header injection.
/// This function REJECTS; it does not strip or replace.
pub fn reject_header_crlf(field: &str, value: &str) -> Result<(), AppError> {
    if value.contains('\r') || value.contains('\n') {
        tracing::warn!(
            field = field,
            event = "header_injection_attempt",
            "CR/LF detected in header-bound field"
        );
        return Err(AppError::Validation(format!(
            "{field}: CR or LF characters are not allowed"
        )));
    }
    Ok(())
}
```

### Fields and call sites

| Field | When checked |
|-------|-------------|
| `to` | `validate_mail_request` after email parse |
| `subject` | `validate_mail_request` after empty/length check |
| `from_name` | `validate_mail_request` if `from_name` is `Some` |
| `reply_to` | `validate_mail_request` after email parse if `Some` |

Note: `to` and `reply_to` are also validated as email addresses by `lettre`, which typically
rejects embedded newlines. The `reject_header_crlf` call is an additional explicit guard that
does not rely on the email parser's behavior.

### Why reject rather than strip

1. **Visibility**: A rejection is logged as an `header_injection_attempt` event. Stripping
   would silently accept malformed input, making attacks invisible in audit logs.
2. **Correctness**: A subject containing `\r\n` is almost certainly an error or attack. There
   is no legitimate use case for a subject with embedded newlines.
3. **Defense depth**: `lettre` constructs the MIME message safely. Even if `lettre` handles
   newlines correctly, an explicit rejection at the application layer means the attack is
   stopped and logged before reaching the library.

### Log event

The warning log must include the field name but must NOT include the field value (which
could contain further injected content or confidential data). Only the field name is logged.

## Test Plan

### Unit Tests

- `reject_header_crlf("subject", "Normal subject")` → `Ok`.
- `reject_header_crlf("subject", "line1\r\nBcc: evil")` → `Err`.
- `reject_header_crlf("subject", "line1\nBcc: evil")` → `Err`.
- `reject_header_crlf("subject", "line1\rBcc: evil")` → `Err`.
- `reject_header_crlf("from_name", "Name\nX-Injected: h")` → `Err`.
- `reject_header_crlf("to", "user@example.com")` → `Ok`.

### Security Tests (regression)

- `subject` = `"Hello\r\nBcc: attacker@evil.com"` → 400 with `validation_failed`.
- `from_name` = `"Name\r\nX-Custom: injected"` → 400.
- `reply_to` = `"ok@a.com\r\nBcc: b@b.com"` → 400.
- `to` = `"user@example.com\r\nBcc: attacker@evil.com"` → 400.

These tests must be retained in the security regression suite permanently (RFC 102).

## Security Considerations

- The audit log event `header_injection_attempt` signals an active attack or a severely
  broken client. Operators should monitor for repeated occurrences from a single IP or key.
- The function does not log the offending value, only the field name. This prevents log
  injection via the field value itself.
- Both `\r` and `\n` are checked individually: some injection attempts use only `\n` (bare LF),
  which is technically a violation of RFC 5322 but may be misinterpreted by some SMTP servers.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-051-01 | `reject_header_crlf` returns `Err` for any string containing `\r` or `\n`. |
| AC-051-02 | Applied to `to`, `subject`, `from_name`, `reply_to` before SMTP. |
| AC-051-03 | Rejection is logged with field name but not field value. |
| AC-051-04 | Stripping is not performed; the request is rejected. |
| AC-051-05 | Security regression tests for CR/LF injection exist and pass. |

## Open Questions

None.
