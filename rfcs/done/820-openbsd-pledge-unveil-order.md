# RFC 820 — OpenBSD pledge/unveil Application Order

**Status.** Proposed  
**Tracks.** T1 — Security / OpenBSD  
**Touches.** `src/security.rs`, `crates/cli/src/main.rs`

## Problem

The current flow in `main.rs` is approximately:

```
apply_initial_restrictions(config_path)   ← unveil(config, "r") + unveil("","") + pledge
load config
apply_sqlite_restrictions(db_path)        ← too late: unveil already locked
apply_tls_file_restrictions(cert, key)    ← too late: unveil already locked
apply_runtime_restrictions(mode, ...)     ← pledge to final profile
```

`apply_initial_restrictions` calls `unveil("", "")` to lock the unveil table,
then the code attempts to add SQLite and TLS paths — which fails silently or
panics on OpenBSD because `unveil` after lock is an error.

Consequence: SQLite mode and TLS mode are **broken on OpenBSD** even though the
feature compiles.

## Decision

```
Phase 1: pre-config
  unveil(config_path, "r")
  pledge("stdio rpath")          ← minimum to read config; unveil NOT locked

Phase 2: post-config (after loading config, before runtime)
  unveil(config_path, "r")       ← re-register (idempotent)
  if sqlite: unveil(db_path, "rwc")
  if tls:    unveil(cert, "r"), unveil(key, "r")
  if pipe:   unveil(pipe_cmd, "x")
  unveil("", "")                 ← lock unveil HERE

Phase 3: runtime pledge
  pledge(final_profile)          ← stdio inet [rpath] [wpath cpath]
```

## API changes

Replace `apply_initial_restrictions` with phase-aware functions:

```rust
pub fn unveil_config(path: &Path) -> Result<(), String>;
pub fn unveil_runtime_paths(opts: &RuntimePaths) -> Result<(), String>;
pub fn lock_unveil() -> Result<(), String>;
pub fn apply_runtime_pledge(mode: RuntimeMode, has_sqlite: bool) -> Result<(), String>;
```

`RuntimePaths` holds optional db_path, cert, key, pipe_cmd.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-820-01 | SQLite mode starts successfully on OpenBSD. |
| AC-820-02 | TLS mode (--features tls) starts successfully on OpenBSD. |
| AC-820-03 | unveil is locked exactly once, after all paths are registered. |
| AC-820-04 | Non-OpenBSD builds compile and run unchanged. |
