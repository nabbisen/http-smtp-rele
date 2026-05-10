# RFC 030 — HTTP API Surface and Versioning

**Status.** Implemented  
**Tracks.** API  
**Touches.** `src/api/routes.rs`, `src/app.rs`, `docs/api.md`

## Summary

Define the complete set of HTTP endpoints exposed by `http-smtp-rele`, their versioning strategy,
and the policy on backwards compatibility.

## Motivation

The API surface is the public contract of the relay. External systems integrate against specific
URLs and expect stable response shapes. Defining the surface explicitly — including what is
NOT exposed — prevents accidental endpoint proliferation and keeps the attack surface minimal
(FR-001, requirement §3.1).

## Scope

- All endpoints in MVP.
- URL versioning scheme.
- HTTP methods for each endpoint.
- `/send` alias policy.
- Stability guarantees.
- What is explicitly not exposed.

## Non-goals

- Request and response body schemas (RFC 031, 032).
- Authentication enforcement (RFC 040).
- Rate limiting (RFC 070).
- Management or admin API (not in MVP).

## Design

### Endpoint table

| Method | Path | Auth required | Purpose |
|--------|------|--------------|---------|
| `POST` | `/v1/send` | Yes | Submit a mail send request |
| `GET` | `/healthz` | No | Liveness check |
| `GET` | `/readyz` | No | Readiness check (SMTP reachable) |
| `GET` | `/version` | No | Version information (optional) |

### URL versioning

The path prefix `/v1` indicates the first stable API version. The version prefix is part of the
URL, not a header, for simplicity and proxy friendliness.

Version increment policy:
- `/v1` → `/v2` only on a breaking change to the request or response schema.
- Non-breaking additions (new optional request fields, new optional response fields) do not
  require a version bump.
- The version prefix only applies to the send API; health and readiness endpoints are versionless.

### `/send` alias

For backwards compatibility with pre-v1 testing, `/send` may be supported as an alias for
`/v1/send`. However:
- The alias is not advertised in documentation.
- The alias uses identical request/response handling.
- The alias may be removed in v0.2 if no downstream dependency exists.

Decision: **not implemented in MVP**. The alias is noted here for future reference but adds
no value for new integrations.

### `/healthz`

- No authentication required.
- Returns 200 if the process is running.
- Does not check SMTP reachability.
- Response body: `{"status":"ok"}`.
- Should be available before config is fully loaded (returns 503 during startup).

Use case: reverse proxy health checks, process monitors.

### `/readyz`

- No authentication required.
- Returns 200 if SMTP is reachable.
- Returns 503 if SMTP is unreachable.
- Response body: `{"status":"ok"}` or `{"status":"error","code":"smtp_unavailable"}`.
- **Must not be exposed externally** (reveals SMTP connectivity state to untrusted clients).

Use case: orchestration systems checking if the relay is ready to handle traffic.

### `/version`

Optional endpoint. Returns build metadata:

```json
{
  "name": "http-smtp-rele",
  "version": "0.1.0"
}
```

Disabled by default in production (can reveal version to scanners).

### Not exposed in MVP

| Item | Reason |
|------|--------|
| Management API | Attack surface; not needed for MVP |
| Web UI | Attack surface |
| OpenMetrics / Prometheus | Not in MVP |
| `/v1/keys` | Key management is config-file-based |
| `DELETE`, `PUT`, `PATCH` methods | Not needed |
| WebSocket | Not applicable |

## Implementation Plan

1. Define routes in `src/api/routes.rs`.
2. Register routes in `src/app.rs`.
3. Stub handlers in `src/api/handlers.rs`.
4. Write `/healthz` handler (returns `{"status":"ok"}`).
5. Write `/readyz` handler stub (full SMTP check in RFC 063).
6. Write `/version` handler (optional; toggle via `--enable-version` or config flag).
7. Document endpoints in `docs/api.md`.

## Test Plan

### Integration Tests

- `POST /v1/send` with valid payload reaches the send handler.
- `GET /healthz` returns 200 with `{"status":"ok"}`.
- `GET /readyz` returns 200 when SMTP is up, 503 when SMTP is down.
- `POST /healthz` returns 405 Method Not Allowed.
- `GET /v1/send` returns 405 Method Not Allowed.
- `GET /admin` returns 404.
- `GET /v2/send` returns 404.

### Security Tests

- `/readyz` is not routed through the auth middleware (authentication not required).
- Unknown paths return 404, not 200 (no default catch-all handler).

## Security Considerations

- `/readyz` must not be accessible from external networks (reverse proxy SHOULD block it).
  Document this requirement explicitly.
- `/version` is disabled by default or omitted to reduce fingerprinting.
- The 404 response for unknown paths must not reveal internal route structure.

## Operational Considerations

- Reverse proxy configuration for a typical deployment:
  - Forward `/v1/send` to the relay.
  - Forward `/healthz` to the relay (for load balancer checks).
  - Block `/readyz` and `/version` at the proxy layer.
- The `/healthz` endpoint is a liveness probe — it does not perform deep health checks to
  keep it fast and dependency-free.

## Documentation Changes

- Create `docs/api.md` documenting all endpoints.
- Include reverse proxy guidance in `docs/security.md`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-030-01 | `POST /v1/send` is the only mail submission endpoint. |
| AC-030-02 | `GET /healthz` returns 200 with `{"status":"ok"}`. |
| AC-030-03 | `GET /readyz` returns 200 when SMTP is reachable, 503 otherwise. |
| AC-030-04 | Wrong HTTP method returns 405. |
| AC-030-05 | Unknown paths return 404. |
| AC-030-06 | All endpoints are documented in `docs/api.md`. |

## Open Questions

- Whether to add a `GET /v1/send` endpoint for probing (returns 405 or 200 with API info).
  Decision: no — unnecessary surface area.
