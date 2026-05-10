# RFC 014 — Graceful Startup and Shutdown

**Status.** Implemented  
**Tracks.** Foundation  
**Touches.** `src/main.rs`, `src/app.rs`

## Summary

Define the ordered startup sequence and signal-driven graceful shutdown for `http-smtp-rele`,
ensuring in-flight requests complete before the process exits and that startup errors abort
with a clear message.

## Motivation

A daemon that fails to start silently or that drops in-flight requests on termination is
operationally unsafe. OpenBSD's `rcctl restart` and `SIGTERM`-based supervisor termination
both rely on clean shutdown (NFR-AVL-002, NFR-AVL-003, NFR-OPS-003).

## Scope

- Startup sequence: logging → config → security → state → server.
- Fail-fast on any startup error with a clear log message and non-zero exit code.
- `SIGTERM` handler: stop accepting new connections, drain in-flight requests, then exit.
- `SIGINT` handler: same as SIGTERM (for local development).
- Shutdown timeout: configurable, with a hard cap.
- Exit code policy: 0 on clean shutdown; 1 on config error; 2 on runtime startup failure.

## Non-goals

- SMTP connection draining (the SMTP transport layer handles its own connection lifecycle).
- Persistent request queue (the application is stateless; if a request cannot complete within
  the shutdown window, it gets a 503, not queued).
- Systemd sd_notify or watchdog protocol.

## Design

### Startup sequence

```
1. logging::init(...)         // first; all subsequent errors are logged
2. cli::parse()               // parse --config path
3. config::load(path)         // load and validate TOML; exit(1) on error
4. security::apply(&config)   // pledge/unveil on OpenBSD; nop elsewhere
5. state::build(&config)      // construct AppState (SMTP transport, rate limiter)
6. app::run(state)            // bind, serve, block until shutdown signal
```

Each step can produce an error. Steps 1–5 run synchronously in the main thread. Step 6 starts
the Tokio runtime.

### Error handling at startup

All startup errors follow the same pattern:

```rust
match config::load(&path) {
    Ok(cfg) => cfg,
    Err(e) => {
        tracing::error!(error = %e, "failed to load config");
        std::process::exit(1);
    }
}
```

Exit codes:
- `0` — clean shutdown via signal.
- `1` — config error (wrong path, parse error, validation failure).
- `2` — runtime startup failure (bind address in use, SMTP unreachable at startup if checked).

### Graceful shutdown with Tokio

```rust
async fn run(state: AppState) {
    let listener = TcpListener::bind(&state.config.server.bind_address).await
        .expect("failed to bind");

    let shutdown = shutdown_signal();

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .expect("server error");

    tracing::info!("server stopped");
}

async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install CTRL+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received");
}
```

Axum's `with_graceful_shutdown` stops accepting new connections immediately and waits for
in-flight requests to complete. There is no explicit timeout in Axum's implementation; a
separate shutdown timeout task can be added if needed.

### Shutdown timeout

If any in-flight request does not complete within `shutdown_timeout_seconds` (default 30),
the process force-exits with code 0 after logging a warning. Implementation:

```rust
tokio::select! {
    _ = server.with_graceful_shutdown(shutdown_signal()) => {},
    _ = tokio::time::sleep(Duration::from_secs(config.server.shutdown_timeout_seconds)) => {
        tracing::warn!("shutdown timeout reached; force exiting");
    }
}
```

### rc.d integration

OpenBSD's `rc.subr` sends `SIGTERM` on `rcctl stop`. The process must exit within the
`daemon_timeout` (default 60 seconds in rc.subr). With a 30-second shutdown window, this is
well within the default.

## Implementation Plan

1. Implement `shutdown_signal()` in `src/app.rs`.
2. Wire `with_graceful_shutdown` into the Axum server call.
3. Implement the five-step startup sequence in `src/main.rs`.
4. Add `shutdown_timeout_seconds` to `[server]` config (RFC 024).
5. Write integration tests using `tokio::time::timeout`.

## Test Plan

### Unit Tests

- The startup sequence calls steps in order (testable via dependency injection).
- A config error at step 3 does not reach step 4.

### Integration Tests

- Server starts, handles one request, then receives SIGTERM and shuts down cleanly.
- In-flight request at shutdown time completes before the process exits.
- Process exits with code 1 on config error.
- Process exits with code 0 on clean SIGTERM.

### Operational Tests

- `rcctl stop http_smtp_rele` causes a clean shutdown on OpenBSD.
- `rcctl start http_smtp_rele` after a config error leaves the process stopped.

## Security Considerations

- `security::apply` runs before the Tokio runtime handles any request, so the OpenBSD
  `pledge`/`unveil` restrictions are applied before any untrusted input is processed.
- The startup sequence must not expose partial state if a later step fails. In particular,
  if `security::apply` fails, the process must exit before binding to any port.

## Operational Considerations

- `shutdown_timeout_seconds` defaults to 30. SMTP submission typically completes in under 5
  seconds; 30 seconds is generous.
- Startup failure messages go to stderr (already initialized at step 1) and are visible in
  `rcctl check` output on OpenBSD.
- The process does not daemonize itself — it is managed by `rc.d` / `rcctl`.

## Documentation Changes

- Document startup sequence and exit codes in `docs/openbsd.md`.
- Document `shutdown_timeout_seconds` in `docs/configuration.md`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-014-01 | Config error causes exit code 1 with a logged message before the server starts. |
| AC-014-02 | `SIGTERM` stops accepting new connections and drains in-flight requests. |
| AC-014-03 | `SIGINT` behaves identically to `SIGTERM`. |
| AC-014-04 | Shutdown completes within `shutdown_timeout_seconds`. |
| AC-014-05 | Clean shutdown exits with code 0. |
| AC-014-06 | `security::apply` runs before the server binds to any port. |

## Open Questions

None.
