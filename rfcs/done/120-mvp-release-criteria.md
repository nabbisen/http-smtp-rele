# RFC 120 — MVP Release Criteria

**Status.** Implemented  
**Tracks.** Release  
**Touches.** All

## Summary

Define the complete set of conditions that must be true before `v0.1.0` is tagged, to ensure
the MVP is secure, tested, documented, and operationally deployable.

## Motivation

Without an explicit release gate, "done" is ambiguous. This RFC makes the MVP completion
condition unambiguous and auditable by listing every required condition with a reference to
the RFC that defines it (requirement §13.1, AC-001 through AC-OBSD-005).

## Scope

All conditions are binary (pass/fail). A release is not made until every condition passes.

## Design

### Functional criteria

| ID | Condition | RFC |
|----|-----------|-----|
| MVP-F-01 | `POST /v1/send` accepts valid JSON and submits to SMTP | 031, 060, 061 |
| MVP-F-02 | SMTP submit returns 202 Accepted | 031 |
| MVP-F-03 | SMTP failure returns 502 | 062 |
| MVP-F-04 | `GET /healthz` returns 200 | 034 |
| MVP-F-05 | `GET /readyz` returns 200/503 based on SMTP reachability | 034, 063 |
| MVP-F-06 | `--config` CLI flag accepted | 021 |
| MVP-F-07 | Startup fails with clear error on invalid config | 021 |

### Security criteria

| ID | Condition | RFC |
|----|-----------|-----|
| MVP-S-01 | Missing API key returns 401 | 040, 044 |
| MVP-S-02 | Invalid API key returns 403 | 040, 044 |
| MVP-S-03 | Disabled API key returns 403 | 040, 044 |
| MVP-S-04 | CR/LF in subject returns 400 | 051 |
| MVP-S-05 | CR/LF in from_name returns 400 | 051 |
| MVP-S-06 | `from` field in request returns 400 | 031 |
| MVP-S-07 | `headers` field in request returns 400 | 031 |
| MVP-S-08 | Oversized body returns 413 | 024, 053 |
| MVP-S-09 | Rate limit exceeded returns 429 | 070, 071, 073 |
| MVP-S-10 | API key never appears in logs | 082 |
| MVP-S-11 | Request body never appears in logs | 082 |
| MVP-S-12 | Default bind is 127.0.0.1 | 024 |
| MVP-S-13 | Disallowed recipient domain returns 400 | 023, 052 |

### All security regression tests pass

All SEC-001 through SEC-017 tests (RFC 102) pass on the release commit.

### OpenBSD criteria

| ID | Condition | RFC |
|----|-----------|-----|
| MVP-O-01 | Runs as `_http_smtp_rele` non-root user | 090, 092 |
| MVP-O-02 | `pledge("stdio inet")` applied in SMTP relay mode | 091 |
| MVP-O-03 | `unveil(NULL, NULL)` applied after config load | 091 |
| MVP-O-04 | `rcctl start/stop/restart` works | 092 |
| MVP-O-05 | OpenSMTPD localhost relay integration works | 093 |

### Quality gate criteria

| ID | Condition | RFC |
|----|-----------|-----|
| MVP-Q-01 | `cargo fmt --check` passes | 004 |
| MVP-Q-02 | `cargo clippy --all-targets -- -D warnings` passes | 004 |
| MVP-Q-03 | `cargo test` passes | 004 |
| MVP-Q-04 | `cargo build --release` passes | 004 |
| MVP-Q-05 | `scripts/check-rfcs.sh` passes | 003 |
| MVP-Q-06 | All MVP RFCs are in `rfcs/done/` | 001 |

### Documentation criteria

| ID | Condition | RFC |
|----|-----------|-----|
| MVP-D-01 | `README.md` follows defined structure | 110, 111 |
| MVP-D-02 | `docs/api.md` documents all endpoints and error codes | 112 |
| MVP-D-03 | `docs/configuration.md` documents all TOML fields | 113 |
| MVP-D-04 | `docs/security.md` covers open relay prevention | 114 |
| MVP-D-05 | `docs/openbsd.md` covers full deployment guide | 115 |
| MVP-D-06 | `examples/http-smtp-rele.toml` exists and parses | 020 |
| MVP-D-07 | `examples/curl-send.sh` works with the example config | 111 |
| MVP-D-08 | `CHANGELOG.md` has `[0.1.0]` entry | 117 |
| MVP-D-09 | `ROADMAP.md` has `[v0.2]` section | 117 |

### Release process

1. Verify all conditions above.
2. Move all MVP RFCs to `rfcs/done/`.
3. Update `CHANGELOG.md`.
4. Commit as "chore: release v0.1.0".
5. Tag `v0.1.0`.
6. Build release binary: `cargo build --release`.
7. Create archive: `http-smtp-rele-v0.1.0.tar.gz`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-120-01 | Every MVP-F, MVP-S, MVP-O, MVP-Q, MVP-D condition is met. |
| AC-120-02 | The release archive is named `http-smtp-rele-v0.1.0.tar.gz`. |
| AC-120-03 | `rfcs/README.md` shows all MVP RFCs as Implemented. |

## Open Questions

None.
