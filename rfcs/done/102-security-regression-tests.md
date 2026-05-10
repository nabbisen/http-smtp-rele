# RFC 102 — Security Regression Test Suite

**Status.** Implemented  
**Tracks.** Testing / Security  
**Touches.** `tests/security_tests.rs`

## Summary

Define the mandatory security regression test suite — a fixed set of tests that cover every
security control and must pass on every commit.

## Motivation

Security controls that are not continuously tested tend to regress as code evolves. A named
test suite makes regressions visible immediately and prevents security controls from being
silently removed (NFR-MNT-003, RFC 004 security gate).

## Design

### Required tests (SEC-NNN)

| ID | Test | Expected |
|----|------|----------|
| SEC-001 | No `Authorization` header | 401 `unauthorized` |
| SEC-002 | `Authorization: Bearer wrong-token` | 403 `forbidden` |
| SEC-003 | Disabled API key with correct secret | 403 `forbidden` |
| SEC-004 | `subject` containing `\r\nBcc: evil@evil.com` | 400 `validation_failed` |
| SEC-005 | `from_name` containing `\nX-Injected: hdr` | 400 `validation_failed` |
| SEC-006 | `reply_to` containing `\r\nX-Custom: h` | 400 `validation_failed` |
| SEC-007 | `to` containing `\r\nBcc: attacker@x.com` | 400 `validation_failed` |
| SEC-008 | Unknown field `"from": "evil@evil.com"` | 400 `validation_failed` |
| SEC-009 | Unknown field `"bcc": "evil@evil.com"` | 400 `validation_failed` |
| SEC-010 | Unknown field `"headers": {"X-Any": "v"}` | 400 `validation_failed` |
| SEC-011 | Body size > `max_request_body_bytes` | 413 `payload_too_large` |
| SEC-012 | `to` domain not in `allowed_recipient_domains` | 400 `validation_failed` |
| SEC-013 | Rate limit exceeded (send > global_per_minute) | 429 `rate_limited` |
| SEC-014 | `X-Forwarded-For` from untrusted peer | IP used is socket peer, not forwarded |
| SEC-015 | Auth failure log does not contain the token | Log assertion |
| SEC-016 | Successful send log does not contain body | Log assertion |
| SEC-017 | `ApiKeyConfig` Debug output does not contain secret | Unit test |

### Log assertion tests

Tests SEC-015 to SEC-017 require capturing tracing output. Use `tracing-test` crate or a
custom subscriber that collects log strings for assertion.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-102-01 | All SEC-001 through SEC-017 tests exist and pass. |
| AC-102-02 | These tests are in a named `security_tests` module/file. |
| AC-102-03 | These tests run as part of `cargo test`. |
| AC-102-04 | The RFC check script (RFC 003) would flag removal of this RFC. |

## Open Questions

None.
