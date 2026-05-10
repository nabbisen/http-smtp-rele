# RFC 501 — Cargo Workspace Split

**Status.** Proposed  
**Tracks.** Architecture

## Summary

Split the single-crate project into a Cargo workspace:
- Root package: `http-smtp-rele` — library crate (all modules)
- `crates/cli/`: `http-smtp-rele-cli` — thin binary crate

## Design

The root package becomes a **pure library** (removes `[[bin]]`).  
`crates/cli/` adds only `src/main.rs` and depends on the root library.

```toml
# Root Cargo.toml (add at top)
[workspace]
members = [".", "crates/cli"]
resolver = "2"

# Remove [[bin]] section from root
```

```toml
# crates/cli/Cargo.toml
[package]
name    = "http-smtp-rele-cli"
version = "0.5.0"

[[bin]]
name = "http-smtp-rele"
path = "src/main.rs"

[dependencies]
http-smtp-rele = { path = "../.." }
tokio = { version = "1", features = ["full"] }
clap  = { version = "4", features = ["derive"] }
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt", "json"] }
```

`main.rs` imports `use http_smtp_rele::...` — unchanged.  
Integration tests under `tests/` stay in the root package — unchanged.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-501-01 | `cargo build --workspace` succeeds. |
| AC-501-02 | `cargo test --workspace` passes all existing tests. |
| AC-501-03 | `crates/cli/target` is NOT used (workspace shares `target/`). |
