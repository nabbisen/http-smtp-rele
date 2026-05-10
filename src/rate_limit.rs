//! Three-tier token bucket rate limiting: global, per-IP, per-key.
//!
//! Implements RFC 070–071: in-process, in-memory rate limiting with lazy
//! token-bucket refill. State resets on process restart (documented limitation).
//!
//! # Tier order
//!
//! ```text
//! Request
//!   -> [1] global limiter  (all requests)
//!   -> [2] per-IP limiter  (per resolved client IP)
//!   -> [3] per-key limiter (after auth, per key_id)
//! ```

use std::{
    collections::HashMap,
    net::IpAddr,
    sync::Mutex,
    time::Instant,
};

use crate::config::RateLimitConfig;

// ---------------------------------------------------------------------------
// Token bucket
// ---------------------------------------------------------------------------

/// A lazy-refill token bucket.
///
/// Tokens accumulate over time up to `capacity`. Each successful request
/// consumes one token. When empty, requests are rejected with a retry estimate.
struct TokenBucket {
    capacity: f64,
    tokens_per_sec: f64,
    tokens: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(per_minute: u32, burst: u32) -> Self {
        let capacity = burst as f64;
        Self {
            capacity,
            tokens_per_sec: per_minute as f64 / 60.0,
            tokens: capacity,
            last_refill: Instant::now(),
        }
    }

    fn try_consume(&mut self) -> Result<(), u64> {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            Ok(())
        } else {
            let wait = ((1.0 - self.tokens) / self.tokens_per_sec).ceil() as u64;
            Err(wait.max(1))
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.tokens_per_sec).min(self.capacity);
        self.last_refill = now;
    }
}

// ---------------------------------------------------------------------------
// Rate limiter error
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct RateLimitedError {
    pub tier: &'static str,
    pub retry_after_secs: u64,
}

// ---------------------------------------------------------------------------
// Rate limiter
// ---------------------------------------------------------------------------

/// In-process three-tier token bucket rate limiter.
pub struct RateLimiter {
    global: Mutex<TokenBucket>,
    by_ip: Mutex<HashMap<IpAddr, TokenBucket>>,
    by_key: Mutex<HashMap<String, TokenBucket>>,
    ip_per_min: u32,
    burst: u32,
    key_per_min: u32,
}

impl RateLimiter {
    pub fn new(cfg: &RateLimitConfig) -> Self {
        Self {
            global: Mutex::new(TokenBucket::new(cfg.global_per_min, cfg.burst_size)),
            by_ip: Mutex::new(HashMap::new()),
            by_key: Mutex::new(HashMap::new()),
            ip_per_min: cfg.per_ip_per_min,
            burst: cfg.burst_size,
            key_per_min: cfg.per_ip_per_min,
        }
    }

    pub fn check_global(&self) -> Result<(), RateLimitedError> {
        self.global.lock().unwrap().try_consume()
            .map_err(|secs| RateLimitedError { tier: "global", retry_after_secs: secs })
    }

    pub fn check_ip(&self, ip: IpAddr) -> Result<(), RateLimitedError> {
        let (pm, burst) = (self.ip_per_min, self.burst);
        self.by_ip.lock().unwrap()
            .entry(ip)
            .or_insert_with(|| TokenBucket::new(pm, burst))
            .try_consume()
            .map_err(|secs| RateLimitedError { tier: "ip", retry_after_secs: secs })
    }

    pub fn check_key(&self, key_id: &str, per_minute_override: Option<u32>) -> Result<(), RateLimitedError> {
        let pm = per_minute_override.unwrap_or(self.key_per_min);
        let burst = self.burst;
        self.by_key.lock().unwrap()
            .entry(key_id.to_string())
            .or_insert_with(|| TokenBucket::new(pm, burst))
            .try_consume()
            .map_err(|secs| RateLimitedError { tier: "key", retry_after_secs: secs })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(per_min: u32, burst: u32) -> RateLimitConfig {
        RateLimitConfig { global_per_min: per_min, per_ip_per_min: per_min, burst_size: burst }
    }

    #[test]
    fn fresh_bucket_allows_burst() {
        let rl = RateLimiter::new(&cfg(60, 5));
        for _ in 0..5 { assert!(rl.check_global().is_ok()); }
    }

    #[test]
    fn burst_exhaustion_returns_err() {
        let rl = RateLimiter::new(&cfg(60, 3));
        for _ in 0..3 { rl.check_global().unwrap(); }
        let e = rl.check_global().unwrap_err();
        assert_eq!(e.tier, "global");
        assert!(e.retry_after_secs >= 1);
    }

    #[test]
    fn two_ips_are_independent() {
        let rl = RateLimiter::new(&cfg(60, 2));
        let a: IpAddr = "1.2.3.4".parse().unwrap();
        let b: IpAddr = "5.6.7.8".parse().unwrap();
        rl.check_ip(a).unwrap();
        rl.check_ip(a).unwrap();
        assert!(rl.check_ip(a).is_err());
        assert!(rl.check_ip(b).is_ok());
    }

    #[test]
    fn two_keys_are_independent() {
        let rl = RateLimiter::new(&cfg(60, 2));
        rl.check_key("a", None).unwrap();
        rl.check_key("a", None).unwrap();
        assert!(rl.check_key("a", None).is_err());
        assert!(rl.check_key("b", None).is_ok());
    }
}
