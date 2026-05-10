# RFC 502 — Attachment Support

**Status.** Proposed  
**Tracks.** API / Mail

## Summary

Add `attachments` field to `MailRequest` accepting base64-encoded file data.

## Design

```json
{
  "to": "user@example.com",
  "subject": "Report",
  "body": "See attached.",
  "attachments": [
    {
      "filename": "report.pdf",
      "content_type": "application/pdf",
      "data": "JVBERi0x..."
    }
  ]
}
```

Config:
```toml
[mail]
max_attachments          = 5
max_attachment_bytes     = 10485760   # 10 MB per file
```

Validation:
- `filename`: no `/`, `\`, NUL; max 255 chars
- `content_type`: valid MIME type string
- `data`: valid base64; decoded size ≤ `max_attachment_bytes`
- Count ≤ `max_attachments`

MIME structure:
- No attachments, text only → `text/plain`
- No attachments, text+html → `multipart/alternative`
- Attachments present → `multipart/mixed` wrapping the body part

lettre: `Attachment::new(filename).body(bytes, content_type)`

New dep: `base64 = "0.22"`

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-502-01 | Attachment data appears in SMTP stub message body. |
| AC-502-02 | Invalid base64 returns 422. |
| AC-502-03 | Attachment exceeding size limit returns 422. |
| AC-502-04 | Count > `max_attachments` returns 422. |
| AC-502-05 | Filename with `/` returns 422. |
