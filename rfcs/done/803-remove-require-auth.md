# RFC 803 — RB-03: Remove require_auth (Authentication Always Required)

**Status.** Proposed  
**Tracks.** T1 — Security  
**Touches.** `src/config.rs`, `src/auth.rs`, `src/config/validate.rs`, docs, tests

## Problem

`SecurityConfig` exposes `require_auth: bool` but `AuthContext::from_request_parts`
always requires a valid API key regardless of this setting. The flag has no
effect: setting `require_auth = false` does not disable authentication.

This is a configuration hazard: an operator might believe they have disabled
authentication when they have not, or conversely they may set it expecting it
to work in future.

## Decision

Remove `require_auth` from the configuration schema entirely.
Authentication is always required. Open relay is never supported.

This is a breaking change to the config schema, but the current phase permits it.

## Implementation

1. Remove `require_auth` field from `SecurityConfig`.
2. Remove all `require_auth` references in validation, docs, tests, and RFCs.
3. Update tests that set `require_auth = false` to remove the field.
4. Document "authentication is always required" in the security reference.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-803-01 | `SecurityConfig` has no `require_auth` field. |
| AC-803-02 | `grep -r 'require_auth'` finds no matches in src/ or tests/. |
| AC-803-03 | All existing auth tests pass. |
| AC-803-04 | docs/security.md explicitly states authentication is always required. |
