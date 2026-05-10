# RFC 712 — HTTP Server TLS (HTTPS)

**Status.** Proposed  
**Tracks.** T1 — Security  
**Touches.** `Cargo.toml`, `src/config.rs`, `src/security.rs`,
             `crates/cli/src/main.rs`

## Summary

Add HTTPS support for the HTTP listener as an optional Cargo feature.
Implemented using `axum-server` with `rustls` (pure-Rust TLS, no OpenSSL).

## Feature flag

```
cargo build --release --features tls
```

Default build unchanged. Non-TLS build with `tls_cert` configured → startup error.

## Configuration

```toml
[server]
# Both must be set to enable HTTPS. Neither = HTTP (default).
tls_cert = "/etc/ssl/private/rele.crt"   # PEM certificate chain
tls_key  = "/etc/ssl/private/rele.key"   # PEM private key
```

Validation:
- `tls_cert` without `tls_key` → startup error.
- `tls_key` without `tls_cert` → startup error.
- Both present with non-TLS build → startup error.

## OpenBSD

`unveil(tls_cert, "r")` and `unveil(tls_key, "r")` in `apply_initial_restrictions`.
Cert and key are loaded into memory before runtime pledge.
No new pledge promises required (rustls operates entirely in-process).

## Dependency

```toml
axum-server = { version = "0.7", features = ["tls-rustls"], optional = true }

[features]
tls = ["dep:axum-server"]
```

## Deployment note

For most deployments (nginx / haproxy as reverse proxy), plain HTTP is
appropriate and TLS termination is the proxy's responsibility. Use `--features tls`
only for direct-expose deployments.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-712-01 | TLS build serves HTTPS with a valid PEM cert+key. |
| AC-712-02 | Config validation rejects one-field-missing configs. |
| AC-712-03 | Non-TLS build rejects `tls_cert` config at startup. |
| AC-712-04 | Plain HTTP mode unchanged when neither field is set. |
