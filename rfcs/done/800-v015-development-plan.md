# RFC 800 — v0.15 Development Plan: Architect Review Remediation

**Status.** Proposed  
**Tracks.** Governance

## Background

Architect static review of v0.14.0 identified Release Blockers, High Priority,
and Medium Priority issues. This RFC is the parent plan.

## RFC inventory

### Release Blockers (must fix before any release)

| RFC | Issue | Summary |
|-----|-------|---------|
| 801 | RB-01 | NUL byte in src/validation.rs |
| 802 | RB-02 | `[[api_keys]]` → `[[security.api_keys]]` schema unification |
| 803 | RB-03 | Remove `require_auth` — auth is always required |
| 804 | RB-04 | `ConnectInfo` not injected — source IP always 127.0.0.1 |
| 805 | RB-05 | Status tracking order — received must be created before rate limit |
| 806 | RB-06 | `request_id` absent from error response JSON |

### High Priority

| RFC | Issue | Summary |
|-----|-------|---------|
| 807 | H-01 | API docs out of sync (UUID→ULID, html_body→body_html) |
| 808 | H-02 | Multipart message double Content-Type header |
| 809 | H-03 | `build_message` failure in `send_mail` leaves non-terminal status |
| 810 | H-04 | SMTP rejection vs unavailable not distinguished |
| 811 | H-05 | SIGHUP reload boundary: specify what is and is not reloadable |
| 812 | H-06 | README/docs show `cargo build --release` instead of workspace build |

### Medium Priority

| RFC | Issue | Summary |
|-----|-------|---------|
| 813 | M-01 | `StatusStore::put()` called twice — metrics double-counted |
| 814 | M-02 | StatusStore backend error silently surfaces as 404 |
| 815 | M-03 | `/readyz` inconsistent with pipe mode |
| 816 | M-04 | `status.enabled=false` still validates backend config |
| 817 | M-05 | HTTP 400 / 422 inconsistency between docs and implementation |
| 818 | M-06 | API contract needs MVP vs Extended split in docs |

## Implementation order

1. RB-01 (NUL byte — makes compilation uncertain)
2. RB-02 (schema unification — affects all users)
3. RB-03 (require_auth — security correctness)
4. RB-04 (ConnectInfo — rate limit and IP allowlist correctness)
5. RB-05 (status tracking order)
6. RB-06 (error response contract)
7. H-01 through H-06
8. M-01 through M-06
