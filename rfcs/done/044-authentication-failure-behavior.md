# RFC 044 — Authentication Failure Behavior

**Status.** Implemented  
**Tracks.** Security  
**Touches.** `src/auth.rs`, `src/logging.rs`, `src/error.rs`

## Summary

Define the precise behavior on authentication failure: which HTTP status to return, what to
log, and what NOT to reveal to the client.

## Motivation

Inconsistent or informative authentication error responses help attackers enumerate valid
keys and understand the key store structure. The error behavior must be deliberately minimal
and uniform (FR-013, NFR-SEC-001).

## Scope

- 401 vs. 403 semantics for auth failures.
- Log content on auth failure: what is safe to record.
- Response body: what is safe to return.
- Timing uniformity across failure modes.

## Non-goals

- Account lockout or blacklisting (not in MVP).
- Alerting on repeated failures (future).

## Design

### Response rules

| Situation | HTTP status | `code` |
|-----------|------------|--------|
| No Authorization header | 401 | `unauthorized` |
| `Authorization` header present but malformed | 401 | `unauthorized` |
| Valid format, no key matches | 403 | `forbidden` |
| Matching key is disabled | 403 | `forbidden` |

The response message is always:
- 401: `"Missing or invalid Authorization header"`
- 403: `"Authentication failed"`

Neither response reveals whether the key exists, is disabled, or how many keys are configured.

### Log content on failure

At `warn` level:

```
event=auth_failure client_ip=1.2.3.4 request_id=... reason=no_credentials
event=auth_failure client_ip=1.2.3.4 request_id=... reason=invalid_token
event=auth_failure client_ip=1.2.3.4 request_id=... reason=disabled_key key_id=service-a
```

Fields that must NOT appear in auth failure logs:
- The raw token value (even partial).
- The secret values of any configured key.
- The total number of configured keys.

`key_id` is logged only when the key is found but disabled (the key's existence is not
sensitive; the key_id appears in config and logs normally).

### Timing uniformity

All auth failure paths must exercise the same code:
- Missing credentials: call `authenticate(None, api_keys)` — the function still iterates all
  keys (with `ConstantTimeEq` returning false for every key against an empty slice).
- Invalid token: call `authenticate(Some(token), api_keys)` — same path, constant time.
- Disabled key: `authenticate` iterates all keys; disabled keys never match because the
  `enabled` check happens after `ConstantTimeEq`.

This ensures the response time is independent of whether zero, one, or all keys are tried.

## Test Plan

### Unit Tests

- `authenticate(None, keys)` returns `Unauthorized`.
- `authenticate(Some(""), keys)` returns `Unauthorized`.
- `authenticate(Some("wrong"), enabled_keys)` returns `Forbidden`.
- `authenticate(Some("correct"), disabled_key)` returns `Forbidden`.

### Integration Tests

- Missing auth header → 401, `code: "unauthorized"`, no token in response.
- Malformed bearer format → 401.
- Wrong token → 403, `code: "forbidden"`.
- Disabled key → 403, same response as wrong token.

### Security Tests

- Auth failure response does not include the submitted token.
- Auth failure response for "no key" and "disabled key" are identical.
- Time for 401 and 403 responses is within a small multiplier of each other (approximate).

## Security Considerations

- The 401 vs. 403 split does not reveal key existence: 401 means "no credentials at all";
  403 means "credentials provided but rejected." This is standard HTTP semantics.
- Logging `reason=disabled_key key_id=...` is acceptable because the `key_id` is not secret
  (it is a log label, not the secret). However, the reason must not be returned to the client.
- Auth failure logging at `warn` level feeds monitoring systems for anomaly detection.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-044-01 | No auth header → 401 `unauthorized`. |
| AC-044-02 | Wrong token → 403 `forbidden`. |
| AC-044-03 | Disabled key → 403 `forbidden` (same as wrong token). |
| AC-044-04 | Auth failure log includes `client_ip` and `request_id` but not the token. |
| AC-044-05 | Auth failure response body does not include the submitted token. |

## Open Questions

None.
