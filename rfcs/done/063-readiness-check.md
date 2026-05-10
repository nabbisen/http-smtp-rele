# RFC 063 — Readiness Check Behavior

**Status.** Implemented  
**Tracks.** SMTP / API  
**Touches.** `src/api/handlers.rs`, `src/smtp.rs`

## Summary

Define the exact behavior of `GET /readyz`: when it returns 200 vs. 503, the SMTP probe
mechanism, and the policy for caching probe results.

## Motivation

`/readyz` is used by orchestration systems to decide whether to route traffic to this instance.
If the probe is too aggressive (connects on every request), it adds latency and load to the
SMTP server. If it is too lazy (cached too long), it routes traffic to an unready instance.
The right balance is a short-lived cached probe result (FR-081, NFR-AVL-001).

## Scope

- `GET /readyz`: 200 or 503.
- SMTP probe: TCP connect (from RFC 034 / RFC 061).
- Probe caching: TTL-based, short-lived.
- Timeout: same as `SmtpConfig.timeout_seconds`.

## Non-goals

- `/healthz` behavior (RFC 034).
- SMTP message submission (RFC 061).
- External access restriction (documented in RFC 034 and RFC 030).

## Design

### Probe caching

To avoid hitting the SMTP server on every readiness request, cache the probe result with a
short TTL (default: 5 seconds).

```rust
pub struct SmtpReadinessCache {
    last_ok: Mutex<Option<Instant>>,
    ttl: Duration,
}

impl SmtpReadinessCache {
    pub async fn is_ready(&self, smtp: &SmtpHandle) -> bool {
        let cached = {
            let guard = self.last_ok.lock().await;
            guard.map(|t| t.elapsed() < self.ttl).unwrap_or(false)
        };

        if cached {
            return true;
        }

        let ok = smtp.probe().await.is_ok();
        if ok {
            *self.last_ok.lock().await = Some(Instant::now());
        }
        ok
    }
}
```

The cache is stored in `AppState`. Cache invalidation is time-based only; there is no
manual invalidation.

### Handler

```rust
pub async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    if state.readiness_cache.is_ready(&state.smtp).await {
        (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({
            "status": "error",
            "code": "smtp_unavailable",
            "message": "SMTP is not reachable"
        })))
    }
}
```

### TTL configuration

Default probe TTL: 5 seconds. Not configurable in MVP (add to `[server]` in v0.2 if needed).

## Test Plan

### Integration Tests

- `/readyz` returns 200 when SMTP port is open.
- `/readyz` returns 503 when SMTP port is closed.
- Second `/readyz` within TTL does not trigger a new probe (requires instrumentation or clock
  injection in tests).
- After TTL expires, a new probe is triggered.

## Security Considerations

- The readiness endpoint leaks SMTP connectivity information. Document external access
  restriction requirement (already in RFC 030 and RFC 034).

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-063-01 | `/readyz` returns 200 when SMTP is reachable. |
| AC-063-02 | `/readyz` returns 503 when SMTP is not reachable. |
| AC-063-03 | Probe results are cached for the TTL duration. |

## Open Questions

None.
