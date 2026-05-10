# RFC 034 — Health and Readiness Endpoints

**Status.** Implemented  
**Tracks.** API  
**Touches.** `src/api/handlers.rs`, `src/api/routes.rs`, `src/smtp.rs`

## Summary

Define the behavior of `GET /healthz` (liveness) and `GET /readyz` (readiness including SMTP
probe), their response shapes, and the policy that `/readyz` must not be externally accessible.

## Motivation

Reverse proxies, container orchestrators, and process supervisors rely on health endpoints to
route traffic and restart unhealthy processes. A liveness endpoint that is too deep
(checks SMTP) can cause a restart storm when SMTP is temporarily down. A readiness endpoint
that is too shallow (returns 200 always) causes traffic to reach an unready process
(FR-080, FR-081, NFR-AVL-001).

## Scope

- `/healthz`: always 200 if the process is running and config is loaded.
- `/readyz`: 200 if SMTP TCP connection can be established; 503 otherwise.
- Response JSON shapes.
- `/readyz` access restriction policy.
- SMTP probe implementation (lightweight TCP connect, no message submission).

## Non-goals

- Authentication on health endpoints.
- Deep health checks (disk space, memory usage).
- Circuit breaker for repeated SMTP failures.

## Design

### `/healthz`

```
GET /healthz → 200 OK
Content-Type: application/json

{"status": "ok"}
```

No dependencies checked. Returns 200 as long as the HTTP server is running and the handler
can execute. This is a pure liveness probe.

### `/readyz`

```
GET /readyz → 200 OK
Content-Type: application/json

{"status": "ok"}
```

or

```
GET /readyz → 503 Service Unavailable
Content-Type: application/json

{"status": "error", "code": "smtp_unavailable", "message": "SMTP is not reachable"}
```

SMTP probe: open a TCP connection to `config.smtp.host:config.smtp.port`. If the connection
succeeds within `config.smtp.timeout_seconds`, return 200. If it fails or times out, return 503.
The connection is closed immediately after success (no SMTP handshake required for the probe).

### Handler implementations

```rust
pub async fn healthz() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

pub async fn readyz(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    state.smtp.probe().await
        .map(|_| Json(serde_json::json!({"status": "ok"})))
        .map_err(|_| AppError::SmtpUnavailable)
}
```

### SMTP probe

```rust
impl SmtpHandle {
    pub async fn probe(&self) -> Result<(), ()> {
        let addr = format!("{}:{}", self.host, self.port);
        tokio::time::timeout(
            Duration::from_secs(self.timeout_seconds),
            TcpStream::connect(&addr),
        )
        .await
        .ok()
        .and_then(|r| r.ok())
        .map(|_| ())
        .ok_or(())
    }
}
```

### Access restriction policy

`/readyz` SHOULD NOT be accessible externally. The operator must configure the reverse proxy
to block external access to `/readyz`. This is documented in `docs/security.md` and
`docs/configuration.md`.

The application itself does not enforce this restriction (it cannot distinguish external from
internal requests at the HTTP level without additional config); it relies on the reverse proxy.

### No authentication

Neither endpoint requires authentication. Rationale:
- `/healthz` must be available even when auth is misconfigured.
- `/readyz` is for internal infrastructure use; the information it exposes (SMTP reachable
  or not) is not sensitive enough to require auth, but should not be publicly accessible.

## Implementation Plan

1. Implement `healthz` handler.
2. Implement SMTP probe in `src/smtp.rs`.
3. Implement `readyz` handler using the probe.
4. Register routes in `routes.rs`.
5. Document access restriction in `docs/security.md`.
6. Write tests.

## Test Plan

### Integration Tests

- `GET /healthz` returns 200 with `{"status":"ok"}`.
- `GET /readyz` returns 200 when SMTP is reachable (use a test TCP listener).
- `GET /readyz` returns 503 when SMTP is not reachable.
- `GET /healthz` returns 200 even when SMTP is down.
- `POST /healthz` returns 405.

## Security Considerations

- `/readyz` exposes whether the SMTP server is reachable. This could aid reconnaissance.
  Document the requirement to restrict external access at the proxy layer.
- No sensitive information (keys, config details) is returned by either endpoint.

## Operational Considerations

- Reverse proxy health check: use `/healthz` (no SMTP dependency).
- Orchestrator readiness probe: use `/readyz`.
- If the SMTP server is temporarily down, `/readyz` returns 503 but the process remains up.
  This prevents new traffic from reaching the relay (correct behavior).

## Documentation Changes

- Document both endpoints in `docs/api.md`.
- Document `/readyz` access restriction in `docs/security.md`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-034-01 | `GET /healthz` returns 200 regardless of SMTP state. |
| AC-034-02 | `GET /readyz` returns 200 when SMTP TCP connection succeeds. |
| AC-034-03 | `GET /readyz` returns 503 when SMTP is unreachable. |
| AC-034-04 | Neither endpoint requires authentication. |
| AC-034-05 | `/readyz` access restriction is documented with reverse proxy guidance. |

## Open Questions

None.
