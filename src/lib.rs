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
pub struct AppState {
    pub config: Arc<config::AppConfig>,
    pub smtp: smtp::SmtpTransport,
    pub rate_limiter: Arc<rate_limit::RateLimiter>,
}

impl AppState {
    /// Build application state from config.
    ///
    /// Constructs the SMTP transport and rate limiter. Panics if the SMTP
    /// transport cannot be constructed (invalid config that passed validation).
    pub fn new(config: config::AppConfig) -> Arc<Self> {
        let smtp = smtp::build_transport(&config.smtp)
            .expect("SMTP transport construction failed after config validation");
        let rate_limiter = Arc::new(rate_limit::RateLimiter::new(&config.rate_limit));
        Arc::new(Self {
            config: Arc::new(config),
            smtp,
            rate_limiter,
        })
    }
}
