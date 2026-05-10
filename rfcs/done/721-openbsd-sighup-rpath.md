# RFC 721 — OpenBSD SIGHUP rpath Fix

**Status.** Proposed  
**Tracks.** T1 — Security / OpenBSD  
**Touches.** `src/security.rs`, `crates/cli/src/main.rs`

## Problem

After `apply_runtime_restrictions`, the pledge set is `stdio inet` (or
`stdio exec proc` for pipe mode). This drops `rpath`, so the SIGHUP handler
cannot `open()` the config file on OpenBSD.

## Fix

Keep `rpath` in the runtime pledge. The `unveil` restriction already limits
accessible paths to the config file, SQLite database, and TLS certs as
unveiled during `apply_initial_restrictions`. `rpath` alone does not expand
the effective read surface.

## New pledge sets

| Mode | Store | TLS | Runtime pledge |
|------|-------|-----|----------------|
| smtp | memory | no | `stdio inet rpath` |
| smtp | memory | yes | `stdio inet rpath` |
| smtp | sqlite | no | `stdio inet rpath wpath cpath` |
| smtp | sqlite | yes | `stdio inet rpath wpath cpath` |
| pipe | memory | no | `stdio exec proc rpath` |
| pipe | sqlite | no | `stdio exec proc rpath wpath cpath` |

`wpath cpath` remain conditional on SQLite. `rpath` is now always included.

## API change

`apply_runtime_restrictions(mode, has_sqlite)` → unchanged signature.
The function body adds `rpath` unconditionally.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-721-01 | SIGHUP handler can read config file after runtime pledge on OpenBSD. |
| AC-721-02 | Non-OpenBSD platforms: no-op unchanged. |
| AC-721-03 | Pledge sets correctly exclude `wpath cpath` for memory store. |
