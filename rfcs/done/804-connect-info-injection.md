# RFC 804 — RB-04: ConnectInfo Injection for Source IP Determination

**Status.** Proposed  
**Tracks.** T1 — Security / T3 — Rate Limiting  
**Touches.** `crates/cli/src/main.rs`, `src/auth.rs`, tests

## Problem

`auth.rs` extracts the client IP from:

```rust
parts.extensions.get::<axum::extract::ConnectInfo<SocketAddr>>()
```

But the CLI serves with:

```rust
axum::serve(listener, router)
```

This form does **not** inject `ConnectInfo<SocketAddr>`. The fallback is
always `127.0.0.1`, meaning:

- `allowed_source_cidrs` never matches non-loopback addresses
- `trusted_source_cidrs` never matches
- `X-Forwarded-For` trust decisions are always wrong
- Per-IP rate limiting always counts against `127.0.0.1`

## Fix

Change `axum::serve` to `into_make_service_with_connect_info`:

```rust
axum::serve(
    listener,
    router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
)
```

For TLS (`--features tls`), apply the same pattern to `axum_server::bind_rustls`.

## Tests

Add an integration test that binds a real TCP listener, sends a request,
and verifies that the IP address in the rate limiter is **not** 127.0.0.1
when the actual client address is different.

`Router::oneshot` bypasses this path and cannot detect the bug.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-804-01 | `ConnectInfo<SocketAddr>` is populated in all serving paths. |
| AC-804-02 | Per-IP rate limit correctly identifies the real peer address over TCP. |
| AC-804-03 | Integration test using a real TCP listener passes. |
