# RFC 305 — SIGHUP Config Reload

**Status.** Implemented  
**Tracks.** Foundation / Platform  
**Touches.** `src/main.rs`, `src/lib.rs`

## Summary

On `SIGHUP`, reload the TOML config file, validate it, and hot-swap the shared
`AppConfig` without dropping in-flight requests.

## Design

`AppState.config` changes from `Arc<AppConfig>` to `Arc<ArcSwap<AppConfig>>` (using the
`arc-swap` crate). On SIGHUP:
1. Reload and validate config from the original path.
2. If validation fails, log the error and keep the current config.
3. If valid, `config.store(Arc::new(new_config))`.

The SMTP transport and rate limiter are NOT rebuilt on SIGHUP (they retain their state).
Only config values read per-request (auth keys, policy, limits) are refreshed.

```rust
tokio::spawn(async move {
    let mut sig = signal(SignalKind::hangup()).unwrap();
    loop {
        sig.recv().await;
        match config::load(&config_path) {
            Ok(new) => { shared_config.store(Arc::new(new)); tracing::info!("config reloaded"); }
            Err(e)  => { tracing::error!(error=%e, "SIGHUP reload failed; keeping current config"); }
        }
    }
});
```

Handlers read config via `state.config.load()` (returns an `Arc<AppConfig>`).

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-305-01 | `kill -HUP <pid>` reloads config without restarting the process. |
| AC-305-02 | Invalid config on SIGHUP keeps the current config and logs an error. |
| AC-305-03 | In-flight requests during reload are not interrupted. |
| AC-305-04 | New API keys from the reloaded config are honoured immediately. |
