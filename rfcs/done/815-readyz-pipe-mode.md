# RFC 815 — M-03: /readyz Consistency with Pipe Mode

**Status.** Proposed  
**Tracks.** T2 — HTTP API  
**Touches.** `src/api/health.rs`, `src/config/validate.rs`

## Problem

`GET /readyz` always attempts a TCP connection to `[smtp].host:port`.
When `[smtp].mode = "pipe"`, the SMTP relay goes through a subprocess
(e.g., sendmail), and there is no TCP listener to probe.

This creates a false negative: readyz reports unhealthy for a correctly
configured pipe-mode deployment.

## Decision

Two acceptable resolutions; pick one:

**Option A (simpler):** Pipe mode is not supported in v0.15.
Add a `validate_config` check that rejects `smtp.mode = "pipe"`:
```
error: smtp.mode = "pipe" is not supported in this version
```

**Option B:** Pipe mode is supported. `/readyz` in pipe mode checks
that the `pipe_command` binary exists and is executable, not TCP.

Recommendation: Option A for now. Pipe mode has no integration test
coverage; formalising it properly is a larger effort.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-815-01 | The chosen option is implemented and tested. |
| AC-815-02 | `/readyz` always checks the actual configured delivery path. |
| AC-815-03 | Documentation reflects supported SMTP modes. |
