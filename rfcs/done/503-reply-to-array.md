# RFC 503 — reply_to Array

**Status.** Proposed  
**Tracks.** API / Mail

## Summary

Extend `reply_to` to accept a string or array of addresses, consistent with
`to` (RFC 302) and `cc` (RFC 404).

## Design

```json
{ "reply_to": "support@example.com" }             // existing
{ "reply_to": ["alice@a.com", "bob@b.com"] }       // new
```

Reuse `Recipients` deserializer. Each address validated as before.
lettre: call `.reply_to(mb)` for each address.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-503-01 | Single string `reply_to` continues to work. |
| AC-503-02 | Array `reply_to` is accepted and forwarded. |
| AC-503-03 | Invalid address in array returns 422. |
