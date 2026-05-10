# RFC 841 — AttachmentSpec content_type CR/LF and Control Char Validation

**Status.** Proposed  
**Tracks.** T3 — Validation  
**Touches.** `src/validation.rs`

## Problem

`AttachmentSpec.content_type` is validated only for the presence of `/`:

```rust
if !spec.content_type.contains('/') {
    return Err(AppError::Validation("invalid content_type".into()));
}
```

`content_type` is placed directly into a MIME `Content-Type` header.
A value containing CR (`\r`), LF (`\n`), or other control characters
could split or inject additional MIME headers in the assembled message.

The existing `contains_header_injection` check from `sanitize.rs` is
already used for `subject` and `from_name` but is not applied here.

## Fix

Apply the same sanitize checks used for other header-bound fields:

```rust
if !spec.content_type.contains('/')
    || sanitize::contains_header_injection(&spec.content_type)
    || sanitize::contains_control_chars(&spec.content_type)
{
    return Err(AppError::Validation(
        "attachments[].content_type: invalid format or forbidden characters".into(),
    ));
}
```

Also validate that the value loosely matches `type/subtype` format
(no spaces, printable ASCII only, optional `;` parameters).

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-841-01 | `content_type` with `\r\n` is rejected with 422. |
| AC-841-02 | `content_type` with control characters is rejected. |
| AC-841-03 | Valid values like `application/pdf` and `image/jpeg` pass. |
| AC-841-04 | Test covers injection attempt and valid types. |
