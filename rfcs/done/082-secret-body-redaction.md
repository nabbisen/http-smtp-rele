# RFC 082 — Secret and Body Log Redaction

**Status.** Implemented  
**Tracks.** Security / Ops  
**Touches.** `src/config.rs`, `src/auth.rs`, `src/api/handlers.rs`

## Summary

Document the enforcement points that prevent secrets and request bodies from appearing in logs,
and verify that `SecretString` (RFC 022) and `#[instrument(skip(...))]` (RFC 080) together
provide complete coverage.

## Motivation

Secrets and message bodies in logs create multiple risks: log aggregation tool exposure, log
file access control failures, and log-based forensic reconstruction of sensitive communication.
Proving that these values cannot reach log output requires explicit coverage checks
(FR-053, NFR-SEC-005, AC-010).

## Scope

- Inventory of all secret and body values in the codebase.
- Enforcement mechanism for each.
- Test that proves enforcement.

## Non-goals

- Recipient masking (RFC 083).
- Log format (RFC 084).

## Design

### Secret values

| Value | Location | Enforcement |
|-------|----------|-------------|
| `ApiKeyConfig.secret` | `SecretString` type | `Debug`/`Display` redaction (RFC 022) |
| Bearer token from request | `extract_token` result | Never assigned to a named field; discarded after auth |

### Body values

| Value | Enforcement |
|-------|-------------|
| `MailRequest.body` | `#[instrument(skip(payload))]` on handler |
| Full subject value | Same; only field name logged |
| Full recipient address | Masked to domain (RFC 083) |

### Verification tests

```rust
#[test]
fn secret_does_not_appear_in_debug_output() {
    let key = ApiKeyConfig {
        secret: SecretString::new("my-secret-value".into()),
        ..Default::default()
    };
    assert!(!format!("{key:?}").contains("my-secret-value"));
}
```

## Test Plan

### Security Tests

- `format!("{:?}", ApiKeyConfig { secret: ... })` does not contain secret.
- `format!("{:?}", AuthContext { ... })` does not contain secret.
- Send handler has `skip(payload)` in `#[instrument]`.

## Security Considerations

- Any future struct that derives `Debug` and contains a `SecretString` field gets redaction
  automatically. Other secret-like strings (e.g., future OAuth tokens) must also use `SecretString`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-082-01 | `SecretString` Debug/Display never exposes the inner value. |
| AC-082-02 | Send handler has `#[instrument(skip(payload))]`. |
| AC-082-03 | Automated tests assert that secret does not appear in Debug output. |

## Open Questions

None.
