# RFC 823 — Attack Surface Reduction: Feature Gates for Extended Capabilities

**Status.** Proposed  
**Tracks.** T1 — Security / T2 — Configuration  
**Touches.** `src/config.rs`, `src/validation.rs`, `src/api/send.rs`, `src/api/send_bulk.rs`

## Problem

v0.14.0 enables HTML body, attachments, CC, bulk send, and pipe mode by
default. Each expands the attack surface:

- **HTML body**: enables phishing/tracking pixel delivery through a trusted relay
- **Attachments**: increases per-request memory allocation; MIME complexity
- **Bulk send**: multiplies per-request SMTP submission load
- **Pipe mode**: adds process execution surface; weakens OpenBSD pledge

These features are useful but should require explicit operator opt-in.

## Decision

Add config flags to enable extended capabilities. All default to `false`.

```toml
[mail]
allow_html_body   = false   # default off; explicit opt-in required
allow_attachments = false   # default off

[smtp]
# mode = "smtp" (default) | "pipe" (explicit opt-in, requires allow_pipe_mode)
allow_pipe_mode   = false   # default off; pipe mode is refused unless true

[server]
allow_bulk_send   = false   # default off; POST /v1/send-bulk returns 404 unless true
```

When a disabled feature is used:
- `body_html` present with `allow_html_body = false` → 400 `feature_disabled`
- `attachments` present with `allow_attachments = false` → 400 `feature_disabled`
- `smtp.mode = "pipe"` with `allow_pipe_mode = false` → startup error
- `POST /v1/send-bulk` with `allow_bulk_send = false` → 404

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-823-01 | HTML body rejected (400) when `allow_html_body = false`. |
| AC-823-02 | Attachments rejected (400) when `allow_attachments = false`. |
| AC-823-03 | `smtp.mode = "pipe"` startup fails when `allow_pipe_mode = false`. |
| AC-823-04 | `POST /v1/send-bulk` returns 404 when `allow_bulk_send = false`. |
| AC-823-05 | All tests for extended features configure the relevant flag to `true`. |
| AC-823-06 | Default example config has all flags set to `false`. |
