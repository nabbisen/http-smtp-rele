//! Prometheus metrics instrumentation.
//!
//! Implements RFC 401: counters and histograms for requests, SMTP submissions,
//! auth failures, and rate limit events. Exposed via `GET /metrics`.
//!
//! # Metrics
//!
//! | Name | Type | Labels |
//! |------|------|--------|
//! | `rele_requests_total` | Counter | `status` (2xx/4xx/5xx) |
//! | `rele_smtp_submissions_total` | Counter | `result` (ok/error) |
//! | `rele_smtp_duration_seconds` | Histogram | — |
//! | `rele_auth_failures_total` | Counter | `reason` |
//! | `rele_rate_limited_total` | Counter | `tier` |
//! | `rele_validation_failures_total` | Counter | — |

use prometheus::{
    exponential_buckets, register_counter_vec_with_registry,
    register_histogram_with_registry, CounterVec, Histogram, Registry,
};

/// All Prometheus metrics for the relay.
pub struct Metrics {
    pub registry: Registry,

    /// Total HTTP requests processed, by response status class.
    pub requests_total: CounterVec,

    /// SMTP submission attempts, by result.
    pub smtp_submissions_total: CounterVec,

    /// SMTP session duration in seconds.
    pub smtp_duration_seconds: Histogram,

    /// Authentication failures, by reason.
    pub auth_failures_total: CounterVec,

    /// Rate limit hits, by tier.
    pub rate_limited_total: CounterVec,

    /// Validation failures.
    pub validation_failures_total: CounterVec,

    // ── Status store metrics (RFC 089 / RFC 601) ──────────────────────────
    /// Current number of live records in the status store (gauge).
    pub status_records_current: prometheus::Gauge,
    /// Cumulative status transitions by status and code.
    pub status_transitions_total: CounterVec,
    /// Records removed by TTL expiry (lazy + periodic combined).
    pub status_expired_total: prometheus::Counter,
}

impl Metrics {
    /// Create and register all metrics in a fresh registry.
    pub fn new() -> Self {
        let registry = Registry::new();

        let requests_total = register_counter_vec_with_registry!(
            "rele_requests_total",
            "Total HTTP requests processed by response status class",
            &["status"],
            registry
        )
        .expect("metric registration failed: rele_requests_total");

        let smtp_submissions_total = register_counter_vec_with_registry!(
            "rele_smtp_submissions_total",
            "SMTP submission attempts by result",
            &["result"],
            registry
        )
        .expect("metric registration failed: rele_smtp_submissions_total");

        let smtp_duration_seconds = register_histogram_with_registry!(
            "rele_smtp_duration_seconds",
            "SMTP session duration in seconds",
            exponential_buckets(0.001, 2.0, 14).unwrap(), // 1ms .. ~8s
            registry
        )
        .expect("metric registration failed: rele_smtp_duration_seconds");

        let auth_failures_total = register_counter_vec_with_registry!(
            "rele_auth_failures_total",
            "Authentication failures by reason",
            &["reason"],
            registry
        )
        .expect("metric registration failed: rele_auth_failures_total");

        let rate_limited_total = register_counter_vec_with_registry!(
            "rele_rate_limited_total",
            "Rate limit hits by tier",
            &["tier"],
            registry
        )
        .expect("metric registration failed: rele_rate_limited_total");

        let validation_failures_total = register_counter_vec_with_registry!(
            "rele_validation_failures_total",
            "Validation failures by field",
            &["field"],
            registry
        )
        .expect("metric registration failed: rele_validation_failures_total");

        let status_records_current = prometheus::register_gauge_with_registry!(
            "rele_status_store_records_current",
            "Current number of live records in the submission status store",
            registry
        ).expect("metric registration failed: rele_status_store_records_current");

        let status_transitions_total = register_counter_vec_with_registry!(
            "rele_status_store_transitions_total",
            "Cumulative submission status transitions by status and code",
            &["status", "code"],
            registry
        ).expect("metric registration failed: rele_status_store_transitions_total");

        let status_expired_total = prometheus::register_counter_with_registry!(
            "rele_status_store_expired_total",
            "Records removed by TTL expiration",
            registry
        ).expect("metric registration failed: rele_status_store_expired_total");

        Self {
            registry,
            requests_total,
            smtp_submissions_total,
            smtp_duration_seconds,
            auth_failures_total,
            rate_limited_total,
            validation_failures_total,
            status_records_current,
            status_transitions_total,
            status_expired_total,
        }
    }

