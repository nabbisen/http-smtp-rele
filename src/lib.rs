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
}

impl AppState {
    /// Build application state from a validated config.
    pub fn new(config: config::AppConfig) -> Arc<Self> {
        let smtp = smtp::build_transport(&config.smtp)
            .expect("SMTP transport construction failed after config validation");
        let rate_limiter = Arc::new(rate_limit::RateLimiter::new(&config.rate_limit));
        Arc::new(Self {
            config_store: arc_swap::ArcSwap::from_pointee(config),
            smtp,
            rate_limiter,
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
    pub fn reload_config(&self, new_config: config::AppConfig) {
        self.config_store.store(Arc::new(new_config));
        tracing::info!(event = "config_reloaded");
    }
}
