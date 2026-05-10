# RFC 116 — Testing Documentation

**Status.** Implemented  
**Tracks.** Release  
**Touches.** `docs/testing.md`

## Summary

Write `docs/testing.md` for contributors: how to run the test suite, how to run the security
regression tests, and how to add a new test.

## Content outline

```markdown
# Testing

## Running tests
cargo test
make check

## Test categories
- Unit tests (validation, auth, config)
- Integration tests (HTTP pipeline)
- Security regression tests (SEC-NNN)
- E2E tests (with SMTP stub)

## Security regression suite
How to add a new SEC-NNN test

## Testing on OpenBSD
Manual steps for pledge/unveil verification
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-116-01 | `docs/testing.md` documents `cargo test` and `make check`. |
| AC-116-02 | Security regression test IDs (SEC-NNN) are listed. |

## Open Questions

None.
