# RFC 041 — Source IP and Trusted Proxy Handling

**Status.** Implemented  
**Tracks.** Security  
**Touches.** `src/context.rs`, `src/app.rs`

## Summary

Define the algorithm for resolving the client's true IP address when a trusted reverse proxy
is in the path, the rules for trusting `X-Forwarded-For`, and the integration with the source
IP allowlist.

## Motivation

If `X-Forwarded-For` is trusted unconditionally, any attacker can forge their apparent source
IP by adding that header directly. If it is never trusted, operators behind a NAT'd reverse
proxy cannot use IP allowlisting for their actual clients. The resolution algorithm must be
correct and conservative (FR-022, requirement §8.3).

## Scope

- `resolve_client_ip(req, trusted_cidrs) → IpAddr`.
- Trusted proxy CIDR matching.
- `X-Forwarded-For` parsing: take the leftmost entry.
- Fallback: socket peer address when no trusted proxy or no forwarding header.
- Integration with `RequestContextLayer` (RFC 011).

## Non-goals

- Source IP allowlist enforcement (RFC 024).
- Rate limiting by IP (RFC 070).
- W3C `Forwarded` header support (deferred to v0.2).

## Design

### Resolution algorithm

```
1. Read the socket peer IP (from ConnectInfo<SocketAddr>).
2. If peer IP is in trusted_proxy_cidrs:
   a. Read X-Forwarded-For header.
   b. If present and parseable:
      - Split by comma.
      - Take the leftmost entry (client IP added by the first proxy).
      - Trim whitespace.
      - Parse as IpAddr.
      - If parse succeeds: use this IP.
      - If parse fails: fall through to step 3.
3. Use the socket peer IP.
```

The leftmost-entry rule is the standard approach: each proxy SHOULD append the address of
the client it received the request from. The leftmost entry is the original client.

```rust
pub fn resolve_client_ip(
    peer_ip: IpAddr,
    headers: &HeaderMap,
    trusted_cidrs: &[IpNet],
) -> IpAddr {
    if !trusted_cidrs.iter().any(|net| net.contains(&peer_ip)) {
        return peer_ip;
    }

    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(str::trim)
        .and_then(|s| s.parse::<IpAddr>().ok())
        .unwrap_or(peer_ip)
}
```

### Multi-hop proxies

If there are multiple trusted proxies in sequence, each adds a hop to `X-Forwarded-For`.
For example: `X-Forwarded-For: 1.2.3.4, 10.0.0.1`
- `1.2.3.4` is the original client.
- `10.0.0.1` is an intermediate proxy.

The algorithm takes the leftmost (`1.2.3.4`), which is the intended behavior.

### `ConnectInfo` extraction

Axum provides `ConnectInfo<SocketAddr>` via the router's `.into_make_service_with_connect_info::<SocketAddr>()`.
The peer address is extracted in `RequestContextLayer`:

```rust
let peer_ip: IpAddr = req
    .extensions()
    .get::<ConnectInfo<SocketAddr>>()
    .map(|c| c.0.ip())
    .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
```

## Implementation Plan

1. Implement `resolve_client_ip` in `src/context.rs`.
2. Integrate into `RequestContextLayer`.
3. Use `.into_make_service_with_connect_info::<SocketAddr>()` in `app::run`.
4. Write unit tests for the resolution algorithm.

## Test Plan

### Unit Tests

- Peer in trusted CIDR + `X-Forwarded-For: 1.2.3.4` → resolved IP is `1.2.3.4`.
- Peer NOT in trusted CIDR + `X-Forwarded-For: 1.2.3.4` → resolved IP is peer IP.
- Peer in trusted CIDR + no `X-Forwarded-For` → resolved IP is peer IP.
- Peer in trusted CIDR + malformed `X-Forwarded-For` → resolved IP is peer IP.
- Multi-hop: `X-Forwarded-For: 1.2.3.4, 10.0.0.1` → resolved IP is `1.2.3.4`.
- IPv6 client address is handled correctly.

### Security Tests

- Forged `X-Forwarded-For` from an untrusted peer is ignored; socket IP is used.
- An attacker cannot bypass IP allowlist by spoofing `X-Forwarded-For`.

## Security Considerations

- Never trust `X-Forwarded-For` from an untrusted peer. The trusted CIDR check is the guard.
- The leftmost IP in `X-Forwarded-For` is controlled by the client, not the proxy, when no
  intermediate proxies are present. If the proxy itself is the first to set it (not append),
  the leftmost IP may still be attacker-controlled. Document this limitation and recommend
  proxy configurations that append rather than replace.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-041-01 | IP from `X-Forwarded-For` is used only when peer is in trusted CIDR. |
| AC-041-02 | Leftmost `X-Forwarded-For` entry is used. |
| AC-041-03 | Malformed `X-Forwarded-For` falls back to peer IP. |
| AC-041-04 | Untrusted peer's `X-Forwarded-For` is ignored entirely. |

## Open Questions

None.
