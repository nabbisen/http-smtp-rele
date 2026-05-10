# RFC 071 — Token Bucket Implementation

**Status.** Implemented  
**Tracks.** Security  
**Touches.** `src/rate_limit.rs`

## Summary

Implement an in-process token bucket algorithm that supports a configurable refill rate and
burst capacity, used by all three tiers of the rate limiter (RFC 070).

## Motivation

The token bucket algorithm is well-understood, supports burst tolerance, and is easy to reason
about. A lazy refill implementation (tokens added based on elapsed time, computed at check time)
requires no background tasks and is compatible with OpenBSD's `pledge("stdio inet")` constraint
(FR-030, FR-031, FR-032).

## Scope

- `TokenBucket` struct: current token count, capacity, refill rate, last refill timestamp.
- `TokenBucket::try_consume(n: u32) -> Result<(), RetryAfter>`.
- Lazy refill: compute tokens to add at check time based on elapsed duration.
- `BucketConfig`: capacity (burst) and refill rate (tokens per minute).
- Thread safety: `Mutex<TokenBucket>` per bucket.

## Non-goals

- Distributed token buckets (not in MVP).
- Fractional token consumption.
- Per-request weight (all requests consume one token).

## Design

### `BucketConfig`

```rust
#[derive(Clone, Debug)]
pub struct BucketConfig {
    /// Maximum tokens (burst capacity).
    pub capacity: u32,
    /// Tokens added per minute (sustained rate).
    pub per_minute: u32,
}
```

### `TokenBucket`

```rust
pub struct TokenBucket {
    config: BucketConfig,
    /// Current token count (floating point for fractional accumulation).
    tokens: f64,
    /// Time of last refill computation.
    last_refill: Instant,
}

impl TokenBucket {
    pub fn new(config: BucketConfig) -> Self {
        Self {
            tokens: config.capacity as f64,  // start full
            last_refill: Instant::now(),
            config,
        }
    }

    /// Attempt to consume one token.
    ///
    /// Lazily refills before checking.
    /// Returns `Ok(())` on success, `Err(secs)` with estimated retry delay on failure.
    pub fn try_consume(&mut self) -> Result<(), u64> {
        self.refill();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            Ok(())
        } else {
            // Time until one token is available.
            let tokens_per_sec = self.config.per_minute as f64 / 60.0;
            let wait_secs = ((1.0 - self.tokens) / tokens_per_sec).ceil() as u64;
            Err(wait_secs)
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let tokens_per_sec = self.config.per_minute as f64 / 60.0;
        let new_tokens = elapsed * tokens_per_sec;

        self.tokens = (self.tokens + new_tokens).min(self.config.capacity as f64);
        self.last_refill = now;
    }
}
```

### Thread safety

Each `TokenBucket` is wrapped in a `Mutex`:

```rust
// Inside RateLimiter:
global: Mutex<TokenBucket>,
by_ip: Mutex<HashMap<IpAddr, TokenBucket>>,
by_key: Mutex<HashMap<String, TokenBucket>>,
```

The `Mutex` is a `tokio::sync::Mutex` so `try_consume` can be called from async contexts
without blocking the executor. The critical section is very short (lazy refill + subtract),
so contention is minimal.

### Starting state

Buckets start full (all tokens available). This means a fresh process allows a full burst
immediately. This is intentional: startup should not penalize legitimate first-use.

### Precision

Using `f64` for token count introduces floating-point rounding at sub-token granularities.
This is acceptable: rate limiting does not require exact token counts; approximate enforcement
is sufficient for abuse prevention.

## Implementation Plan

1. Define `BucketConfig` and `TokenBucket` in `src/rate_limit.rs`.
2. Implement `try_consume` and `refill`.
3. Implement `RateLimiter::check_global`, `check_ip`, `check_key` using `try_consume`.
4. Write unit tests.

## Test Plan

### Unit Tests

- Full bucket: `try_consume` succeeds `capacity` times in rapid succession.
- Empty bucket: `try_consume` after capacity exhaustion returns `Err`.
- Refill: after sleeping (or advancing a mock clock), `try_consume` succeeds again.
- Burst: a bucket with `capacity = 5, per_minute = 60` allows 5 quick requests.
- `retry_after`: returned wait seconds are positive and approximately correct.
- Starting state: bucket starts with `capacity` tokens.

### Integration Tests

- Sending `capacity + 1` requests returns exactly one 429 on the last.
- After `60 / per_minute` seconds, one new token is available.

## Security Considerations

- Floating point arithmetic does not affect security; small rounding errors in token count
  cannot be exploited to meaningfully exceed the rate limit.
- The bucket starts full to avoid penalizing legitimate traffic on restart, but this means a
  restart allows a burst. Documented in RFC 070.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-071-01 | A fresh bucket allows `capacity` consecutive requests. |
| AC-071-02 | The `(capacity + 1)`th request is rejected. |
| AC-071-03 | `try_consume` returns a positive retry estimate on failure. |
| AC-071-04 | Tokens refill over time (lazy refill is correct). |

## Open Questions

None.
