# RFC 303 — W3C Forwarded Header

**Status.** Implemented  
**Tracks.** Security / Foundation  
**Touches.** `src/auth.rs`

## Summary

Parse the W3C standard `Forwarded` header (`Forwarded: for=1.2.3.4`) as an alternative
to `X-Forwarded-For` when resolving client IP.

## Design

Priority when `trust_proxy_headers = true` and peer is in `trusted_source_cidrs`:
1. `Forwarded: for=<addr>` (RFC 7239) — preferred
2. `X-Forwarded-For: <addr>` — fallback

```rust
fn parse_forwarded_for(headers: &HeaderMap) -> Option<IpAddr> {
    headers.get("forwarded")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            // "for=1.2.3.4" or "for=\"[::1]\""
            s.split(';')
                .find(|p| p.trim().to_lowercase().starts_with("for="))
                .and_then(|p| p.trim()["for=".len()..].trim()
                    .trim_matches('"').trim_matches('[').trim_matches(']')
                    .parse::<IpAddr>().ok())
        })
}
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-303-01 | `Forwarded: for=1.2.3.4` from trusted proxy resolves to 1.2.3.4. |
| AC-303-02 | `Forwarded` takes precedence over `X-Forwarded-For` when both present. |
| AC-303-03 | Malformed `Forwarded` header falls back to `X-Forwarded-For` then peer IP. |
| AC-303-04 | IPv6 addresses in `Forwarded` are parsed correctly. |
