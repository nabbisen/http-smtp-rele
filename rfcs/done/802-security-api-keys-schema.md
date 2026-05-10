# RFC 802 — RB-02: [[security.api_keys]] Config Schema Unification

**Status.** Proposed  
**Tracks.** T2 — Configuration  
**Touches.** `src/config.rs`, `examples/`, `docs/`, `rfcs/`

## Problem

The implementation nests `api_keys` inside `SecurityConfig`:

```rust
pub struct AppConfig {
    pub security: SecurityConfig,
}
pub struct SecurityConfig {
    pub api_keys: Vec<ApiKeyConfig>,
}
```

The correct TOML path is therefore `[[security.api_keys]]`.

However, README, example config, and docs all show `[[api_keys]]`, which
fails silently: the parsed `api_keys` is empty, and startup fails with
`NoApiKeys`.

## Decision

Unify all documentation and tests on `[[security.api_keys]]`.
This is the correct path and keeps API keys in the same security boundary
as `trust_proxy_headers`, `trusted_source_cidrs`, `allowed_source_cidrs`,
`mask_recipient`, and `allowed_recipients`.

## Scope

Replace `[[api_keys]]` with `[[security.api_keys]]` in:

- `README.md`
- `examples/http-smtp-rele.toml`
- `docs/configuration.md`
- `docs/getting-started.md`
- `docs/security.md`
- `docs/src/guides/configuration.md`
- `docs/src/getting-started.md`
- `docs/src/introduction.md`
- All RFC files containing `[[api_keys]]`
- All test fixtures that construct TOML strings

Add a config parse integration test that loads a TOML string with
`[[security.api_keys]]` and verifies the key is present.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-802-01 | `grep -r '\[\[api_keys\]\]'` finds no matches outside rfcs/done history. |
| AC-802-02 | `examples/http-smtp-rele.toml` parses correctly with the current code. |
| AC-802-03 | Config parse test with `[[security.api_keys]]` passes. |
| AC-802-04 | All existing tests pass. |
