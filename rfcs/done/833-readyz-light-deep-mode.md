# RFC 833 — /readyz Light and Deep Readiness

**Status.** Proposed  
**Tracks.** T2 — HTTP API / Operations  
**Touches.** `src/api/health.rs`, docs

## Problem

`GET /readyz` performs a TCP connect to `[smtp].host:port`. This does not
verify SMTP EHLO, STARTTLS handshake, SMTP AUTH, or pipe-mode command
availability. Mismatches:

- STARTTLS configured but cert expired → TCP connects but SMTP fails → readyz reports OK
- SMTP AUTH configured but credentials wrong → TCP connects → readyz reports OK
- `smtp.mode = "pipe"` → TCP connect to wrong host → readyz always fails

## Decision

Define two explicit modes, opt-in via query parameter:

```
GET /readyz        → light check (TCP connect only)
GET /readyz?deep=1 → deep check (EHLO + STARTTLS + AUTH probe)
```

Light mode is safe for load-balancer health probes (fast, stateless).
Deep mode is for operator diagnostics and alerting, not continuous polling.

For pipe mode (`smtp.mode = "pipe"`): light readyz checks that the pipe
command exists and is executable. Deep mode executes the command with
`--version` or a no-op invocation if supported.

Document the limitations of each mode clearly. The current behaviour
(TCP only) is preserved as the default light mode.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-833-01 | `GET /readyz` (no param) performs TCP connect only — behaviour unchanged. |
| AC-833-02 | `GET /readyz?deep=1` performs EHLO (and STARTTLS/AUTH if configured). |
| AC-833-03 | Pipe mode readyz checks executable presence. |
| AC-833-04 | Both modes are documented in the operations guide. |
