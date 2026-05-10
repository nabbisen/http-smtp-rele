# RFC 813 — M-01: StatusStore put() Called Twice — Metrics Double-Counted

**Status.** Proposed  
**Tracks.** T5 — Status Tracking  
**Touches.** `src/api/send.rs`, `src/api/send_bulk.rs`, `src/status_memory.rs`

## Problem

`send_mail` and `send_bulk` call `status_store.put()` twice for successful
submissions:

1. First `put()`: placeholder record with empty `recipient_domains` and
   `recipient_count = 0`.
2. Second `put()`: complete record with real metadata, `Utc::now()` as
   `created_at`.

Consequences:
- `status_record_created` metric incremented twice per request.
- `created_at` reflects validation time, not auth time.
- `status_records_current` gauge may be inflated.

## Fix

**Option A (preferred):** Remove the first `put()`. After validation, do a
single `put()` with the complete record. Use a single `update_status()` for
all subsequent transitions.

**Option B:** Add a separate `StatusStore::update_metadata()` method for
post-validation metadata updates that does not increment creation metrics.

Option A is simpler.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-813-01 | `status_record_created` incremented exactly once per submission. |
| AC-813-02 | `created_at` is set at auth-success time, not validation time. |
| AC-813-03 | `status_records_current` gauge reflects actual record count. |
