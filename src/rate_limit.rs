//! Three-tier token bucket rate limiting: global, per-IP, per-key.
//!
//! Implements RFC 070–071, 201–203, 206.
//!
//! # Tiers
//! - Global: all requests before auth.
//! - Per-IP: per resolved client IP before auth.
//! - Per-key: per API key_id after auth.
//!
//! # Burst
//! Each tier has its own burst capacity (RFC 201).
//! Per-key burst can be further overridden per key (RFC 203).
//!
//! # LRU eviction
//! The per-IP map is capped at `ip_table_size` entries (RFC 206).
//! Evicted IPs get a fresh full bucket on next access.

use std::{
    collections::VecDeque,
    net::IpAddr,
    time::Instant,
};
use parking_lot::Mutex;

use crate::config::RateLimitConfig;

// ---------------------------------------------------------------------------
// Token bucket
// ---------------------------------------------------------------------------

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
// LRU-capped IP map
// ---------------------------------------------------------------------------

/// A `HashMap` with a LRU-eviction cap on the number of entries.
///
/// Eviction order is approximate (insert order, not strict LRU) but sufficient
/// for abuse prevention — the goal is bounding memory, not exact LRU semantics.
struct LruMap<K, V> {
    map: std::collections::HashMap<K, V>,
    order: VecDeque<K>,
    cap: usize,
}

impl<K: Clone + std::hash::Hash + Eq> LruMap<K, TokenBucket> {
    fn new(cap: usize) -> Self {
        Self {
            map: std::collections::HashMap::new(),
            order: VecDeque::new(),
            cap,
        }
    }

    /// Get or insert a bucket, evicting the oldest entry if over capacity.
    fn get_or_insert(&mut self, key: K, per_minute: u32, burst: u32) -> &mut TokenBucket {
        // Evict if at capacity and key is new
        if self.cap > 0 && !self.map.contains_key(&key) && self.map.len() >= self.cap {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            }
        }
        if !self.map.contains_key(&key) {
            self.map.insert(key.clone(), TokenBucket::new(per_minute, burst));
            self.order.push_back(key.clone());
        }
        self.map.get_mut(&key).unwrap()
    }
}

// ---------------------------------------------------------------------------
// Rate limiter
// ---------------------------------------------------------------------------

pub struct RateLimiter {
    global: Mutex<TokenBucket>,
    by_ip: Mutex<LruMap<IpAddr, TokenBucket>>,
    by_key: Mutex<std::collections::HashMap<String, TokenBucket>>,

    // Tier configurations (captured at construction)
    #[allow(dead_code)] global_per_min: u32,
    #[allow(dead_code)] global_burst:    u32,
    ip_per_min:       u32,
    ip_burst:         u32,
    key_per_min:      u32,   // default; overridden per key
    key_burst:        u32,   // default; overridden per key
}

impl RateLimiter {
    pub fn new(cfg: &RateLimitConfig) -> Self {
        let global_burst = cfg.effective_global_burst();
        let ip_burst     = cfg.effective_per_ip_burst();
        let key_burst    = cfg.effective_per_key_burst();
        let ip_table_cap = cfg.ip_table_size;

        // Emit a deprecation warning if only the legacy field was set
        if cfg.burst_size > 0
            && cfg.global_burst == 0
            && cfg.per_ip_burst == 0
            && cfg.per_key_burst == 0
        {
            tracing::warn!(
                "rate_limit.burst_size is deprecated; \
                 use global_burst, per_ip_burst, per_key_burst instead"
            );
        }

        Self {
            global: Mutex::new(TokenBucket::new(cfg.global_per_min, global_burst)),
            by_ip:  Mutex::new(LruMap::new(ip_table_cap)),
            by_key: Mutex::new(std::collections::HashMap::new()),
            global_per_min: cfg.global_per_min,
            global_burst,
            ip_per_min:  cfg.per_ip_per_min,
            ip_burst,
            key_per_min: cfg.per_key_per_min,
            key_burst,
        }
    }

    pub fn check_global(&self) -> Result<(), RateLimitedError> {
        self.global.lock().try_consume()
            .map_err(|s| RateLimitedError { tier: "global", retry_after_secs: s })
    }

    pub fn check_ip(&self, ip: IpAddr) -> Result<(), RateLimitedError> {
        let (pm, burst) = (self.ip_per_min, self.ip_burst);
        self.by_ip.lock()
            .get_or_insert(ip, pm, burst)
            .try_consume()
            .map_err(|s| RateLimitedError { tier: "ip", retry_after_secs: s })
    }

