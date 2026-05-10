//! http-smtp-rele: minimal, secure HTTP-to-SMTP submission relay.
//!
//! # Architecture
//!
//! ```text
//! External Client
//!   | HTTPS POST /v1/send
//! Reverse Proxy / TLS Endpoint
//!   | HTTP localhost
//! http-smtp-rele
//!   | SMTP localhost:25
//! OpenSMTPD / SMTP Server
//! ```
//!
//! Responsibility: auth, validation, sanitization, rate limit, SMTP submission.

use std::sync::Arc;

pub use request_id::RequestId;

pub mod api;
pub mod auth;
pub mod config;
pub mod error;
pub mod logging;
pub mod mail;
pub mod policy;
pub mod rate_limit;
pub mod sanitize;
pub mod security;
pub mod smtp;
pub mod metrics;
pub mod request_id;
pub mod status;
pub mod status_memory;
pub mod validation;

#[cfg(test)]
mod tests;

/// Shared application state, cloned into every Axum request via `Arc`.
///
/// `config` is wrapped in `ArcSwap` for atomic hot-swap on SIGHUP (RFC 305).
/// All code accesses config via `state.config()`, which returns a snapshot
/// `Arc<AppConfig>` valid for the lifetime of one request.
pub struct AppState {
    config_store: arc_swap::ArcSwap<config::AppConfig>,
    pub smtp: smtp::SmtpTransport,
    pub rate_limiter: Arc<rate_limit::RateLimiter>,
    pub metrics: Arc<metrics::Metrics>,
    /// Submission status store (RFC 086/087).
    pub status_store: Arc<dyn status::StatusStore>,
}

impl AppState {
    /// Build application state from a validated config.
    pub fn new(config: config::AppConfig) -> Arc<Self> {
        let smtp = smtp::build_transport(&config.smtp)
            .expect("SMTP transport construction failed after config validation");
        let rate_limiter = Arc::new(rate_limit::RateLimiter::new(&config.rate_limit));
        let m = Arc::new(metrics::Metrics::new());
        let status_store: Arc<dyn status::StatusStore> = if config.status.enabled {
            status_memory::InMemoryStatusStore::new(&config.status, Arc::clone(&m))
        } else {
            Arc::new(status_memory::NoopStatusStore)
        };
        Arc::new(Self {
            config_store: arc_swap::ArcSwap::from_pointee(config),
            smtp,
            rate_limiter,
            metrics: m,
            status_store,
        })
    }

    /// Load a snapshot of the current config.
    ///
    /// Returns an `Arc<AppConfig>` that remains valid even if a concurrent
    /// SIGHUP reload replaces the stored config.
    pub fn config(&self) -> Arc<config::AppConfig> {
        self.config_store.load_full()
    }

    /// Replace the stored config atomically (called on SIGHUP, RFC 305).
    ///
    /// SIGHUP-reloadable status fields: `ttl_seconds`, `max_records`, `cleanup_interval_seconds`.
    pub fn reload_config(&self, new_config: config::AppConfig) {
        self.status_store.reload_config(&new_config.status);
        self.config_store.store(Arc::new(new_config));
        tracing::info!(event = "config_reloaded");
    }
}
