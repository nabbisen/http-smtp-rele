# RFC 603 — Per-Key mask_recipient Override

**Status.** Proposed  
**Tracks.** Security / Config  
**Touches.** `src/config.rs`, `src/api/send.rs`

## Summary

Add `mask_recipient: Option<bool>` to `ApiKeyConfig` so individual keys can
override the global `[logging].mask_recipient` policy.

## Design

```toml
[[api_keys]]
id   = "privacy-sensitive-key"
mask_recipient = true   # always mask regardless of global setting

[[api_keys]]
id   = "debug-key"
mask_recipient = false  # never mask (for debugging only)
```

| `ApiKeyConfig.mask_recipient` | `LoggingConfig.mask_recipient` | Effective |
|-------------------------------|--------------------------------|-----------|
| `None` | any | inherit global |
| `Some(true)` | any | always mask |
| `Some(false)` | any | never mask |

The effective value is resolved in `send_mail` and passed to the log event.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-603-01 | `mask_recipient = true` masks `recipient_domain` in smtp_submitted log. |
| AC-603-02 | `mask_recipient = false` shows domain even when global is `true`. |
| AC-603-03 | `mask_recipient` absent inherits global setting. |
