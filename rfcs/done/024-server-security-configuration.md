# RFC 024 — Server and Security Configuration

**Status.** Implemented  
**Tracks.** Foundation / Security  
**Touches.** `src/config.rs`, `src/app.rs`

## Summary

Define the runtime behavior of `ServerConfig` and `SecurityConfig` — bind address, body size
limit, request timeout, concurrency, trusted proxy CIDR list, and source IP allowlist — and how
they translate into Axum middleware layers.

## Motivation

The server and security configuration fields are the first line of defense before any handler
runs. An incorrectly configured `max_request_body_bytes` allows DoS via oversized payloads;
an unconstrained bind address exposes the service directly to the internet; an untrusted
`X-Forwarded-For` header allows IP allowlist bypass (FR-020, FR-021, FR-022, NFR-SEC-001).

## Scope

- `ServerConfig` and `SecurityConfig` struct definitions.
- Axum middleware layers derived from config: body limit, timeout, concurrency.
- Source IP allowlist enforcement as a middleware layer.
- Trusted proxy logic: when to read `X-Forwarded-For`.
- Default values and validation (rules in RFC 021).

## Non-goals

- TLS termination (handled by reverse proxy).
- Authentication (RFC 040–044).
- Rate limiting (RFC 070).

## Design

### `ServerConfig`

```rust
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub bind_address: String,                   // default: "127.0.0.1:8080"
    pub trusted_proxy_cidrs: Vec<String>,       // default: ["127.0.0.1/32"]
    pub max_request_body_bytes: usize,          // default: 1_048_576
    pub request_timeout_seconds: u64,           // default: 10
    pub concurrency_limit: usize,               // default: 64
    pub shutdown_timeout_seconds: u64,          // default: 30
}
```

### `SecurityConfig`

```rust
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    pub require_auth: bool,                     // default: true
    pub allowed_source_cidrs: Vec<String>,      // default: ["127.0.0.1/32"]
    pub reject_raw_headers: bool,               // default: true
    pub allow_multiple_recipients: bool,        // default: false
    pub max_recipients: usize,                  // default: 1
}
```

### Middleware stack order

Axum applies middleware from outermost (first in the `layer()` chain) to innermost. The order
matters for correctness and security:

```
incoming request
  │
  ▼
[1] ConcurrencyLimitLayer     — shed excess load before doing any work
  │
  ▼
[2] TimeoutLayer              — enforce request deadline
  │
  ▼
[3] RequestBodyLimitLayer     — reject oversized bodies early (413)
  │
  ▼
[4] RequestContextLayer       — inject request_id and resolve client_ip
  │
  ▼
[5] SourceIpAllowlistLayer    — IP allowlist check (403)
  │
  ▼
[6] handlers (auth, rate limit, validation, SMTP)
```

### Source IP allowlist layer

```rust
pub struct SourceIpAllowlistLayer {
    allowed: Vec<IpNet>,
}

impl<S: Service<Request>> Service<Request> for SourceIpAllowlist<S> {
    fn call(&mut self, req: Request) -> Self::Future {
        let ctx = req.extensions().get::<RequestContext>();
        let ip = ctx.map(|c| c.client_ip).unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

        if self.allowed.iter().any(|net| net.contains(&ip)) {
            self.inner.call(req)
        } else {
            // Return 403 immediately without hitting any handler
            ready(Ok(AppError::Forbidden.into_response()))
        }
    }
}
```

If `allowed_source_cidrs` is empty, the layer is a no-op (all IPs allowed). A startup warning
is emitted in this case (RFC 021).

### Trusted proxy and client IP resolution

Client IP resolution (also in RFC 041):

```rust
fn resolve_client_ip(req: &Request, trusted_cidrs: &[IpNet]) -> IpAddr {
    let peer_ip = /* socket peer address from ConnectInfo */;

    if trusted_cidrs.iter().any(|net| net.contains(&peer_ip)) {
        // Trust X-Forwarded-For; take the leftmost (client) IP
        if let Some(fwd) = req.headers().get("X-Forwarded-For") {
            if let Ok(s) = fwd.to_str() {
                if let Some(first) = s.split(',').next() {
                    if let Ok(ip) = first.trim().parse::<IpAddr>() {
                        return ip;
                    }
                }
            }
        }
    }

    // Untrusted peer or no forwarding header: use socket peer IP
    peer_ip
}
```

The resolved IP is stored in `RequestContext.client_ip` and used for allowlist enforcement and
IP-based rate limiting.

### Body size limit

Uses `tower_http::limit::RequestBodyLimitLayer`:

```rust
let body_limit = RequestBodyLimitLayer::new(config.server.max_request_body_bytes);
```

Requests exceeding the limit receive `413 Payload Too Large` before the request body is read.

### Request timeout

Uses `tower_http::timeout::TimeoutLayer`:

```rust
let timeout = TimeoutLayer::new(Duration::from_secs(config.server.request_timeout_seconds));
```

Timed-out requests receive `408 Request Timeout` (or 500, depending on Axum version behavior;
verify during implementation).

### Concurrency limit

Uses `tower::limit::ConcurrencyLimitLayer`:

```rust
let concurrency = ConcurrencyLimitLayer::new(config.server.concurrency_limit);
```

## Implementation Plan

1. Define `ServerConfig` and `SecurityConfig` in `src/config.rs`.
2. Implement `SourceIpAllowlistLayer` in `src/api/extractors.rs` or a new `src/middleware.rs`.
3. Build the middleware stack in `src/app.rs`.
4. Wire `client_ip` resolution into `RequestContextLayer`.
5. Write tests.

## Test Plan

### Unit Tests

- Client IP from `X-Forwarded-For` is used when peer is a trusted proxy.
- Client IP from socket peer is used when peer is not a trusted proxy.
- Source IP allowlist blocks a disallowed IP with 403.
- Source IP allowlist allows a permitted IP.

### Integration Tests

- Request with body size > limit returns 413.
- Request taking longer than timeout returns 408 (or 500; verify).
- Request from disallowed IP returns 403.
- Request from trusted proxy with `X-Forwarded-For` from a blocked IP returns 403.

### Security Tests

- `X-Forwarded-For` from an untrusted peer is ignored; the socket IP is used for allowlist.
- Empty `allowed_source_cidrs` allows all IPs (verify the startup warning is emitted).

## Security Considerations

- Body size limit must be enforced before the JSON deserializer reads the body, preventing
  memory exhaustion from maliciously large payloads.
- Timeout must be enforced to prevent Slowloris-style attacks.
- The `X-Forwarded-For` trust chain must only be applied to connections from known-trusted
  proxy CIDRs. An attacker who can send requests directly (bypassing the proxy) must not be
  able to spoof the IP via this header.

## Operational Considerations

- The default bind address `127.0.0.1:8080` ensures the service is not accidentally exposed.
- Operators deploying behind nginx/Caddy/relayd must add the proxy's IP to `trusted_proxy_cidrs`.
- `concurrency_limit` prevents the service from being overwhelmed; tune based on expected load
  and SMTP server capacity.

## Documentation Changes

- Document all `[server]` and `[security]` fields in `docs/configuration.md`.
- Document trusted proxy setup in `docs/security.md`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-024-01 | Default bind address is `127.0.0.1:8080`. |
| AC-024-02 | Request body exceeding limit returns 413. |
| AC-024-03 | Request from unlisted IP returns 403 when allowlist is non-empty. |
| AC-024-04 | `X-Forwarded-For` from untrusted peer is ignored. |
| AC-024-05 | `X-Forwarded-For` from trusted proxy is used for IP resolution. |

## Open Questions

None.
