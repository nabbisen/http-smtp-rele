# RFC 053 — Body and Subject Limits

**Status.** Implemented  
**Tracks.** Security  
**Touches.** `src/validation.rs`, `src/config.rs`

## Summary

Define the size limits for `subject` (character count) and `body` (byte count), how they
relate to the server-level body limit, and where in the pipeline each limit is enforced.

## Motivation

Large subjects waste log space and can confuse some SMTP servers. Large bodies exhaust memory
and SMTP bandwidth. The two limits serve different purposes and are configured independently:
the server-level limit (RFC 024) prevents memory exhaustion before JSON parsing; the mail-level
limits enforce business rules after parsing (FR-044, FR-045, NFR-PERF-003).

## Scope

- `max_subject_chars`: maximum UTF-8 character count for `subject`.
- `max_body_bytes`: maximum byte count for `body`.
- Relationship between `max_body_bytes` and `server.max_request_body_bytes`.
- Enforcement points.

## Non-goals

- Subject encoding (Quoted-Printable, Base64); handled by `lettre`.
- Attachment size limits (not in MVP).

## Design

### Limits

| Field | Config key | Default | Unit | Enforcement |
|-------|-----------|---------|------|-------------|
| `subject` | `mail.max_subject_chars` | 255 | UTF-8 chars | After deserialization, before sanitize |
| `body` | `mail.max_body_bytes` | 1,048,576 | bytes | After deserialization |
| Request | `server.max_request_body_bytes` | 1,048,576 | bytes | Middleware, before deserialization |

### Subject limit

Character count (not byte count) to handle multi-byte Unicode characters correctly:

```rust
fn validate_subject(subject: &str, config: &MailConfig) -> Result<(), AppError> {
    if subject.trim().is_empty() {
        return Err(AppError::Validation("subject: must not be empty".into()));
    }
    let char_count = subject.chars().count();
    if char_count > config.max_subject_chars {
        return Err(AppError::Validation(format!(
            "subject: exceeds maximum of {} characters",
            config.max_subject_chars
        )));
    }
    Ok(())
}
```

### Body limit

Byte count (important for SMTP bandwidth accounting):

```rust
fn validate_body(body: &str, config: &MailConfig) -> Result<(), AppError> {
    if body.is_empty() {
        return Err(AppError::Validation("body: must not be empty".into()));
    }
    if body.contains('\0') {
        return Err(AppError::Validation("body: contains NUL character".into()));
    }
    if body.len() > config.max_body_bytes {
        return Err(AppError::Validation(format!(
            "body: exceeds maximum of {} bytes",
            config.max_body_bytes
        )));
    }
    Ok(())
}
```

### Relationship between limits

Invariant enforced in config validation (RFC 021):
```
max_body_bytes ≤ max_request_body_bytes
```

Because the request body contains JSON wrapping around the body field, the effective body
limit is slightly less than the request limit. The config validation enforces that both
limits are set such that a body at `max_body_bytes` fits within a request at
`max_request_body_bytes` (there is enough headroom for the JSON structure).

Practical guideline: set `max_body_bytes` to 90% of `max_request_body_bytes` to leave room
for other request fields.

## Test Plan

### Unit Tests

- `subject` with exactly `max_subject_chars` characters → `Ok`.
- `subject` with `max_subject_chars + 1` characters → `Err`.
- Empty `subject` → `Err`.
- Whitespace-only `subject` → `Err`.
- `body` with exactly `max_body_bytes` bytes → `Ok`.
- `body` with `max_body_bytes + 1` bytes → `Err`.
- `body` containing NUL → `Err`.
- `body` with a multi-byte Unicode subject at limit → correct character counting.

## Security Considerations

- The body limit prevents memory exhaustion during validation and mail construction.
- The server-level limit (RFC 024) is the primary DoS defense; the mail-level limit is a
  secondary semantic check.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-053-01 | `subject` exceeding `max_subject_chars` characters returns 400. |
| AC-053-02 | Empty or whitespace-only `subject` returns 400. |
| AC-053-03 | `body` exceeding `max_body_bytes` bytes returns 400. |
| AC-053-04 | `body` containing NUL returns 400. |
| AC-053-05 | Subject character counting uses UTF-8 char count, not byte count. |

## Open Questions

None.
