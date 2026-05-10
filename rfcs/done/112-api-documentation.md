# RFC 112 — API Documentation

**Status.** Implemented  
**Tracks.** Release  
**Touches.** `docs/api.md`

## Summary

Write `docs/api.md` documenting all HTTP endpoints, request/response schemas, error codes,
and request examples.

## Content outline

```markdown
# API Reference

## Authentication
- Bearer token header
- X-API-Key fallback

## POST /v1/send
### Request
- Headers required
- Request body schema (all fields with types, required/optional)
- Examples: minimal, full

### Response
- 202 Accepted shape
- All error codes and their meanings

## GET /healthz
## GET /readyz

## Error codes
| Code | HTTP | Meaning |
|------|------|---------|
...

## request_id
- How to use it for support requests
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-112-01 | `docs/api.md` documents all endpoints. |
| AC-112-02 | All error codes from RFC 032 are listed. |
| AC-112-03 | Request and response examples are included. |

## Open Questions

None.
