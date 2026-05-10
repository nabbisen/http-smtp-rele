# RFC 801 — RB-01: NUL Byte Removal from src/validation.rs

**Status.** Proposed  
**Tracks.** T3 — Validation / Correctness  
**Touches.** `src/validation.rs`

## Problem

`src/validation.rs` contains a literal NUL byte (`\x00`) inside the attachment
filename validation check:

```rust
if spec.filename.contains('/') || spec.filename.contains('\\') || spec.filename.contains('①') {
```

The `'①'` is actually an embedded NUL byte `'\0'`, not the displayed character.
This is a Rust compiler-level error waiting to happen and makes the file
uninterpretable as valid UTF-8 source.

## Fix

Replace with an explicit escape:

```rust
if spec.filename.contains('/')
    || spec.filename.contains('\\')
    || spec.filename.contains('\0')
{
    return Err(AppError::Validation(
        "attachments[].filename: path separators and NUL not allowed".into(),
    ));
}
```

After this fix, run the full test suite:

```sh
cargo fmt --check
cargo test --all-targets
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-801-01 | `src/validation.rs` contains no NUL bytes. |
| AC-801-02 | `cargo build --all-features` succeeds with zero warnings. |
| AC-801-03 | Test `attachment filename with NUL rejected` passes. |
