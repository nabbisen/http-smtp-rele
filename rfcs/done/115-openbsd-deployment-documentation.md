# RFC 115 — OpenBSD Deployment Documentation

**Status.** Implemented  
**Tracks.** Release  
**Touches.** `docs/openbsd.md`

## Summary

Write `docs/openbsd.md` as the complete guide for deploying `http-smtp-rele` on OpenBSD,
covering user creation, file placement, rc.d setup, pledge/unveil behavior, and OpenSMTPD
integration.

## Content outline

```markdown
# Deploying on OpenBSD

## Prerequisites
- OpenBSD version
- OpenSMTPD running

## System user
useradd command

## File placement and permissions
(table from RFC 092)

## rc.d script
(content from examples/)

## OpenSMTPD configuration
(from RFC 093)

## pledge and unveil
- What promise set is applied
- When it is applied
- What happens if a violation occurs

## Verifying the deployment
curl examples

## Upgrading
Binary replacement + rcctl restart
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-115-01 | `docs/openbsd.md` covers all listed sections. |
| AC-115-02 | The rc.d script in the docs matches `examples/openbsd-rc.d-http_smtp_rele`. |
| AC-115-03 | pledge/unveil behavior is explained. |

## Open Questions

None.
