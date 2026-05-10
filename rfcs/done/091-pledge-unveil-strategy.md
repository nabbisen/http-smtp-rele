# RFC 091 — pledge and unveil Strategy

**Status.** Implemented  
**Tracks.** Platform  
**Touches.** `src/security.rs`

## Summary

Define the exact `pledge` promise string and `unveil` path table for SMTP relay mode,
justifying each entry.

## Motivation

Specifying pledge promises and unveil paths precisely, with justifications, makes future
audits easier and prevents promise creep (NFR-SEC-003, NFR-SEC-004, AC-OBSD-002, AC-OBSD-003).

## Scope

- Final `pledge` promise string for SMTP relay mode.
- Every `unveil` call: path, permissions, and reason.
- Locking sequence: `unveil(NULL, NULL)` before `pledge`.
- `SecurityError` definition.

## Non-goals

- Sendmail pipe mode promises (requires `exec proc`; deferred with RFC 064).

## Design

### `pledge` promise set — SMTP relay mode

```
"stdio inet"
```

| Promise | Justification |
|---------|---------------|
| `stdio` | Read/write to existing file descriptors (stderr for logs, stdin/stdout if needed) |
| `inet` | TCP connections to SMTP server and incoming HTTP connections |

Promises NOT included:

| Promise | Why excluded |
|---------|-------------|
| `rpath` | Config file already read before pledge |
| `wpath`/`cpath` | No file writes |
| `exec`/`proc` | No child processes in SMTP mode |
| `dns` | Using IP addresses directly; no hostname resolution needed |
| `tty` | No terminal interaction |

Note: If `smtp.host` is a hostname (not an IP), DNS resolution requires either resolving
before `pledge` or adding `dns` to promises. For MVP: document that `smtp.host` should be
an IP address (`127.0.0.1`) to avoid needing `dns`.

### `unveil` table — SMTP relay mode

| Path | Permissions | Reason |
|------|-------------|--------|
| (none) | — | No file access needed after config is loaded |

The config file is read during `config::load` (before `security::apply`). After pledge, the
config file is not re-read. Therefore no `unveil` calls are needed for SMTP relay mode.

The unveil finalization call (`unveil(NULL, NULL)`) restricts all filesystem access:

```rust
unsafe {
    let ret = libc::unveil(std::ptr::null(), std::ptr::null());
    if ret != 0 {
        return Err(SecurityError::UnveilFailed(errno()));
    }
}
```

### Implementation

```rust
#[cfg(target_os = "openbsd")]
fn apply_unveil(_config: &AppConfig) -> Result<(), SecurityError> {
    // For SMTP relay mode: no file access needed after config load.
    // Lock down the filesystem entirely.
    unsafe {
        let ret = libc::unveil(std::ptr::null(), std::ptr::null());
        if ret != 0 {
            return Err(SecurityError::UnveilFailed(std::io::Error::last_os_error()));
        }
    }
    Ok(())
}

#[cfg(target_os = "openbsd")]
fn apply_pledge(_config: &AppConfig) -> Result<(), SecurityError> {
    // SMTP relay mode: stdio + inet only.
    let promises = c"stdio inet";
    unsafe {
        let ret = libc::pledge(promises.as_ptr(), std::ptr::null());
        if ret != 0 {
            return Err(SecurityError::PledgeFailed(std::io::Error::last_os_error()));
        }
    }
    Ok(())
}
```

### `SecurityError`

```rust
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("pledge failed: {0}")]
    PledgeFailed(#[from] std::io::Error),
    #[error("unveil failed: {0}")]
    UnveilFailed(std::io::Error),
}
```

`SecurityError` causes `main` to exit with code 2 and a logged message.

## Implementation Plan

1. Add `libc = "0.2"` to `[target.'cfg(target_os = "openbsd")'.dependencies]`.
2. Implement `apply_unveil` and `apply_pledge` in `src/security.rs`.
3. Define `SecurityError`.
4. Write the note in `src/security.rs` about future pipe mode requirements.
5. Test on OpenBSD.

## Test Plan

### Operational Tests (manual, OpenBSD)

- After `pledge`, attempting to open a file → process killed with SIGABRT.
- After `unveil(NULL, NULL)`, `open("/etc/passwd", O_RDONLY)` → process killed.
- SMTP submission to `127.0.0.1:25` works normally after pledge.
- HTTP server continues to accept and respond after pledge.

## Security Considerations

- `c"stdio inet"` is a narrow promise set. Any future feature that needs additional syscalls
  must propose a RFC that explicitly expands the promise set with justification.
- The `unsafe` blocks are necessary for FFI to OpenBSD system calls. Safety comment must be
  present: "Safe because pledge/unveil operate on the process-level security context, not on
  Rust memory."

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-091-01 | Pledge promise string is `"stdio inet"` for SMTP relay mode. |
| AC-091-02 | `unveil(NULL, NULL)` is called before `pledge`. |
| AC-091-03 | `SecurityError` causes process exit with a logged message. |
| AC-091-04 | `libc` dependency is OpenBSD-only (`[target.cfg(...)].dependencies]`). |

## Open Questions

- Whether `dns` promise is needed when `smtp.host` is a hostname. Decision: document that
  `smtp.host` must be an IP address in OpenBSD deployments; validate in config (RFC 021).
