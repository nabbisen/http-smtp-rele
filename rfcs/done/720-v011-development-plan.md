# RFC 720 — v0.11 Development Plan

**Status.** Proposed  
**Tracks.** Governance

## Theme: Hardening Correctness and Shared State

| RFC | Feature |
|-----|---------|
| 721 | OpenBSD SIGHUP `rpath` fix — keep `rpath` in runtime pledge |
| 722 | Redis/Valkey shared status store — optional feature |

## OpenBSD SIGHUP analysis

The current runtime `pledge("stdio inet")` drops `rpath`, breaking SIGHUP
config reload on OpenBSD. The fix is to keep `rpath` in the runtime pledge.

Security rationale: `unveil` already restricts readable paths to the config
file only. Keeping `rpath` in the pledge does not expand the effective read
attack surface beyond what `unveil` permits. This is the standard pattern
used by OpenBSD daemons that require config reload.

## Redis scope

The `StatusStore` trait abstraction (RFC 086) was designed to accommodate
Redis. Implementation is optional (`--features redis`), uses the `redis` crate
sync client, and follows degraded-mode behavior: Redis unavailability logs an
error but does not fail mail delivery.

`max_records` is not enforced in the Redis store; Redis memory policy
(`maxmemory-policy`) handles eviction. `expire_old_records()` is a no-op:
Redis handles TTL natively via EXPIRE.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-720-01 | SIGHUP reloads config on OpenBSD. |
| AC-720-02 | Redis store passes put/get/expire unit tests. |
| AC-720-03 | Redis unavailability does not fail mail delivery. |
| AC-720-04 | All 103+ existing tests continue to pass. |
