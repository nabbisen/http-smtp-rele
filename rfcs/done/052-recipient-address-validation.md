# RFC 052 — Recipient Address Validation

**Status.** Implemented  
**Tracks.** Security  
**Touches.** `src/validation.rs`, `src/policy.rs`

## Summary

Define the two-layer check on the `to` field: syntactic validation (is it a valid RFC 5321
email address?) followed by policy check (is the domain or address permitted by config?).

## Motivation

An invalid `to` address would cause SMTP rejection after needlessly connecting to the server.
An address in a disallowed domain would constitute an open-relay violation. Both must be caught
before SMTP contact (FR-043, FR-044, FR-046, NFR-SEC-001, AC-005).

## Scope

- Syntactic validation of `to` via `lettre` address parsing.
- Domain extraction from validated address.
- Policy check (RFC 023) integration.
- Error messages for each failure mode.

## Non-goals

- MX record lookups (too slow; SMTP handles delivery errors).
- Multi-recipient support (deferred; MVP is single `to`).
- `cc`/`bcc` (not in MVP).

## Design

### Validation layer

`lettre::Address::from_str(&req.to)` is used for RFC 5321 syntactic validation.

```rust
fn validate_email_address(field: &str, value: &str) -> Result<lettre::Address, AppError> {
    value.parse::<lettre::Address>().map_err(|_| {
        AppError::Validation(format!("{field}: invalid email address"))
    })
}
```

This call is made inside `validate_mail_request` (RFC 050).

### Domain extraction

After parsing succeeds:

```rust
fn domain_of(addr: &lettre::Address) -> &str {
    addr.domain()
}
```

The domain is used for the recipient policy check (RFC 023).

### Policy check

Immediately after syntactic validation in the pipeline:

```rust
let to_addr = validate_email_address("to", &req.to)?;
sanitize::reject_header_crlf("to", &req.to)?;
policy::check_recipient(to_addr.as_ref(), &auth.effective_recipient_policy)?;
```

`policy::check_recipient` maps `PolicyError` to `AppError::Validation`.

### Error codes on failure

| Failure | HTTP | `code` |
|---------|------|--------|
| Invalid syntax | 400 | `validation_failed` |
| Domain not in allowlist | 400 | `validation_failed` |

Both map to `validation_failed`; the messages differ:
- Syntax: `"to: invalid email address"`
- Policy: `"to: recipient domain is not permitted"`

The policy rejection message must NOT include the disallowed domain in the client response
(to avoid confirming the allowlist). It MAY be included in the server log.

## Test Plan

### Unit Tests

- `"user@example.com"` → valid.
- `"not-an-email"` → `Err`.
- `"user@"` → `Err`.
- `"@example.com"` → `Err`.
- `"user@example.com"` with domain in allowlist → policy `Ok`.
- `"user@evil.com"` with `allowed_recipient_domains = ["example.com"]` → policy `Err`.

### Security Tests

- Request to a disallowed domain returns 400, not 200 or 502.
- The allowlist domain is not returned in the error response.
- CR/LF in `to` is caught by `reject_header_crlf` before reaching policy.

## Security Considerations

- Policy check runs after syntactic validation to ensure the domain is well-formed before
  comparisons. Comparison against a malformed domain could produce unexpected results.
- Error messages for policy failure must not leak the allowlist configuration to clients.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-052-01 | Invalid email syntax in `to` returns 400. |
| AC-052-02 | `to` domain not in allowlist returns 400. |
| AC-052-03 | Policy rejection message does not expose allowlist contents. |

## Open Questions

None.