    // ---------------------------------------------------------------------------
    // Convenience increment methods
    // ---------------------------------------------------------------------------

    pub fn inc_request(&self, status_class: &str) {
        self.requests_total.with_label_values(&[status_class]).inc();
    }

    pub fn inc_smtp_ok(&self) {
        self.smtp_submissions_total.with_label_values(&["ok"]).inc();
    }

    pub fn inc_smtp_error(&self) {
        self.smtp_submissions_total.with_label_values(&["error"]).inc();
    }

    pub fn observe_smtp_duration(&self, secs: f64) {
        self.smtp_duration_seconds.observe(secs);
    }

    pub fn inc_auth_failure(&self, reason: &str) {
        self.auth_failures_total.with_label_values(&[reason]).inc();
    }

    pub fn inc_rate_limited(&self, tier: &str) {
        self.rate_limited_total.with_label_values(&[tier]).inc();
    }

    pub fn inc_validation_failure(&self, field: &str) {
        self.validation_failures_total.with_label_values(&[field]).inc();
    }

    // ── Status store helpers (RFC 601) ────────────────────────────────────

    pub fn status_record_created(&self) {
        self.status_records_current.inc();
        self.status_transitions_total.with_label_values(&["received", "none"]).inc();
    }

    pub fn status_transitioned(&self, status: &str, code: &str) {
        self.status_transitions_total.with_label_values(&[status, code]).inc();
    }

    pub fn status_records_expired(&self, count: usize) {
        if count > 0 {
            self.status_records_current.sub(count as f64);
            self.status_expired_total.inc_by(count as f64);
        }
    }

    /// Increment the status store error counter (RFC 814).
    pub fn status_store_error(&self, operation: &str) {
        tracing::warn!(event = "status_store_error", operation = %operation);
    }

    pub fn status_record_expired_one(&self) {
        self.status_records_current.dec();
        self.status_expired_total.inc();
    }

    pub fn status_set_current(&self, count: usize) {
        self.status_records_current.set(count as f64);
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Serialize all metrics in the registry to Prometheus text format.
pub fn encode(registry: &Registry) -> Result<String, String> {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metric_families = registry.gather();
    let mut buf = Vec::new();
    encoder
        .encode(&metric_families, &mut buf)
        .map_err(|e| format!("metrics encoding failed: {e}"))?;
    String::from_utf8(buf).map_err(|e| format!("metrics UTF-8 error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_register_without_panic() {
        let m = Metrics::new();
        m.inc_request("2xx");
        m.inc_smtp_ok();
        m.inc_auth_failure("invalid_token");
        m.inc_rate_limited("global");
        m.inc_validation_failure("subject");
        let output = encode(&m.registry).unwrap();
        assert!(output.contains("rele_requests_total"));
        assert!(output.contains("rele_smtp_submissions_total"));
        assert!(output.contains("rele_auth_failures_total"));
    }

    #[test]
    fn request_counter_increments() {
        let m = Metrics::new();
        m.inc_request("2xx");
        m.inc_request("2xx");
        m.inc_request("4xx");
        let out = encode(&m.registry).unwrap();
        // Check that the text format contains our metric
        assert!(out.contains(r#"rele_requests_total{status="2xx"} 2"#));
        assert!(out.contains(r#"rele_requests_total{status="4xx"} 1"#));
    }
}
