# RFC 090 — OpenBSD Runtime Hardening

**Status.** Implemented  
**Tracks.** Platform  
**Touches.** `src/security.rs`, `src/main.rs`

## Summary

Define the overall OpenBSD security hardening strategy: the dedicated unprivileged user,
the sequence of `pledge` and `unveil` calls, and the no-op behavior on non-OpenBSD targets.

## Motivation

Running as root or with unrestricted syscalls on OpenBSD provides no defense against an
attacker who finds a vulnerability in the relay code. `pledge` and `unveil` limit the blast
radius of any exploit to the minimum set of operations actually needed (NFR-SEC-002,
NFR-SEC-003, NFR-SEC-004, AC-OBSD-001, AC-OBSD-002, AC-OBSD-003).

## Scope

- `security::apply(config: &AppConfig)` — the public function called from `main`.
- `pledge` promise set for SMTP relay mode.
- `unveil` paths and permissions.
- Call sequence: unveil before pledge.
- Non-OpenBSD: `apply` is a no-op; compiles cleanly on Linux/macOS.
- User: `_http_smtp_rele` (documentation; not enforced by the binary itself).

## Non-goals

- Sendmail pipe mode pledge (RFC 064 — deferred; pipe mode requires additional promises).
- Systemd sandboxing on Linux (future).

## Design

### `security::apply`

```rust
pub fn apply(config: &AppConfig) -> Result<(), SecurityError> {
    #[cfg(target_os = "openbsd")]
    {
        apply_openbsd(config)?;
    }
    #[cfg(not(target_os = "openbsd"))]
    {
        let _ = config;  // no-op on non-OpenBSD
    }
    Ok(())
}

#[cfg(target_os = "openbsd")]
fn apply_openbsd(config: &AppConfig) -> Result<(), SecurityError> {
    // 1. unveil: restrict filesystem access
    apply_unveil(config)?;
    // 2. pledge: restrict syscalls (must come after unveil)
    apply_pledge(config)?;
    Ok(())
}
```

### Sequence

The call to `security::apply` happens in `main`:

```
1. logging::init()
2. config::load()         ← reads config file (needs rpath)
3. security::apply()      ← locks down after config is loaded
4. build AppState         ← sets up SMTP transport (needs inet)
5. app::run()             ← serves requests (needs stdio inet)
```

`security::apply` runs after config is loaded so `rpath` is no longer needed. It runs before
the HTTP server starts accepting connections, so no untrusted input has been processed yet.

### Non-OpenBSD behavior

On Linux, macOS, and other platforms, `apply` compiles to an empty function. This ensures the
binary is portable and CI can run on any platform.

## Implementation Plan

1. Create `src/security.rs` with `apply`, `apply_openbsd`, `apply_unveil`, `apply_pledge`.
2. Use `libc::pledge` and `libc::unveil` via `unsafe` blocks, isolated to `security.rs`.
3. Call `security::apply(&config)` in `main` between config load and `AppState` construction.
4. Write a non-OpenBSD build test confirming the no-op compiles cleanly.

## Test Plan

### Operational Tests (manual, OpenBSD only)

- Binary starts as `_http_smtp_rele` user.
- `fstat` shows no unexpected open file descriptors after startup.
- Attempting to open a file not covered by `unveil` fails (OpenBSD kills the process).
- Attempting a syscall not covered by `pledge` fails (OpenBSD kills the process).

### Non-OpenBSD Tests (automated)

- `cargo build` succeeds on Linux.
- `cargo test` passes on Linux.
- `security::apply` returns `Ok(())` on Linux.

## Security Considerations

- `apply` must be called before any user request is handled. If it returns an error, the
  process must exit (fail-fast). A partially pledged process is dangerous.
- `unsafe` blocks for `libc::pledge` and `libc::unveil` are isolated to `security.rs`.
  No other module calls `unsafe` for these syscalls.
- The pledge promise set must be as minimal as possible. Adding promises "just in case"
  defeats the purpose.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-090-01 | `security::apply` is called between config load and server start. |
| AC-090-02 | On non-OpenBSD, `apply` compiles and returns `Ok` without doing anything. |
| AC-090-03 | On OpenBSD, `unveil` is called before `pledge`. |
| AC-090-04 | Failure in `security::apply` causes process exit. |

## Open Questions

None.
