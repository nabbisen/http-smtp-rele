# RFC 402 — SMTP STARTTLS and TLS

**Status.** Implemented  
**Tracks.** SMTP / Config

## Summary

Add `smtp.tls` configuration to support STARTTLS and implicit TLS for non-localhost relay.

## Design

```toml
[smtp]
tls = "none"      # default — plain TCP (localhost relay)
# tls = "starttls"  # opportunistic/required STARTTLS (port 587)
# tls = "tls"       # implicit TLS wrapper (port 465)
```

lettre builders:
- `"none"` → `builder_dangerous` (current)
- `"starttls"` → `starttls_relay(&host)?.port(port)...`
- `"tls"` → `relay(&host)?.port(port)...`

The `tokio1-rustls-tls` feature is already enabled in Cargo.toml.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-402-01 | `tls = "none"` builds a plain transport (unchanged). |
| AC-402-02 | `tls = "starttls"` uses lettre's STARTTLS builder. |
| AC-402-03 | `tls = "tls"` uses lettre's implicit TLS builder. |
| AC-402-04 | Invalid `tls` value fails at startup. |
