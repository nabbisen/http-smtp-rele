# RFC 042 — API Key Policy and Per-Key Permissions

**Status.** Implemented  
**Tracks.** Security  
**Touches.** `src/auth.rs`, `src/policy.rs`, `src/config.rs`

## Summary

Define how per-key permissions (`allowed_recipient_domains`, `allowed_recipients`,
`rate_limit_per_minute`, `burst`) override global settings, and how `AuthContext` exposes
effective policy to the handler pipeline.

## Motivation

Different API keys may have different trust levels. A key used by an internal billing service
might be allowed to send to any `@company.com` address, while a key used by a third-party
integration might be restricted to a single recipient. Per-key configuration makes this
expressible without separate deployments (FR-012, FR-031).

## Scope

- Effective recipient policy derivation from key + global config (see RFC 023 for algorithm).
- Effective rate limit derivation from key + global config.
- `AuthContext` exposing `effective_recipient_policy` and `effective_rate_limits`.
- Validation that key-level policies are more restrictive, not less, than global policies
  (no escalation).

## Non-goals

- Policy enforcement (RFC 023, RFC 070).
- Authentication (RFC 040).
- Adding new per-key fields beyond MVP.

## Design

### Effective rate limit

```rust
pub struct EffectiveRateLimits {
    pub per_minute: u32,
    pub burst: u32,
}

impl EffectiveRateLimits {
    pub fn for_key(key: &ApiKeyConfig, global: &RateLimitConfig) -> Self {
        Self {
            per_minute: if key.rate_limit_per_minute > 0 {
                key.rate_limit_per_minute
            } else {
                global.per_key_per_minute
            },
            burst: if key.burst > 0 {
                key.burst
            } else {
                global.per_key_burst
            },
        }
    }
}
```

A key with `rate_limit_per_minute = 0` inherits the global `per_key_per_minute`. A key with
a non-zero value uses its own limit regardless of whether it is more or less restrictive.

Note: RFC 023 handles recipient domain policy derivation.

### `AuthContext` extensions

```rust
#[derive(Clone, Debug)]
pub struct AuthContext {
    pub key: ApiKeyConfig,
    pub effective_rate_limits: EffectiveRateLimits,
    pub effective_recipient_policy: RecipientPolicy,
}
```

Constructed in the auth extractor after the key is found:

```rust
let effective_rate_limits = EffectiveRateLimits::for_key(&key, &state.config.rate_limit);
let effective_recipient_policy =
    effective_recipient_policy(&key, &state.config.mail);

Ok(AuthContext {
    key: key.clone(),
    effective_rate_limits,
    effective_recipient_policy,
})
```

## Test Plan

### Unit Tests

- Key with `rate_limit_per_minute = 0` inherits global default.
- Key with `rate_limit_per_minute = 10` uses 10, regardless of global setting.
- Effective recipient policy follows RFC 023 precedence rules.

## Security Considerations

- Per-key rate limits can be set higher than global defaults; this is intentional for trusted
  internal services. The global rate limit is a hard ceiling that per-key settings cannot exceed.
  Consider enforcing this in validation (RFC 021): `key.rate_limit_per_minute ≤ global.global_per_minute`.
  Decision: document as a responsibility of the operator; not enforced in MVP.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-042-01 | Key with `rate_limit_per_minute = 0` inherits global `per_key_per_minute`. |
| AC-042-02 | Key with non-zero `rate_limit_per_minute` uses its own value. |
| AC-042-03 | `AuthContext` carries `effective_rate_limits` and `effective_recipient_policy`. |

## Open Questions

None.
