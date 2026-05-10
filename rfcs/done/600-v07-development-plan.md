# RFC 600 — v0.7 Development Plan

**Status.** Proposed  
**Tracks.** Governance

## Theme: Observability and Operations

| RFC | Feature | Rationale |
|-----|---------|-----------|
| 601 | Status store Prometheus metrics | Implements RFC 089 spec; closes observability gap |
| 602 | `GET /v1/keys/self` — authenticated key info | Operational self-inspection without admin model |
| 603 | Per-key `mask_recipient` override | Privacy policy per API key |

## Deferred to v0.8

- SQLite persistent status store (RFC 088 implementation) — complex, pledge changes
- OpenBSD SIGHUP `rpath` window — platform-specific

## Implementation order

1. RFC 601 — Status store metrics (extends existing metrics.rs, minimal scope)
2. RFC 603 — Per-key mask_recipient (config.rs + logging in send_mail)
3. RFC 602 — Key info endpoint (new handler, auth scoped)

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-600-01 | `GET /metrics` includes `rele_status_store_*` metrics. |
| AC-600-02 | `GET /v1/keys/self` returns current key's non-secret config. |
| AC-600-03 | Per-key `mask_recipient` overrides global setting in logs. |
| AC-600-04 | `cargo test` passes 0 failures. |
