# Testing

## Running Tests

```sh
# Requires Rust 1.91 and cargo 1.91 (see build environment notes below)
make test

# Or directly:
RUSTC=/usr/bin/rustc-1.91 RUSTDOC=/usr/bin/rustdoc-1.91 \
  /usr/bin/cargo-1.91 test
```

All tests should pass before any commit. The pre-commit gate:

```sh
make gate   # runs: check + test + check-rfcs
```

---

## Test Categories

### Unit Tests

Located in `#[cfg(test)]` modules in each source file. Cover:

- `auth`: token extraction, constant-time matching, disabled key rejection
- `config`: TOML parsing, validation errors
- `sanitize`: CR/LF detection
- `validation`: all validation rules (email format, limits, domain policy)
- `rate_limit`: token bucket refill, burst exhaustion, per-bucket independence
- `security`: no-op on non-OpenBSD

Run unit tests only:
```sh
make test
```

### Security Regression Tests (RFC 102)

Tests `SEC-001` through `SEC-017` cover every security control. Located in the security
test section of `src/tests.rs` (or `tests/security_tests.rs` when integration tests are
expanded in v0.2).

These tests must be retained permanently. Removing one requires an RFC proposing a
replacement or explaining why the control is no longer needed.

| ID | What it tests |
|----|---------------|
| SEC-001 | No auth header → 401 |
| SEC-002 | Wrong token → 403 |
| SEC-003 | Disabled key → 403 |
| SEC-004–007 | CR/LF injection in subject, from_name, reply_to, to → 400 |
| SEC-008–010 | Unknown fields (from, bcc, headers) → 400 |
| SEC-011 | Body size > limit → 413 |
| SEC-012 | Disallowed recipient domain → 400 |
| SEC-013 | Rate limit exceeded → 429 |
| SEC-014 | Forged X-Forwarded-For from untrusted peer ignored |
| SEC-015 | Auth failure log does not contain the token |
| SEC-016 | Submission log does not contain body |
| SEC-017 | `ApiKeyConfig` Debug does not expose secret |

### RFC Integrity Check

```sh
make check-rfcs
# or
sh scripts/check-rfcs.sh
```

Verifies: all RFC files named correctly, no duplicate numbers, all listed in README, all
links resolve, Status field matches folder.

---

## Build Environment

The project requires `rustc 1.91.1` and `cargo 1.91.1`. If your system has both installed
(as on the development machine):

```sh
# Explicit path invocation
RUSTC=/usr/bin/rustc-1.91 RUSTDOC=/usr/bin/rustdoc-1.91 /usr/bin/cargo-1.91 test

# Via make
make test
```

---

## Adding a Test

### Unit test

Add a `#[test]` function in the relevant module's `#[cfg(test)]` block.

### Security regression test

1. Add a `#[test]` function with the `SEC-NNN` label as a comment.
2. List it in the table in this document and in RFC 102.
3. The test must fail if the control it covers is removed.

Example:
```rust
/// SEC-018: X-API-Key header is accepted as an alternative to Authorization.
#[test]
fn sec_018_x_api_key_accepted() {
    // ...
}
```

---

## Manual Testing on OpenBSD

pledge/unveil behavior cannot be tested in CI (Linux). Manual verification steps:

1. Start the relay as `_http_smtp_rele`.
2. Verify SMTP submission works end-to-end.
3. Attempt to open a file not covered by unveil → process is killed (SIGABRT).
4. Check `/var/log/messages` for pledge violation records.
5. Run `fstat | grep http-smtp-rele` — only expected file descriptors should appear.
