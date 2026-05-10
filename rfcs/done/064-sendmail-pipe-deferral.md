# RFC 064 — Sendmail Pipe Mode Deferral

**Status.** Implemented  
**Tracks.** SMTP  
**Touches.** `src/config.rs`, `src/smtp.rs`, `docs/configuration.md`

## Summary

Document the decision to defer sendmail pipe mode (`mode = "pipe"`) to v0.2, explain the
rationale, and specify what placeholder behavior the MVP provides.

## Motivation

Pipe mode requires spawning a child process, which complicates OpenBSD `pledge` (requires
`exec proc` in addition to `stdio inet`). It also requires `unveil("/usr/sbin/sendmail", "x")`.
Deferring pipe mode allows the MVP to use the simplest and most secure `pledge` promise set
(`stdio inet`), while still documenting the path for future implementation (FR-061, FR-062).

## Scope

- Config parsing for `mode = "pipe"` (accepted; stored).
- Startup behavior when `mode = "pipe"` is set: log a warning and fall back to SMTP mode, OR
  emit an error and refuse to start. Decision: refuse to start (fail-fast, RFC 021).
- Documentation of the deferral decision.
- Placeholder in `smtp.rs` for future pipe implementation.

## Non-goals

- Actual sendmail pipe implementation (deferred to v0.2).
- SMTP AUTH (also deferred).

## Design

### Config validation behavior

When `mode = "pipe"` is set in the config, `config::validate` returns:

```rust
ConfigError::Validation(
    "smtp.mode = \"pipe\" is not yet supported; use mode = \"smtp\""
)
```

The process refuses to start. This is stricter than a warning, because starting in SMTP mode
when the operator intended pipe mode could cause mail to go to the wrong server.

### `SmtpMode::Pipe` in code

The enum variant exists and is parsed, but `SmtpHandle::from_config` returns an error for it:

```rust
SmtpMode::Pipe => {
    return Err(SmtpBuildError::PipeModeNotSupported);
}
```

This provides a clear error if the config validation is somehow bypassed.

### Future pipe mode pledge requirements

When pipe mode is implemented, the pledge call will change from:

```
pledge("stdio inet", None)
```

to:

```
pledge("stdio inet proc exec", None)
```

This is documented in a comment in `security.rs` as a forward reference:

```rust
// NOTE: If sendmail pipe mode (RFC 064) is implemented in a future version,
// the pledge promise set must include "proc exec" and unveil must add:
//   unveil("/usr/sbin/sendmail", "x")
```

## Documentation Changes

- Document `mode = "smtp"` as the only supported mode in `docs/configuration.md`.
- Note pipe mode as a planned v0.2 feature in `ROADMAP.md`.
- Add a comment in `SmtpConfig` that `pipe` is parsed but not yet supported.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-064-01 | Config with `mode = "pipe"` causes startup failure with a clear error. |
| AC-064-02 | `mode = "smtp"` is documented as the only supported mode in MVP. |
| AC-064-03 | Pipe mode is listed in `ROADMAP.md` as a v0.2 item. |

## Open Questions

None.
