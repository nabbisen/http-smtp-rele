# RFC 114 — Security Documentation

**Status.** Implemented  
**Tracks.** Release  
**Touches.** `docs/security.md`

## Summary

Write `docs/security.md` covering: open relay prevention model, API key handling, header
injection protection, logging policy, rate limiting, and reverse proxy guidance.

## Content outline

```markdown
# Security

## Open relay prevention
- Authentication required
- Recipient domain allowlist
- Fixed From address

## API key management
- Generating secure keys
- Key rotation procedure
- Per-key rate limits
- Revoking a key

## Header injection protection
- What fields are checked
- Why we reject rather than strip

## Logging and privacy
- What is logged
- What is never logged (secrets, body, full recipient)
- Recipient masking

## Rate limiting
- Three-tier model
- In-memory limitation
- Abuse response playbook (RFC 074)

## Reverse proxy guidance
- TLS requirement
- Binding to localhost only
- Restricting /readyz

## OpenBSD hardening
(brief; link to docs/openbsd.md)
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-114-01 | `docs/security.md` covers all listed sections. |
| AC-114-02 | Open relay prevention is explained clearly. |
| AC-114-03 | Reverse proxy TLS requirement is documented. |

## Open Questions

None.
