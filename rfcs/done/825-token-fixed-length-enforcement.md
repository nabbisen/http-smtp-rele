# RFC 825 — Token Comparison: Fixed-Length Secret Enforcement

**Status.** Proposed  
**Tracks.** T1 — Security  
**Touches.** `src/auth.rs`, `src/config/validate.rs`

## Problem

The current constant-time comparison short-circuits on length mismatch:

```rust
if a.len() != b.len() {
    return false;
}
a.ct_eq(b)
```

The early return leaks whether the submitted token has the same length as
any configured secret. An attacker who knows the secret length can narrow
brute-force space.

## Decision

Enforce fixed-length secrets (32 raw bytes, typically base64-encoded as
43 characters) to make all secrets the same length, eliminating the
length-oracle.

```toml
[[security.api_keys]]
secret = "dGhpcyBpcyBleGFjdGx5IDMyIGJ5dGVzLWxvbmchISE"  # 32-byte base64
```

Config validation rejects secrets that are not 32+ bytes after decoding (or
equivalently, not of a standard encoded length). This is consistent with
RFC 824's minimum-length requirement.

Additionally, add a dummy comparison when no key matches to prevent
timing differences between "no such key ID" and "wrong secret":

```rust
let dummy = [0u8; 32];
subtle::ConstantTimeEq::ct_eq(&dummy, &dummy); // consume time
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-825-01 | Config validation enforces minimum secret length (RFC 824). |
| AC-825-02 | Auth path always performs a constant-time comparison (no early exit on ID mismatch). |
