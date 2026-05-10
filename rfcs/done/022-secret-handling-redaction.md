# RFC 022 — Secret Handling and Redaction

**Status.** Implemented  
**Tracks.** Foundation / Security  
**Touches.** `src/config.rs`, `src/auth.rs`

## Summary

Define `SecretString`, a wrapper type for API key secrets that prevents their accidental
exposure in `Debug`, `Display`, log output, or serialization, and document the policy for
secret handling throughout the codebase.

## Motivation

Rust's `derive(Debug)` automatically formats all struct fields. Without an explicit wrapper,
`ApiKeyConfig`'s `secret` field would be printed in logs whenever the config is logged for
diagnostic purposes, or when a handler panics and the state is dumped.
This violates `NFR-SEC-005`, `FR-053`, and `AC-010`.

## Scope

- `SecretString` wrapper type with redacted `Debug` and `Display`.
- Usage of `SecretString` in `ApiKeyConfig.secret`.
- A `constant_time_eq` method for timing-safe comparison.
- Policy: where `SecretString` is mandatory, where raw `String` is acceptable.
- Tests that explicitly verify the wrapper does not expose the secret.

## Non-goals

- Encrypted storage of secrets at rest (deferred; plain secrets in config for MVP).
- Key rotation mechanisms.
- Secret derivation or hashing at startup (future consideration).
- Zeroizing memory on drop (future consideration using `zeroize` crate).

## Design

### `SecretString`

```rust
/// A string that must not appear in logs, debug output, or error messages.
///
/// Use for API key secrets and any other value that must not be disclosed.
pub struct SecretString(String);

impl SecretString {
    /// Create a new secret from a plaintext string.
    pub fn new(s: String) -> Self {
        Self(s)
    }

    /// Expose the raw secret value for cryptographic comparison only.
    ///
    /// Callers must not pass the returned reference to any logging or
    /// formatting function.
    pub fn expose_secret(&self) -> &str {
        &self.0
    }

    /// Constant-time equality check against a candidate value.
    ///
    /// Uses `subtle::ConstantTimeEq` to resist timing attacks.
    pub fn constant_time_eq(&self, candidate: &str) -> bool {
        use subtle::ConstantTimeEq;
        self.0.as_bytes().ct_eq(candidate.as_bytes()).into()
    }
}

/// Never expose the secret in Debug output.
impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString([REDACTED])")
    }
}

/// Never expose the secret in Display output.
impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

/// Deserialize from TOML / JSON without exposing in error messages.
impl<'de> Deserialize<'de> for SecretString {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d).map(SecretString::new)
    }
}

/// Never serialize the secret value.
///
/// Serialization is only needed for debug tooling; production code must not
/// serialize `SecretString`.
impl Serialize for SecretString {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str("[REDACTED]")
    }
}
```

### Usage in `ApiKeyConfig`

```rust
#[derive(Debug, Deserialize)]
pub struct ApiKeyConfig {
    pub key_id: String,
    pub secret: SecretString,  // ← wrapped
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub description: Option<String>,
    #[serde(default)]
    pub allowed_recipient_domains: Vec<String>,
    #[serde(default)]
    pub allowed_recipients: Vec<String>,
    #[serde(default)]
    pub rate_limit_per_minute: u32,
    #[serde(default)]
    pub burst: u32,
}
```

Because `ApiKeyConfig` derives `Debug`, and `SecretString` implements `Debug` with redaction,
logging the entire `ApiKeyConfig` is safe.

### Constant-time comparison in auth

```rust
// In auth.rs
pub fn authenticate(
    token: &str,
    api_keys: &[ApiKeyConfig],
) -> Option<&ApiKeyConfig> {
    let mut found: Option<&ApiKeyConfig> = None;

    // Iterate all keys to avoid timing-based enumeration of key count.
    for key in api_keys {
        if key.enabled && key.secret.constant_time_eq(token) {
            found = Some(key);
            // Do NOT break early — iterate all keys for constant time.
        }
    }

    found
}
```

The loop does not `break` early to prevent timing attacks that could determine how many keys
exist or where in the list a matching key appears.

### Where `SecretString` is mandatory

| Location | Reason |
|----------|--------|
| `ApiKeyConfig.secret` | Primary secret |
| Any future credential field | Extend the policy |

### Where raw `String` is acceptable

| Location | Reason |
|----------|--------|
| Log fields (`key_id`) | Identifier, not secret |
| `allowed_recipient_domains` | Not secret |
| All other config fields | Not secret |

## Implementation Plan

1. Add `subtle = "2"` to `Cargo.toml`.
2. Create `SecretString` in `src/config.rs` (or a separate `src/secret.rs`).
3. Replace `String` with `SecretString` in `ApiKeyConfig.secret`.
4. Implement `constant_time_eq` using `subtle`.
5. Write tests.

## Test Plan

### Unit Tests

- `format!("{:?}", SecretString::new("abc".into()))` does not contain `"abc"`.
- `format!("{}", SecretString::new("abc".into()))` does not contain `"abc"`.
- `SecretString::constant_time_eq` returns `true` for equal strings.
- `SecretString::constant_time_eq` returns `false` for unequal strings.
- `serde_json::to_string(&SecretString::new("abc".into()))` does not contain `"abc"`.
- The full `ApiKeyConfig` Debug output does not contain the secret value.
- Auth returns `Some` for a valid key and `None` for an invalid key.
- Auth iterates all keys even after a match (timing property; verify via instrumentation
  or mock).

### Security Tests

- Logging `AppConfig` at `DEBUG` level does not expose any secret value.
- Logging an `ApiKeyConfig` struct does not expose the secret value.
- A timing test (approximate) shows that matching the first key vs. the last key takes
  similar time (order-of-magnitude; exact timing is environment-dependent).

## Security Considerations

- `expose_secret()` must only be called inside `constant_time_eq`. Any other caller is a
  potential bug and should be flagged in code review.
- The constant-time comparison uses `subtle::ConstantTimeEq` on byte slices. This mitigates
  timing attacks but does not eliminate them if the caller leaks timing through other means
  (e.g., early return from auth after the first match). The full-iteration loop in `authenticate`
  prevents this.
- `SecretString` does not zero memory on drop. In a future version, the `zeroize` crate can
  be added to clear the secret from memory after use.

## Operational Considerations

- Secrets are stored in plaintext in the config file. The config file must be protected with
  appropriate filesystem permissions (`640`, root:_http_smtp_rele).
- Log aggregation tools must not receive secret values. The `SecretString` wrapper ensures
  this even if a developer accidentally logs the config struct.

## Documentation Changes

- Document the secret handling policy in `docs/security.md`.
- Document the config file permission requirements in `docs/openbsd.md`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-022-01 | `SecretString` Debug and Display output does not contain the secret value. |
| AC-022-02 | `SecretString` serialization does not contain the secret value. |
| AC-022-03 | `SecretString::constant_time_eq` uses `subtle::ConstantTimeEq`. |
| AC-022-04 | `authenticate` iterates all keys without early termination on match. |
| AC-022-05 | Logging `AppConfig` at any level does not expose any secret. |

## Open Questions

- Whether to add a `zeroize` dependency to clear secrets from memory on drop. Deferred to
  v0.2; the priority for MVP is preventing log disclosure.
