# RFC 403 — HTML Body (multipart/alternative)

**Status.** Implemented  
**Tracks.** API / Mail

## Summary

Add optional `body_html` field to `MailRequest`. When both `body` and `body_html` are
present, build a `multipart/alternative` message with text/plain and text/html parts.

## Design

```json
{
  "to": "user@example.com",
  "subject": "Hello",
  "body": "Hello in plain text.",
  "body_html": "<h1>Hello</h1><p>In HTML.</p>"
}
```

- `body` remains required (plain text fallback).
- `body_html` is optional.
- Both undergo the same size limits (`max_body_bytes` applies to the larger of the two).
- `body_html` is validated for NUL bytes but **not** HTML-sanitised (out of scope).

lettre multipart:
```rust
let body = MultiPart::alternative()
    .singlepart(SinglePart::plain(validated.body))
    .singlepart(SinglePart::html(validated.body_html));
message_builder.multipart(body)
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-403-01 | Request with only `body` produces a `text/plain` message (unchanged). |
| AC-403-02 | Request with `body` and `body_html` produces `multipart/alternative`. |
| AC-403-03 | SMTP stub receives both parts in the message body. |
