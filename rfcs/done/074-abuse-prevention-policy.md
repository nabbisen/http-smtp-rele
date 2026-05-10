# RFC 074 — Abuse Prevention Policy

**Status.** Implemented  
**Tracks.** Security  
**Touches.** `docs/security.md`

## Summary

Document the abuse prevention posture: what the rate limiter protects against, its known
limitations, and the operator response playbook for detected abuse.

## Motivation

Rate limiting is necessary but not sufficient. Operators need to know what the relay protects
against automatically, and what requires manual or external intervention, to configure their
deployment correctly (NFR-SEC-001, requirement §11).

## Scope

- What abuse scenarios are mitigated by rate limiting.
- Known limitations of the in-process rate limiter.
- Operator response playbook.
- Documentation in `docs/security.md`.

## Non-goals

- Automatic IP blocking (not in MVP).
- Integration with external WAF or threat intelligence (not in MVP).

## Design

### Protected scenarios

| Scenario | Mitigation |
|----------|-----------|
| Stolen key used for spam | Per-key rate limit + key disable |
| High-volume unauthenticated flood | IP rate limit + global rate limit |
| Single-source DoS | IP rate limit |
| Burst abuse | Burst capacity limits |

### Known limitations

| Limitation | Explanation |
|-----------|-------------|
| In-memory state | Rate limits reset on restart; burst possible post-restart |
| No distributed state | Multiple instances do not share rate limit state |
| No IP block | High-volume IPs are rate-limited but not blocked |
| No adaptive limits | Limits are static; no automatic tightening on abuse |

### Operator playbook

| Observation | Recommended action |
|------------|-------------------|
| Auth failures from one IP | Block IP at firewall or reverse proxy |
| Key causing repeated rate limits | Disable key (`enabled = false`) and restart |
| Global limit hit during normal traffic | Increase `global_per_minute` or investigate |
| Repeated `header_injection_attempt` events | Block IP at firewall |

### No automatic blocking

The relay does not block IPs or revoke keys automatically. These decisions involve business
context and should be made by operators or an external WAF.

## Documentation Changes

- Create or update `docs/security.md` with an "Abuse Prevention" section.
- Include the limitations table and operator playbook.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-074-01 | `docs/security.md` contains an "Abuse Prevention" section. |
| AC-074-02 | Known limitations are documented. |
| AC-074-03 | Operator playbook is documented. |

## Open Questions

None.
