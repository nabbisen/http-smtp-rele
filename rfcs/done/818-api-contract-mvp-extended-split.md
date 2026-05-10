# RFC 818 — M-06: API Contract MVP vs Extended Feature Split in Docs

**Status.** Proposed  
**Tracks.** T6 — Documentation  
**Touches.** `docs/api.md`, `docs/src/guides/api-reference.md`

## Problem

The implementation includes features well beyond the original MVP scope
(HTML body, CC, attachments, reply_to, bulk send, pipe mode, SMTP AUTH,
STARTTLS, Redis/SQLite stores) but the API reference treats them as if
they were all part of the core. This makes it hard for new users to
understand what is essential vs. optional.

## Decision

Organise the API reference in two tiers:

### Core (always available)

- `POST /v1/send` — `to`, `subject`, `body`
- `GET /healthz`, `GET /readyz`
- `GET /metrics`
- Authentication and rate limiting

### Extended (available, document separately)

- `POST /v1/send` extensions: `cc`, `body_html`, `reply_to`, `attachments`
- `POST /v1/send-bulk`
- `GET /v1/submissions/{request_id}` (status tracking)
- `GET /v1/keys/self`
- `[status] store = sqlite|redis` (requires feature flag builds)
- `[server] tls_cert/tls_key` (requires `--features tls`)

This structure helps operators understand the minimal viable configuration
and lets experienced users find advanced features.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-818-01 | API reference has clear Core and Extended sections. |
| AC-818-02 | Feature-flag-dependent features are clearly annotated. |
| AC-818-03 | Core section is sufficient for a new user to send a basic email. |
