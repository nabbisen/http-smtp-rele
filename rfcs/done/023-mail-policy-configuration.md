# RFC 023 — Mail Policy Configuration

**Status.** Implemented  
**Tracks.** Foundation / Security  
**Touches.** `src/config.rs`, `src/policy.rs`, `docs/configuration.md`

## Summary

Define the mail policy layer: how `MailConfig` fields translate into runtime policy decisions
about allowed senders, recipient domains, and per-key overrides, and how the `policy` module
enforces them.

## Motivation

The relay must not be an open relay. The two primary open-relay risks are unconstrained
recipients (anyone can send to anyone) and unconstrained senders (anyone can forge any From).
The policy layer is the business-logic enforcement point for both, sitting between validation
(syntactic checks) and SMTP submission (FR-046, FR-052, FR-063, NFR-SEC-001).

## Scope

- `MailConfig` fields and their policy implications.
- `policy::check_recipient` — domain allowlist enforcement.
- `policy::check_from` — From is always the config default; client-supplied From is always rejected.
- Per-key `allowed_recipient_domains` override.
- Per-key `allowed_recipients` override (specific addresses).
- Fail-closed behavior: unknown domains and empty allowlists.

## Non-goals

- Email address syntax validation (RFC 050).
- Header injection prevention (RFC 051).
- Rate limiting (RFC 070).
- SMTP submission (RFC 061).

## Design

### Policy model

Two policy checks run on every validated request:

1. **Recipient domain check** — the `to` address domain must appear in the effective allowlist.
2. **From check** — the `from` is always the `default_from` from config; any client-supplied
   `from` is rejected at the validation layer (RFC 050), never reaching policy.

### Recipient domain allowlist

The effective recipient domain allowlist for a request is determined as follows:

```
if api_key.allowed_recipient_domains is non-empty:
    effective = api_key.allowed_recipient_domains
else if api_key.allowed_recipients is non-empty:
    effective = {domain part of each address in api_key.allowed_recipients}
else if config.mail.allowed_recipient_domains is non-empty:
    effective = config.mail.allowed_recipient_domains
else:
    effective = UNRESTRICTED  (emit warn at startup; allow any domain)
```

```rust
pub enum RecipientPolicy {
    /// Any recipient domain is allowed.
    /// This configuration emits a startup warning.
    Unrestricted,
    /// Only these domains are allowed.
    Domains(Vec<String>),
    /// Only these specific addresses are allowed.
    Addresses(Vec<String>),
}
```

```rust
/// Returns Ok if the recipient is permitted, Err with the rejection reason otherwise.
pub fn check_recipient(
    to: &str,
    policy: &RecipientPolicy,
) -> Result<(), PolicyError> {
    match policy {
        RecipientPolicy::Unrestricted => Ok(()),
        RecipientPolicy::Domains(allowed) => {
            let domain = domain_of(to)?;
            if allowed.iter().any(|d| d.eq_ignore_ascii_case(&domain)) {
                Ok(())
            } else {
                Err(PolicyError::RecipientDomainNotAllowed(domain))
            }
        }
        RecipientPolicy::Addresses(allowed) => {
            if allowed.iter().any(|a| a.eq_ignore_ascii_case(to)) {
                Ok(())
            } else {
                Err(PolicyError::RecipientNotAllowed)
            }
        }
    }
}
```

### From policy

No client-supplied `from` is accepted. This check lives in validation (RFC 050). The policy
module provides `mail_from_address(&config) -> &str` which always returns `config.mail.default_from`.

### `PolicyError`

```rust
#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("recipient domain not allowed")]
    RecipientDomainNotAllowed(String),

    #[error("recipient address not allowed")]
    RecipientNotAllowed,

    #[error("could not parse recipient domain")]
    InvalidRecipient,
}
```

`PolicyError` maps to `AppError::Validation`.

### Per-key policy resolution

`AuthContext` (set by auth middleware) carries the active `ApiKeyConfig`. The `policy` module
receives `&AuthContext` and `&MailConfig` and computes the effective `RecipientPolicy` at
request time.

## Implementation Plan

1. Define `RecipientPolicy` and `PolicyError` in `src/policy.rs`.
2. Implement `check_recipient`.
3. Implement `effective_recipient_policy(key: &ApiKeyConfig, mail: &MailConfig) -> RecipientPolicy`.
4. Implement `mail_from_address`.
5. Wire into the send handler pipeline (RFC 030).
6. Write tests.

## Test Plan

### Unit Tests

- Recipient in allowed domain → `Ok`.
- Recipient in disallowed domain → `Err(RecipientDomainNotAllowed)`.
- Per-key domain override takes precedence over global list.
- Per-key address list allows exact match, rejects near-miss.
- `RecipientPolicy::Unrestricted` → always `Ok`.
- Empty global + empty per-key → `Unrestricted`.

### Security Tests

- A key with `allowed_recipient_domains = ["example.com"]` cannot send to `evil.com`.
- A key with `allowed_recipients = ["alice@example.com"]` cannot send to `bob@example.com`.
- A request with a client-supplied `from` field is rejected by validation before reaching policy.

## Security Considerations

- `RecipientPolicy::Unrestricted` is the fail-open case and must emit a startup `warn!`.
- Domain comparisons are case-insensitive to prevent bypasses like `EVIL.COM` vs `evil.com`.
- The policy check must happen before SMTP submission. A policy failure must never result in
  a message being submitted to SMTP.

## Operational Considerations

- Operators who want to allow all domains must leave `allowed_recipient_domains = []` globally
  and set no per-key overrides. The startup warning reminds them of the risk.
- Per-key `allowed_recipients` (exact addresses) enables fine-grained control for keys used
  in specific integration scenarios.

## Documentation Changes

- Document recipient policy in `docs/configuration.md`.
- Document the open-relay prevention model in `docs/security.md`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-023-01 | Recipient in a permitted domain passes policy. |
| AC-023-02 | Recipient in a non-permitted domain is rejected with `PolicyError`. |
| AC-023-03 | Per-key domain override takes effect when set. |
| AC-023-04 | Empty global and per-key allowlist produces `Unrestricted` policy. |
| AC-023-05 | Domain comparison is case-insensitive. |

## Open Questions

- Whether to support wildcard subdomains (`*.example.com` matching `foo.example.com`).
  Deferred to v0.2; MVP uses exact domain matching only.
