# RFC 812 — H-06: Documentation — cargo build for CLI Binary

**Status.** Proposed  
**Tracks.** T6 — Documentation  
**Touches.** `README.md`, `docs/src/getting-started.md`, `docs/src/operations/deployment.md`

## Problem

README and docs show:

```sh
cargo build --release
```

The root package is a library crate. Running `cargo build --release`
without a package selector builds only the library. The `http-smtp-rele`
binary is in `crates/cli/`.

Users following the quick start will not produce a runnable binary.

## Fix

Replace all `cargo build --release` invocations with:

```sh
# Build the CLI binary
cargo build --release --workspace
# Binary location:
# target/release/http-smtp-rele
```

Or the explicit form:

```sh
cargo build --release -p http-smtp-rele-cli
```

Also add the binary path explicitly where install steps are shown:

```sh
install -m 755 target/release/http-smtp-rele /usr/local/bin/http-smtp-rele
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-812-01 | No `cargo build --release` without `--workspace` or `-p` in user-facing docs. |
| AC-812-02 | Binary output path `target/release/http-smtp-rele` is stated. |
