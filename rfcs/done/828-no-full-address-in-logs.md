# RFC 828 — Remove Full Recipient Addresses from Logs

**Status.** Proposed  
**Tracks.** T1 — Security / Privacy  
**Touches.** `src/mail.rs`, `src/api/send.rs`, `src/api/send_bulk.rs`

## Problem

`mail::build_message()` may log the full email address on internal errors:

```rust
error!(error = %e, addr = %addr, "invalid to address after validation");
```

This conflicts with the `mask_recipient` policy, which exists precisely
to avoid logging full addresses.

The `smtp_submitted` log event already uses `recipient_domain` rather than
the full address. Internal error paths should follow the same discipline.

## Fix

Replace `addr = %addr` with `domain = %domain` or with `request_id` only:

```rust
// Before
error!(error = %e, addr = %addr, "internal: address failed after validation");

// After
let domain = addr.rfind('@').map(|i| &addr[i+1..]).unwrap_or("unknown");
error!(error = %e, request_id = %request_id, domain = %domain,
    "internal: address failed after validation");
```

Audit all log macro calls in mail.rs, send.rs, send_bulk.rs for full
address exposure.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-828-01 | No `tracing` call in src/ logs a full email address. |
| AC-828-02 | Error paths log `domain` (part after @) at most. |
| AC-828-03 | `grep -r 'addr = %' src/` finds no matches. |