    /// Check per-key rate limit.
    ///
    /// `per_min_override`: `ApiKeyConfig.rate_limit_per_min` (None = use default).
    /// `burst_override`:   `ApiKeyConfig.burst` (0 = use default).
    pub fn check_key(
        &self,
        key_id: &str,
        per_min_override: Option<u32>,
        burst_override: u32,
    ) -> Result<(), RateLimitedError> {
        let pm    = per_min_override.unwrap_or(self.key_per_min);
        let burst = if burst_override > 0 { burst_override } else { self.key_burst };
        self.by_key.lock()
            .entry(key_id.to_string())
            .or_insert_with(|| TokenBucket::new(pm, burst))
            .try_consume()
            .map_err(|s| RateLimitedError { tier: "key", retry_after_secs: s })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(global_per_min: u32, ip_per_min: u32, burst: u32) -> RateLimitConfig {
        RateLimitConfig {
            global_per_min,
            per_ip_per_min: ip_per_min,
            per_key_per_min: 30,
            global_burst: burst,
            per_ip_burst: burst,
            per_key_burst: burst,
            burst_size: 0,
            ip_table_size: 10_000,
        }
    }

    #[test]
    fn fresh_bucket_allows_burst() {
        let rl = RateLimiter::new(&cfg(60, 20, 5));
        for _ in 0..5 { assert!(rl.check_global().is_ok()); }
    }

    #[test]
    fn burst_exhaustion_returns_err() {
        let rl = RateLimiter::new(&cfg(60, 20, 3));
        for _ in 0..3 { rl.check_global().unwrap(); }
        let e = rl.check_global().unwrap_err();
        assert_eq!(e.tier, "global");
        assert!(e.retry_after_secs >= 1);
    }

    #[test]
    fn two_ips_are_independent() {
        let rl = RateLimiter::new(&cfg(60, 20, 2));
        let a: IpAddr = "1.2.3.4".parse().unwrap();
        let b: IpAddr = "5.6.7.8".parse().unwrap();
        rl.check_ip(a).unwrap();
        rl.check_ip(a).unwrap();
        assert!(rl.check_ip(a).is_err());
        assert!(rl.check_ip(b).is_ok());
    }

    #[test]
    fn two_keys_are_independent() {
        let rl = RateLimiter::new(&cfg(60, 20, 2));
        rl.check_key("a", None, 0).unwrap();
        rl.check_key("a", None, 0).unwrap();
        assert!(rl.check_key("a", None, 0).is_err());
        assert!(rl.check_key("b", None, 0).is_ok());
    }

    #[test]
    fn per_key_burst_override_respected() {
        let rl = RateLimiter::new(&cfg(60, 20, 2));
        // Key with burst=5 override
        for _ in 0..5 { rl.check_key("hi-burst", None, 5).unwrap(); }
        assert!(rl.check_key("hi-burst", None, 5).is_err());
    }

    #[test]
    fn lru_eviction_caps_map_size() {
        let mut map: LruMap<u32, TokenBucket> = LruMap::new(3);
        for i in 0u32..5 {
            map.get_or_insert(i, 60, 5);
        }
        assert_eq!(map.map.len(), 3, "map should be capped at 3");
    }

    #[test]
    fn per_tier_burst_effective_values() {
        let cfg = RateLimitConfig {
            global_per_min: 60, per_ip_per_min: 20, per_key_per_min: 30,
            global_burst: 10, per_ip_burst: 5, per_key_burst: 8,
            burst_size: 0, ip_table_size: 100,
        };
        assert_eq!(cfg.effective_global_burst(), 10);
        assert_eq!(cfg.effective_per_ip_burst(), 5);
        assert_eq!(cfg.effective_per_key_burst(), 8);
    }

    #[test]
    fn legacy_burst_size_falls_back() {
        let cfg = RateLimitConfig {
            global_per_min: 60, per_ip_per_min: 20, per_key_per_min: 30,
            global_burst: 0, per_ip_burst: 0, per_key_burst: 0,
            burst_size: 7, ip_table_size: 100,
        };
        assert_eq!(cfg.effective_global_burst(), 7);
        assert_eq!(cfg.effective_per_ip_burst(), 7);
        assert_eq!(cfg.effective_per_key_burst(), 7);
    }
}
