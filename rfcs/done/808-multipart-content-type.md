# RFC 808 — H-02: Multipart Message Content-Type Double-Setting

**Status.** Proposed  
**Tracks.** T4 — SMTP / Mail  
**Touches.** `src/mail.rs`

## Problem

`mail::build_message()` unconditionally applies `ContentType::TEXT_PLAIN`
to the builder before checking for HTML or attachments. When a multipart
message is built, the `Content-Type: text/plain` header conflicts with the
multipart boundary that lettre sets, causing a potentially malformed message.

## Fix

Apply `ContentType::TEXT_PLAIN` only when there is no HTML body and no
attachments (plain-text-only path):

```rust
if plain_only {
    // single-part plain text
    builder.body(validated.body.clone())
} else {
    // multipart: let lettre set Content-Type
    builder.multipart(multipart_body)
}
```

Add tests that verify the raw SMTP message received by the stub contains
the correct `Content-Type` header for:
1. Plain text only → `text/plain`
2. HTML + plain → `multipart/alternative`
3. Plain + attachment → `multipart/mixed`
4. HTML + plain + attachment → `multipart/mixed` wrapping `multipart/alternative`

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-808-01 | Plain-text-only message has `Content-Type: text/plain`. |
| AC-808-02 | HTML+plain message has `Content-Type: multipart/alternative`. |
| AC-808-03 | Attachment message has `Content-Type: multipart/mixed`. |
| AC-808-04 | No duplicate or conflicting `Content-Type` headers. |
