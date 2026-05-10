# RFC 602 — GET /v1/keys/self — Authenticated Key Info

**Status.** Proposed  
**Tracks.** T2 — HTTP API  
**Touches.** `src/api/keys.rs`, `src/api/mod.rs`

## Summary

Add `GET /v1/keys/self` returning the non-secret configuration of the currently
authenticated API key. This gives clients a way to verify their effective policy
without administrator access to the config file.

## Response

```http
GET /v1/keys/self
Authorization: Bearer <token>
```

```json
{
  "id": "service-a",
  "enabled": true,
  "description": "Production notification service",
  "allowed_recipient_domains": ["example.com"],
  "allowed_recipients": [],
  "rate_limit_per_min": 30,
  "burst": 0,
  "mask_recipient": null
}
```

- `secret` is never returned.
- `rate_limit_per_min: null` means the global default applies.
- `mask_recipient: null` means the global setting applies.

## Authentication

Standard `Authorization: Bearer` required. Returns 401/403 as usual.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-602-01 | 200 with key config for valid auth. |
| AC-602-02 | `secret` is absent from the response. |
| AC-602-03 | 401/403 for missing/invalid auth. |
