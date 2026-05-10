# RFC 824 — API Key Secret Security Policy

**Status.** Proposed  
**Tracks.** T1 — Security  
**Touches.** `src/config/validate.rs`, `src/auth.rs`, docs

## Problem

API key secrets are stored as plaintext in the TOML config file with no
minimum-length enforcement. The example config ships placeholder values
that operators may accidentally use in production.

## Decision

### Minimum secret length enforcement

```rust
const MIN_SECRET_BYTES: usize = 32;

if key.secret.expose_secret().len() < MIN_SECRET_BYTES {
    return Err(ConfigError::Validation(
        format!("api_keys[{}].secret: minimum {} bytes required", key.id, MIN_SECRET_BYTES)
    ));
}
```

### Block example/placeholder secrets

Reject known example values at startup:

```rust
const BLOCKED_SECRETS: &[&str] = &[
    "your-secret-here",
    "generate-with-openssl-rand-base64-32",
    "changeme",
    "secret",
];
```

### Config file permission check (OpenBSD / Linux)

At startup, warn if the config file is world-readable:

```sh
stat /etc/http-smtp-rele.toml → mode 0644 → WARN: config file world-readable, secrets exposed
```

Recommended: `chmod 640` with group `_http_smtp_rele` / `http-smtp-rele`.

### Documentation

- Security checklist: add explicit API key generation command
- `examples/http-smtp-rele.toml`: comment must show `openssl rand -base64 32` output format

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-824-01 | Config with secret shorter than 32 bytes fails validation. |
| AC-824-02 | Config with placeholder secret fails validation. |
| AC-824-03 | World-readable config file logs a WARNING at startup. |
| AC-824-04 | Security checklist documents minimum key generation command. |
