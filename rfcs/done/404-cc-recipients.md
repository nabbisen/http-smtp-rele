# RFC 404 — cc Recipients

**Status.** Implemented  
**Tracks.** API / Mail

## Summary

Add optional `cc` field (string or array) to `MailRequest`, applying the same validation
pipeline as `to` (RFC 302).

## Design

```json
{
  "to": "alice@example.com",
  "cc": "bob@example.com",
  "subject": "Hello",
  "body": "Text."
}
```

- Accepts string or array (reuses `Recipients` deserializer from RFC 302).
- Each cc address: email format validation, CR/LF check, domain/address policy.
- Combined `to + cc` count must not exceed `mail.max_recipients`.
- lettre: `.cc(mailbox)` for each address.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-404-01 | `cc` string or array is accepted and appears in SMTP DATA. |
| AC-404-02 | Invalid `cc` address returns 400/422. |
| AC-404-03 | `to + cc` exceeding `max_recipients` returns 400/422. |
