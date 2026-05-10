# RFC 822 — /metrics Access Control

**Status.** Proposed  
**Tracks.** T1 — Security / T2 — HTTP API  
**Touches.** `src/api/mod.rs`, `src/api/metrics_handler.rs`, `src/config.rs`

## Problem

`GET /metrics` is registered without authentication and without any
source-IP restriction. It is publicly accessible on all interfaces.

Metrics reveal operational data (auth failure counts, rate limit hits,
SMTP error rates, status store size) that can assist attackers in
calibrating abuse.

Proxy-layer restriction is documented but there is no defense-in-depth
at the application layer.

## Decision

Add an application-layer source-IP restriction for monitoring endpoints.

```toml
[server]
# CIDRs allowed to access /metrics, /healthz, /readyz without further auth.
# Default: loopback only. Set to [] to allow all (proxy-restricted deployments).
monitoring_cidrs = ["127.0.0.1/32", "::1/128"]
```

Behaviour:
- Requests to `/metrics`, `/readyz` from outside `monitoring_cidrs` → 403
- `/healthz` remains open (needed by load balancers)
- Default restricts to loopback only, matching most deployment patterns

## Alternative considered

Bearer token on /metrics — rejected as too much friction for Prometheus scrape
configuration. CIDR restriction is simpler and matches industry practice.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-822-01 | `/metrics` returns 403 when caller is outside `monitoring_cidrs`. |
| AC-822-02 | `/metrics` returns 200 when caller is in `monitoring_cidrs`. |
| AC-822-03 | `/healthz` is not affected by `monitoring_cidrs`. |
| AC-822-04 | Default `monitoring_cidrs` restricts to loopback. |
| AC-822-05 | Integration test covers both allowed and blocked cases. |
