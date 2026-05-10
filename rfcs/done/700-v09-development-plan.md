# RFC 700 — v0.9 Development Plan

**Status.** Proposed  
**Tracks.** Governance

## Theme: Bulk Submission

Single `POST /v1/send-bulk` that accepts an array of independent messages,
processes each through the same validation/SMTP pipeline, and returns
per-message results. Enables notification services to submit many
messages in one HTTP round-trip.

| RFC | Feature |
|-----|---------|
| 701 | `POST /v1/send-bulk` API |
| 702 | Bulk submission rate limiting strategy |
| 703 | Bulk submission integration tests |

## Scope boundaries

In: per-message validation, per-message status tracking, per-message
result in response, rate limit counted per message, `max_bulk_messages` config.

Out: message templating, fan-out (one body to many recipients is already
handled by the `to` array), parallel SMTP (v1.0 optimisation), Redis store.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-700-01 | `POST /v1/send-bulk` accepts array, returns per-message results. |
| AC-700-02 | Rate limits applied per message, not per request. |
| AC-700-03 | Each message gets its own `request_id` and status record. |
| AC-700-04 | `cargo test` — 0 failures in both default and sqlite builds. |
