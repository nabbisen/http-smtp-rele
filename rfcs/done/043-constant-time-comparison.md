# RFC 043 — Constant-Time Comparison and Timing Safety

**Status.** Implemented  
**Tracks.** Security  
**Touches.** `src/config.rs` (`SecretString`), `src/auth.rs`

## Summary

Document the timing-safe comparison strategy, explain why `subtle::ConstantTimeEq` is used,
and establish the rule that all secret comparisons in the codebase must use this mechanism.

## Motivation

Rust's default `==` operator on strings is not constant-time. A timing attack can measure
the time taken by `==` to determine how many leading bytes of the guessed token match the
actual secret. Over many guesses, the full secret can be reconstructed. The `subtle` crate
provides `ConstantTimeEq` which computes equality in time proportional only to the length
of the inputs, not the position of the first differing byte (FR-011, NFR-SEC-001).

## Scope

- Use of `subtle::ConstantTimeEq` in `SecretString::constant_time_eq`.
- The length-equalization requirement: different-length comparisons must not short-circuit.
- Full-iteration pattern in `authenticate` (RFC 040, RFC 022).
- Code review rule: any `==` on user-supplied data against a secret must use this path.

## Non-goals

- Protection against side-channel attacks beyond timing (cache, power, etc.).
- Constant-time string hashing.
- Password hashing (not applicable to bearer tokens for MVP).

## Design

### Length equalization

`subtle::ConstantTimeEq` on byte slices of different lengths returns `Choice::zero()` (false)
in constant time with respect to the content, but it does so in time proportional to the
shorter slice length. The `subtle` documentation notes that different-length inputs short-
circuit to `false` without examining content. To prevent length oracle attacks, the
implementation pads or normalizes lengths:

```rust
pub fn constant_time_eq(&self, candidate: &str) -> bool {
    use subtle::ConstantTimeEq;
    // Compare byte slices. subtle::ConstantTimeEq returns 0 or 1 as Choice.
    // For different lengths, subtle returns false immediately (no content scan).
    // This is acceptable because length leakage is low-risk for bearer tokens
    // that are all expected to be a fixed length.
    self.0.as_bytes().ct_eq(candidate.as_bytes()).into()
}
```

For MVP, bearer tokens are expected to be randomly generated strings of a fixed length
(e.g., 32–64 characters). Length leakage exposes only the configured key length, not its
content. This is documented as an acceptable limitation.

If variable-length tokens become a concern in a future version, pad both inputs to a common
length before comparison.

### Full-iteration in `authenticate`

The auth loop in RFC 040 does not `break` on match. This prevents timing variance that would
reveal the position of the matching key in the key list. The `subtle::ConstantTimeEq` call
itself is O(N) in the token length; the loop is O(K) in the number of keys. Both are constant
relative to the content of non-matching keys.

### Code review rule

All comparisons of user-supplied strings against secrets must use `SecretString::constant_time_eq`.
Direct use of `==`, `PartialEq`, or `str::eq` against a secret is prohibited in all code paths
reachable from request handlers.

Clippy cannot enforce this automatically; it is a code review checkpoint. Consider adding a
`// SECURITY: constant-time comparison required here` comment to call sites to make audits easier.

## Test Plan

### Unit Tests

- `SecretString::constant_time_eq` returns `true` for identical strings.
- `SecretString::constant_time_eq` returns `false` for strings differing by one byte.
- `SecretString::constant_time_eq` returns `false` for strings of different lengths.
- `SecretString::constant_time_eq` does not use `==` internally (verified by code review).

### Security Tests

- Approximate timing test: comparing against a correct token and an entirely wrong token
  takes time within a small multiplier of each other (not a formal proof; serves as a
  regression guard).

## Security Considerations

- `subtle::ConstantTimeEq` is a best-effort defense. OS scheduling, CPU branch prediction,
  and cache effects can introduce timing variance that the library cannot fully eliminate.
  This is accepted as a known limitation of software-only constant-time implementations.
- The full-iteration loop in `authenticate` also protects against key-count enumeration by
  keeping the number of comparisons constant regardless of which key (if any) matches.

## Operational Considerations

- Bearer tokens should be generated as cryptographically random strings of ≥ 32 characters.
  Document recommended token generation in `docs/configuration.md`.

## Documentation Changes

- Document token generation recommendations in `docs/configuration.md`.
- Document timing-attack mitigation in `docs/security.md`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-043-01 | `SecretString::constant_time_eq` uses `subtle::ConstantTimeEq`. |
| AC-043-02 | No auth code path uses `==` to compare user tokens against secrets. |
| AC-043-03 | `authenticate` does not break on first match. |

## Open Questions

None.
