# RFC 004 — Project Quality Gates

**Status.** Implemented  
**Tracks.** Governance  
**Touches.** `Makefile` or `scripts/`, `Cargo.toml`, `README.md`

## Summary

Define the set of commands that must pass on every changeset, covering formatting, linting,
tests, build, RFC integrity, and documentation checks. These gates are the shared definition
of "this is ready to ship."

## Motivation

Without explicit quality gates, different contributors apply different standards. In a
security-sensitive project, a missed `clippy` warning or an untested path can introduce a
vulnerability. Codifying the gates as a runnable command removes ambiguity and makes the bar
visible to everyone.

## Scope

- Define the mandatory quality gate commands.
- Define the security-specific gate items.
- Define the milestone-level gates (what must pass before each milestone is marked done).
- Specify how gates are run locally (a single `make check` or equivalent).
- Specify which gates must pass before an RFC moves to `done/`.

## Non-goals

- Setting up CI/CD infrastructure (pipelines are out of scope for MVP).
- Code coverage thresholds.
- Fuzzing (future consideration).

## Design

### Mandatory gate commands

Every changeset must pass the following:

```sh
# Formatting
cargo fmt --check

# Linting (zero warnings policy)
cargo clippy --all-targets --all-features -- -D warnings

# Tests
cargo test

# Release build (catches link errors)
cargo build --release

# RFC integrity
scripts/check-rfcs.sh
```

These five commands are the definition of "green." Nothing ships unless all five pass.

### Security gate

For every changeset that touches authentication, validation, sanitization, rate limiting,
or logging, additionally verify:

| Gate | Check |
|------|-------|
| No secret log | `grep -r 'api_key\|secret\|token' target/` is not present in compiled strings (manual review) |
| No body log | Body field does not appear in audit log output (integration test) |
| Fail closed | Auth and validation failures return 4xx, never 200 |
| Strict DTO | Unknown JSON fields cause 400, not silent ignore |
| No arbitrary header | Requests with `headers` field are rejected |
| No arbitrary From | Requests with `from` field are rejected |
| Localhost default | Default bind is `127.0.0.1`, not `0.0.0.0` |

### Milestone gate

Before a milestone is marked complete, in addition to the mandatory gate:

| Extra check | When |
|-------------|------|
| All RFC `proposed/` for this milestone moved to `done/` | On milestone completion |
| `CHANGELOG.md` updated | On milestone completion |
| Example config still parses | Every milestone |
| Security regression test suite passes | M5 and every subsequent milestone |
| OpenBSD checklist verified | M9 |
| RFC README updated | Any RFC state change |

### Release gate (M12)

Before v0.1.0:

| Gate | Check |
|------|-------|
| All MVP RFCs in `done/` | RFC index |
| API contract tests pass | Integration tests |
| Security regression tests pass | Test suite |
| OpenBSD checklist complete | Manual |
| `README.md` covers quick start | Documentation review |
| `CHANGELOG.md` has v0.1.0 entry | File check |
| `ROADMAP.md` has v0.2 section | File check |

### Local runner

Provide a `Makefile` or `scripts/check-all.sh` that runs all five mandatory gates in sequence
and reports a summary. A single command is better than five separate commands for developers
to remember.

```makefile
.PHONY: check
check:
	cargo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings
	cargo test
	cargo build --release
	scripts/check-rfcs.sh
	@echo "All gates passed."
```

## Implementation Plan

1. Add `Makefile` with a `check` target.
2. Verify all five commands pass on the initial skeleton.
3. Document the check command in `README.md` under "Development."
4. Define the security gate checklist in `docs/security.md`.

## Test Plan

### Operational Tests

- `make check` passes on a fresh checkout with a valid example config present.
- `make check` fails when `cargo fmt --check` would fail (introduce a formatting error and verify).
- `make check` fails when `cargo clippy` would fail (introduce a lint error and verify).
- `make check` fails when `scripts/check-rfcs.sh` would fail (introduce an RFC inconsistency and verify).

## Security Considerations

The quality gates exist in part to prevent security regressions. The clippy `-D warnings` flag
prevents common Rust pitfalls. The security gate checklist (secret logging, fail-closed behavior)
must not be abbreviated or skipped.

## Operational Considerations

- Developers must run `make check` before submitting changes.
- The gate commands must complete in a reasonable time to avoid being skipped.
- `cargo build --release` may be slow on first run; subsequent runs with an incremental cache
  are fast.

## Documentation Changes

- Document `make check` in `README.md` under a "Development" section.
- Reference the security gate in `docs/security.md`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-004-01 | `make check` (or `scripts/check-all.sh`) runs all five gate commands. |
| AC-004-02 | `make check` exits 0 on a correct tree. |
| AC-004-03 | `make check` exits non-zero if any individual gate fails. |
| AC-004-04 | Security gate checklist is documented in `docs/security.md`. |

## Open Questions

- Whether to add a `cargo audit` (dependency vulnerability scan) gate. Deferred to v0.2
  unless a vulnerability scanner is readily available in the build environment.
