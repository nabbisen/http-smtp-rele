# RFC 302 — Multi-Recipient `to`

**Status.** Implemented  
**Tracks.** API / Mail  
**Touches.** `src/validation.rs`, `src/mail.rs`, `docs/api.md`

## Summary

Allow `to` to be a single string or an array of strings, enabling mail submission to
multiple recipients in one request.

## Design

```json
{"to": "alice@example.com", ...}              // still valid
{"to": ["alice@example.com", "bob@example.com"], ...}  // new
```

Serde: use `#[serde(deserialize_with)]` with a custom deserializer that accepts
both forms. Maximum recipients: `mail.max_recipients` (default 10).

Policy checks apply to every recipient independently. All must pass or the request fails.

lettre: call `.to(addr)` for each validated address.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-302-01 | Single string `to` continues to work. |
| AC-302-02 | Array `to` with valid addresses delivers to all recipients. |
| AC-302-03 | Array with any invalid or policy-denied address returns 400/422. |
| AC-302-04 | Array exceeding `max_recipients` returns 400/422. |
