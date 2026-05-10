# RFC 732 — Security Checklist

**Status.** Proposed  
**Tracks.** T1 — Security  
**Touches.** `docs/src/operations/security-checklist.md`

## Summary

A structured, actionable pre-deployment security checklist for operators.
Covers all known threat vectors: authentication, allowlisting, rate limiting,
network exposure, TLS, logging/privacy, OpenBSD hardening, and monitoring.

## Checklist categories

1. Authentication and API keys
2. Recipient and domain policy
3. Rate limiting
4. Network exposure
5. TLS and transport security
6. Logging and privacy
7. OpenBSD hardening (when applicable)
8. Monitoring and alerting
9. Operations and incident response

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-732-01 | Checklist covers all categories above. |
| AC-732-02 | Each item is specific and actionable (not vague). |
| AC-732-03 | Checklist references config fields and CLI options by exact name. |
