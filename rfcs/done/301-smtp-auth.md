# RFC 301 — SMTP AUTH

**Status.** Implemented  
**Tracks.** SMTP / Config  
**Touches.** `src/config.rs`, `src/smtp.rs`, `docs/configuration.md`

## Summary

Add optional SMTP AUTH (username + password) to the SMTP transport, enabling relay to
non-localhost SMTP servers that require authentication.

## Design

```toml
[smtp]
mode = "smtp"
host = "smtp.example.com"
port = 587
auth_user     = "relay@example.com"
auth_password = "smtp-password"  # stored as SecretString
```

lettre credential injection:
```rust
.credentials(lettre::transport::smtp::authentication::Credentials::new(user, pass))
```

## Validation

- `auth_user` and `auth_password` must be either both set or both absent.
- If set with `mode = "pipe"`, startup fails (pipe mode has no credentials concept).

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-301-01 | SMTP session uses AUTH LOGIN/PLAIN when credentials are configured. |
| AC-301-02 | Setting only `auth_user` without `auth_password` (or vice versa) fails startup. |
| AC-301-03 | `auth_password` never appears in logs. |
