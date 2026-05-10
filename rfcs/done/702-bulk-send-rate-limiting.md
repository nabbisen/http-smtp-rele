# RFC 702 — Bulk Submission Rate Limiting

**Status.** Proposed  
**Tracks.** T3 — Rate Limiting  
**Touches.** `src/api/send_bulk.rs`

## Design

Rate limits are checked per message, not per bulk request. This prevents
`POST /v1/send-bulk` from being a mechanism to bypass rate controls.

## Check order

For each message in the array (in index order):

1. Global rate limit (`check_global`)
2. IP rate limit (`check_ip`)
3. Per-key rate limit (`check_key`)

If any check fails, that message is marked `rejected` with `code = rate_limited`.
Processing of subsequent messages continues.

## Rate limit exhaustion

If the global rate limit is exhausted mid-array:
- Messages processed before exhaustion: their outcomes are preserved.
- Remaining messages: marked `rejected / rate_limited`.

This provides the most useful behaviour for notification services: as many
messages as possible are delivered before limits are hit.

## `Retry-After` header

When the global rate limit is exhausted, `Retry-After` is included in the
202 response header (set to the earliest retry window across all rejected messages).

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-702-01 | Each message in the array decrements the rate limit bucket. |
| AC-702-02 | Global exhaustion mid-array: earlier messages accepted, remainder rejected. |
| AC-702-03 | Rate-limited messages in results: `status = "rejected"`, `code = "rate_limited"`. |
