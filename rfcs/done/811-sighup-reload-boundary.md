# RFC 811 — H-05: SIGHUP Reload Boundary Specification

**Status.** Proposed  
**Tracks.** T2 — Configuration  
**Touches.** `src/lib.rs`, `src/config.rs`, `crates/cli/src/main.rs`, docs

## Problem

`AppState::reload_config()` updates `config_store` and `status_store` config,
but leaves `rate_limiter`, `smtp` transport, router body limit, timeout layer,
and concurrency limit unchanged. Handlers that call `state.config()` see new
values for `mail.*`, `security.*`, `status.*`, but these other components
keep their startup values.

This creates silent inconsistency: SIGHUP appears to work but only partially.

## Decision

Specify and enforce the reload boundary clearly.

### SIGHUP reloadable (current implementation already supports)

```toml
[mail].*
[logging].*
[status].ttl_seconds
[status].max_records
[status].cleanup_interval_seconds
```

### Restart required (document and enforce in validate_config on reload path)

```toml
[server].*
[smtp].*
[rate_limit].*
[security].*
[status].enabled
[status].store
[status].db_path
[status].redis_url
```

## Implementation

1. Add a `config::changes_require_restart(old: &AppConfig, new: &AppConfig) -> bool`
   function that detects restart-required changes.
2. On SIGHUP, if restart-required fields changed, log a WARNING:
   ```
   WARN event=sighup_restart_required fields="smtp.host, rate_limit.global_per_min"
   ```
   and apply only the reloadable fields.
3. Document the boundary in `docs/src/guides/configuration.md` and
   `docs/src/operations/deployment.md`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-811-01 | `changes_require_restart()` is implemented and tested. |
| AC-811-02 | Changing a restart-required field on SIGHUP logs a WARNING. |
| AC-811-03 | Reloadable fields take effect on SIGHUP without restart. |
| AC-811-04 | The boundary is documented in configuration reference. |
