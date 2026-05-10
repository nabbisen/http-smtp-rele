# RFC 204 — Per-Address Recipient Allowlist

**Status.** Implemented  
**Tracks.** Security / Config  
**Touches.** `src/config.rs`, `src/policy.rs`, `src/validation.rs`

## Summary

Add `allowed_recipients: Vec<String>` to `ApiKeyConfig` for keys that must be
restricted to specific full email addresses, not just domains.

## Design

```toml
[[api_keys]]
id    = "narrow-key"
secret = "..."
enabled = true
allowed_recipients = ["alice@example.com", "ops@example.com"]
```

Policy precedence:
1. If `allowed_recipients` is non-empty, the recipient must be in that list (exact match,
   case-insensitive local-part).
2. Then `allowed_recipient_domains` applies as before.
3. Then global `mail.allowed_recipient_domains`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-204-01 | Key with `allowed_recipients` only delivers to listed addresses. |
| AC-204-02 | An address not in the list returns 400. |
| AC-204-03 | `allowed_recipients = []` falls through to domain policy. |
