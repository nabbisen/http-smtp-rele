# RFC 807 — H-01: API Documentation Sync

**Status.** Proposed  
**Tracks.** T6 — Documentation  
**Touches.** `docs/api.md`, `docs/src/guides/api-reference.md`

## Problem

The API docs still contain:
- `"request_id": "550e8400-..."` (UUID v4 — should be `req_ + ULID`)
- "Every response includes a request_id (UUIDv4)"
- `html_body` field name (implementation uses `body_html`)
- `to` described as string-only (implementation accepts `string | string[]`)
- Missing: `cc`, `reply_to`, `attachments`, bulk send, status endpoint

These inaccuracies cause `unknown field` 400 errors when following the docs.

## Fix

Update both `docs/api.md` and `docs/src/guides/api-reference.md`:

| Doc | Fix |
|-----|-----|
| `request_id` format | `req_` + ULID |
| `html_body` | rename to `body_html` |
| `to` type | `string \| string[]` |
| `cc` | document |
| `reply_to` | `string \| string[]` |
| `attachments` | document |
| `GET /v1/submissions/{request_id}` | add as formal endpoint |
| `POST /v1/send-bulk` | add with per-message results |
| HTTP status codes | align with implementation (see RFC 817) |

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-807-01 | No UUID v4 examples remain in API docs. |
| AC-807-02 | `body_html` used consistently in docs and examples. |
| AC-807-03 | All documented fields are accepted by the implementation. |
| AC-807-04 | Status and bulk endpoints are documented. |
