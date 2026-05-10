# RFC 819 — v0.15 Non-Functional Review Plan

**Status.** Proposed  
**Tracks.** Governance

## Background

Architect non-functional static review of v0.14.0 identified Release Blockers,
High Priority, and Medium Priority issues across security, stability,
performance, concurrency, memory safety, and attack surface.

## Cross-reference: issues already covered by functional review RFCs

| NF issue | Existing RFC |
|---------|-------------|
| ConnectInfo not injected | RFC 804 |
| StatusStore backend error → 404 | RFC 814 (extended by RFC 820) |
| require_auth = false | RFC 803 |
| SIGHUP reload boundary | RFC 811 |
| docs/RFC/implementation gap | RFC 807, 818 |
| StatusStore put() double metrics | RFC 813 |

## New RFCs from non-functional review

### Release Blockers

| RFC | Issue |
|-----|-------|
| 820 | OpenBSD pledge/unveil application order |
| 821 | StatusStore blocking I/O in async handlers |
| 822 | /metrics unauthenticated access |
| 823 | Attack surface: HTML/attachments/bulk/pipe feature gates |

### High Priority

| RFC | Issue |
|-----|-------|
| 824 | API key secret security policy (min length, file permissions) |
| 825 | Token comparison: fixed-length enforcement |
| 826 | Attachment pre-decode size check and total size limit |
| 827 | body_html default-off policy |
| 828 | Full recipient address removal from error logs |

### Medium Priority

| RFC | Issue |
|-----|-------|
| 829 | Mutex/RwLock poison handling |
| 830 | Redis TTL: remaining TTL on update, not full reset |
| 831 | Redis key prefix configurable |
| 832 | record_count() lightweight implementation for Redis |
| 833 | /readyz light vs deep mode clarification |
| 834 | /v1/keys/self policy info exposure |

### Performance / Concurrency / Architecture

| RFC | Issue |
|-----|-------|
| 835 | Rate limiter global lock: documentation and improvement path |
| 836 | Bulk send memory limits (total attachment bytes) |
| 837 | HTTP timeout / SMTP timeout coordination |
| 838 | AppError and ErrorCode unification |
| 839 | StatusStore async trait (long-term) |
| 840 | MailTransport trait abstraction (long-term) |
| 841 | AttachmentSpec.content_type CR/LF validation |
