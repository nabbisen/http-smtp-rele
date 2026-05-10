# RFC 205 — Server Concurrency Limit

**Status.** Implemented  
**Tracks.** Foundation / Config  
**Touches.** `src/config.rs`, `src/api/mod.rs`

## Summary

Add `server.concurrency_limit` to cap the number of in-flight requests via
`tower::limit::ConcurrencyLimitLayer`.

## Design

```toml
[server]
concurrency_limit = 100  # 0 = unlimited (default)
```

Applied as the outermost middleware layer (before body limit and timeout) so excess
requests receive 503 immediately without consuming router resources.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-205-01 | Requests beyond `concurrency_limit` receive 503. |
| AC-205-02 | `concurrency_limit = 0` disables the limit. |
