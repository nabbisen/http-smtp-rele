//! Configuration loading and validation.
//!
//! Config is read from a TOML file at startup. Invalid configuration causes
//! immediate process termination (fail-fast).
//!
//! # Schema overview
//!
//! ```toml
//! [server]
//! bind_address = "127.0.0.1:8080"
//!
//! [security]
//! require_auth = true
//! trust_proxy_headers = false
//! trusted_source_cidrs = ["127.0.0.1/32"]
//!
//! [[security.api_keys]]
//! id = "svc-a"
//! secret = "tok-..."
//! enabled = true
//!
//! [mail]
//! default_from = "relay@example.com"
//!
//! [smtp]
//! host = "127.0.0.1"
//! port = 25
//!
//! [rate_limit]
//!
//! [logging]
//! ```

use std::fmt;
use std::path::Path;

use lettre::Address;
use serde::Deserialize;
use thiserror::Error;

// ---------------------------------------------------------------------------
// SecretString
// ---------------------------------------------------------------------------

/// An opaque string that is never printed in logs or debug output.
///
/// Used to store API key secrets from config. The underlying value is
/// accessible only via [`SecretString::expose`].
#[derive(Clone, Deserialize)]
#[serde(transparent)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Return the underlying secret value.
    ///
    /// Callers must not log, store, or transmit the returned value.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

// ---------------------------------------------------------------------------
// Config structs
// ---------------------------------------------------------------------------

/// Top-level application configuration.

/// Submission status tracking configuration (RFC 086, 087).
#[derive(Debug, Clone, Deserialize)]
pub struct StatusConfig {
    /// Enable status tracking. When false, no records are created.
    /// Requires restart to change.
    #[serde(default = "default_status_enabled")]
    pub enabled: bool,
    /// Backend: "memory" only in MVP. Requires restart to change.
    #[serde(default = "default_status_store")]
    pub store: String,
    /// Record time-to-live in seconds. SIGHUP-reloadable.
    #[serde(default = "default_status_ttl_seconds")]
    pub ttl_seconds: u64,
    /// Maximum records in store. SIGHUP-reloadable.
    #[serde(default = "default_status_max_records")]
    pub max_records: usize,
    /// Background cleanup interval in seconds. SIGHUP-reloadable.
    #[serde(default = "default_status_cleanup_interval_seconds")]
    pub cleanup_interval_seconds: u64,
    /// Path to the SQLite database file. Required when `store = "sqlite"`.
    /// The parent directory must exist; the file is created on first run.
    pub db_path: Option<std::path::PathBuf>,
    /// Redis/Valkey URL for the shared status store. Required when `store = "redis"`.
    /// Example: `redis://127.0.0.1:6379/0` or `redis+unix:///var/run/redis/redis.sock`.
    pub redis_url: Option<String>,
}

fn default_status_enabled() -> bool { true }
fn default_status_store() -> String { "memory".into() }
fn default_status_ttl_seconds() -> u64 { 3600 }
fn default_status_max_records() -> usize { 10_000 }
fn default_status_cleanup_interval_seconds() -> u64 { 60 }

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server:     ServerConfig,
    pub security:   SecurityConfig,
    pub mail:       MailConfig,
    pub smtp:       SmtpConfig,
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    #[serde(default)]
    pub logging:    LoggingConfig,
    #[serde(default)]
    pub status:     StatusConfig,
}

impl Default for StatusConfig {
    fn default() -> Self {
        Self {
            enabled: default_status_enabled(),
            store: default_status_store(),
            ttl_seconds: default_status_ttl_seconds(),
            max_records: default_status_max_records(),
            cleanup_interval_seconds: default_status_cleanup_interval_seconds(),
            db_path: None,
            redis_url: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub bind_address: String,
    #[serde(default = "default_max_request_body_bytes")]
    pub max_request_body_bytes: usize,
    #[serde(default = "default_request_timeout_seconds")]
    pub request_timeout_seconds: u64,
    #[serde(default = "default_shutdown_timeout_seconds")]
    pub shutdown_timeout_seconds: u64,
    /// Maximum concurrent in-flight requests. 0 = unlimited.
    #[serde(default)]
    pub concurrency_limit: usize,
    /// PEM certificate for HTTPS (RFC 712). Both cert and key must be set.
    pub tls_cert: Option<std::path::PathBuf>,
    /// PEM private key for HTTPS (RFC 712). Both cert and key must be set.
    pub tls_key:  Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    #[serde(default = "default_true")]
    pub require_auth: bool,
    /// When true, read `X-Forwarded-For` to resolve client IP.
    /// Only applies when the peer IP is listed in `trusted_source_cidrs`.
    #[serde(default)]
    pub trust_proxy_headers: bool,
    /// CIDRs whose X-Forwarded-For headers may be trusted for IP resolution.
    /// Distinct from `allowed_source_cidrs` — see security model.
    #[serde(default)]
    pub trusted_source_cidrs: Vec<String>,
    /// CIDRs that are permitted to connect at all (empty = allow all source IPs).
    /// Applied after IP resolution; independent of proxy header trust.
    #[serde(default)]
    pub allowed_source_cidrs: Vec<String>,
    #[serde(default)]
    pub api_keys: Vec<ApiKeyConfig>,
}

/// Per-API-key configuration entry.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiKeyConfig {
    pub id: String,
    pub secret: SecretString,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub description: Option<String>,
    /// Recipient domain allowlist for this key (empty = use global policy).
    #[serde(default)]
    pub allowed_recipient_domains: Vec<String>,
    /// Exact recipient address allowlist (empty = domain-level policy only).
    /// Takes precedence over `allowed_recipient_domains` when non-empty.
    #[serde(default)]
    pub allowed_recipients: Vec<String>,
    /// Per-key sustained rate (tokens/minute). None = inherit `[rate_limit].per_key_per_min`.
    pub rate_limit_per_min: Option<u32>,
    /// Per-key burst override. 0 = inherit `[rate_limit].per_key_burst`.
    #[serde(default)]
    pub burst: u32,
    /// Override global `[logging].mask_recipient` for this key (RFC 603).
    /// `None` = inherit global setting.
    pub mask_recipient: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MailConfig {
    pub default_from: String,
    pub default_from_name: Option<String>,
    /// Global recipient domain allowlist (empty = allow all domains).
    #[serde(default)]
    pub allowed_recipient_domains: Vec<String>,
    #[serde(default = "default_max_subject_chars")]
    pub max_subject_chars: usize,
    #[serde(default = "default_max_body_bytes")]
    pub max_body_bytes: usize,
    /// Maximum number of recipients per request. Default 10.
    #[serde(default = "default_max_recipients")]
    pub max_recipients: usize,
    /// Maximum number of attachments per request (RFC 502).
    #[serde(default = "default_max_attachments")]
    pub max_attachments: usize,
    /// Maximum decoded size per attachment in bytes (RFC 502). Default 10 MiB.
    #[serde(default = "default_max_attachment_bytes")]
    pub max_attachment_bytes: usize,
    /// Maximum number of messages per `POST /v1/send-bulk` request (RFC 701).
    #[serde(default = "default_max_bulk_messages")]
    pub max_bulk_messages: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SmtpConfig {
    #[serde(default = "default_smtp_mode")]
    pub mode: String,
    /// TLS mode: "none" (plain TCP), "starttls" (STARTTLS), or "tls" (implicit TLS).
    #[serde(default = "default_smtp_tls")]
    pub tls: String,
    #[serde(default = "default_smtp_host")]
    pub host: String,
    #[serde(default = "default_smtp_port")]
    pub port: u16,
    #[serde(default = "default_connect_timeout_seconds")]
    pub connect_timeout_seconds: u64,
    #[serde(default = "default_submission_timeout_seconds")]
    pub submission_timeout_seconds: u64,
    /// SMTP AUTH username. Must be set together with `auth_password` (RFC 301).
    pub auth_user: Option<String>,
    /// SMTP AUTH password. Never logged. Must be set together with `auth_user`.
    pub auth_password: Option<SecretString>,
    /// Command for pipe mode. Only used when `mode = "pipe"` (RFC 304).
    #[serde(default = "default_pipe_command")]
    pub pipe_command: String,
    /// Max concurrent SMTP submissions per bulk request (RFC 711).
    /// 0 = unlimited. Default: 5.
    #[serde(default = "default_bulk_concurrency")]
    pub bulk_concurrency: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    // Sustained rates (tokens/minute)
    #[serde(default = "default_global_per_min")]
    pub global_per_min: u32,
    #[serde(default = "default_per_ip_per_min")]
    pub per_ip_per_min: u32,
    /// Default per-key rate. Overridden by `ApiKeyConfig.rate_limit_per_min`.
    #[serde(default = "default_per_key_per_min")]
    pub per_key_per_min: u32,

    // Burst capacities (tokens a fresh bucket starts with)
    #[serde(default = "default_global_burst")]
    pub global_burst: u32,
    #[serde(default = "default_per_ip_burst")]
    pub per_ip_burst: u32,
    /// Default per-key burst. Overridden by `ApiKeyConfig.burst` when > 0.
    #[serde(default = "default_per_key_burst")]
    pub per_key_burst: u32,

    /// Legacy field — sets all three burst values if the per-tier fields are absent.
    /// Deprecated; use `global_burst`, `per_ip_burst`, `per_key_burst` instead.
    #[serde(default)]
    pub burst_size: u32,

    /// Maximum entries in the per-IP bucket map; LRU eviction above this.
    /// 0 = unlimited (not recommended in production).
    #[serde(default = "default_ip_table_size")]
    pub ip_table_size: usize,
}

impl RateLimitConfig {
    /// Effective global burst: per-tier value if set, else legacy `burst_size`, else default.
    pub fn effective_global_burst(&self) -> u32 {
        if self.global_burst > 0 { self.global_burst }
        else if self.burst_size > 0 { self.burst_size }
        else { default_global_burst() }
    }
    pub fn effective_per_ip_burst(&self) -> u32 {
        if self.per_ip_burst > 0 { self.per_ip_burst }
        else if self.burst_size > 0 { self.burst_size }
        else { default_per_ip_burst() }
    }
    pub fn effective_per_key_burst(&self) -> u32 {
        if self.per_key_burst > 0 { self.per_key_burst }
        else if self.burst_size > 0 { self.burst_size }
        else { default_per_key_burst() }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    /// Output format: `"text"` (default) or `"json"`.
    #[serde(default = "default_log_format")]
    pub format: String,
    #[serde(default = "default_log_level")]
    pub level: String,
    /// When true, mask the recipient address in audit log entries.
    #[serde(default)]
    pub mask_recipient: bool,
}

// ---------------------------------------------------------------------------
// Default value functions
// ---------------------------------------------------------------------------

fn default_max_request_body_bytes() -> usize { 1_048_576 }
fn default_request_timeout_seconds() -> u64 { 30 }
fn default_shutdown_timeout_seconds() -> u64 { 30 }
fn default_true() -> bool { true }
fn default_max_subject_chars() -> usize { 255 }
fn default_max_body_bytes() -> usize { 65_536 }
fn default_smtp_mode() -> String { "smtp".into() }
fn default_smtp_host() -> String { "127.0.0.1".into() }
fn default_smtp_port() -> u16 { 25 }
fn default_connect_timeout_seconds() -> u64 { 5 }
fn default_submission_timeout_seconds() -> u64 { 30 }
fn default_bulk_concurrency() -> usize { 5 }
fn default_global_per_min() -> u32 { 60 }
fn default_per_ip_per_min() -> u32 { 20 }
#[allow(dead_code)]
fn default_burst_size() -> u32 { 5 }
fn default_max_recipients() -> usize { 10 }
fn default_max_attachments() -> usize { 5 }
fn default_max_attachment_bytes() -> usize { 10 * 1024 * 1024 } // 10 MiB
fn default_pipe_command() -> String { "/usr/sbin/sendmail".into() }
fn default_smtp_tls() -> String { "none".into() }
fn default_global_burst() -> u32 { 10 }
fn default_per_ip_burst() -> u32 { 5 }
fn default_per_key_burst() -> u32 { 5 }
fn default_per_key_per_min() -> u32 { 30 }
fn default_ip_table_size() -> usize { 10_000 }
fn default_log_format() -> String { "text".into() }
fn default_log_level() -> String { "info".into() }

// ---------------------------------------------------------------------------
// Config error
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("cannot read config file: {0}")]
    Io(#[from] std::io::Error),

    #[error("config parse error: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("invalid server.bind_address: must be host:port (e.g. 127.0.0.1:8080)")]
    InvalidBindAddress,

    #[error("invalid mail.default_from: must be a valid email address")]
    InvalidDefaultFrom,

    #[error("security.require_auth is true but no api_keys are defined")]
    NoApiKeys,

    #[error("no api_keys entries have enabled = true")]
    NoEnabledApiKeys,

    #[error("invalid CIDR: {0}")]
    InvalidCidr(String),

    #[error("configuration error: {0}")]
    Validation(String),

    #[error("invalid smtp.port: must be 1-65535")]
    InvalidSmtpPort,

    #[error("invalid rate_limit values: all per_min values must be > 0")]
    InvalidRateLimit,

    #[error("invalid logging.level: must be trace, debug, info, warn, or error")]
    InvalidLogLevel,

    #[error("invalid logging.format: must be 'text' or 'json'")]
    InvalidLogFormat,
}

// ---------------------------------------------------------------------------
// Default impls (needed for #[serde(default)] on AppConfig fields)
// ---------------------------------------------------------------------------

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            global_per_min:    default_global_per_min(),
            per_ip_per_min:    default_per_ip_per_min(),
            per_key_per_min:   default_per_key_per_min(),
            global_burst:      default_global_burst(),
            per_ip_burst:      default_per_ip_burst(),
            per_key_burst:     default_per_key_burst(),
            ip_table_size:     default_ip_table_size(),
            burst_size:        0,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            format:         default_log_format(),
            level:          default_log_level(),
            mask_recipient: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Load and validate
// ---------------------------------------------------------------------------

pub fn load_from_str(toml_str: &str) -> Result<AppConfig, ConfigError> {
    let config: AppConfig = toml::from_str(toml_str)?;
    validate_config(&config)?;
    Ok(config)
}

pub fn load(path: &Path) -> Result<AppConfig, ConfigError> {
    let text = std::fs::read_to_string(path)?;
    let config: AppConfig = toml::from_str(&text)?;
    validate_config(&config)?;
    Ok(config)
}

pub fn validate_config(config: &AppConfig) -> Result<(), ConfigError> {
    // bind_address
    config
        .server
        .bind_address
        .parse::<std::net::SocketAddr>()
        .map_err(|_| ConfigError::InvalidBindAddress)?;

    // default_from
    config
        .mail
        .default_from
        .parse::<Address>()
        .map_err(|_| ConfigError::InvalidDefaultFrom)?;

    // API keys
    if config.security.require_auth && config.security.api_keys.is_empty() {
        return Err(ConfigError::NoApiKeys);
    }
    if config.security.require_auth
        && !config.security.api_keys.iter().any(|k| k.enabled)
    {
        return Err(ConfigError::NoEnabledApiKeys);
    }

    // CIDRs — validate both lists
    for cidr in config.security.trusted_source_cidrs.iter()
        .chain(config.security.allowed_source_cidrs.iter())
    {
        cidr.parse::<ipnet::IpNet>()
            .map_err(|_| ConfigError::InvalidCidr(cidr.clone()))?;
    }

    // SMTP port
    if config.smtp.port == 0 {
        return Err(ConfigError::InvalidSmtpPort);
    }

    // SMTP TLS mode
    match config.smtp.tls.as_str() {
        "none" | "starttls" | "tls" => {}
        other => return Err(ConfigError::Validation(
            format!("smtp.tls must be \"none\", \"starttls\", or \"tls\"; got \"{other}\"")
        )),
    }

    // SMTP AUTH: both user and password must be set or both absent
    match (&config.smtp.auth_user, &config.smtp.auth_password) {
        (Some(_), None) | (None, Some(_)) => {
            return Err(ConfigError::Validation(
                "smtp.auth_user and smtp.auth_password must both be set or both absent".into(),
            ));
        }
        _ => {}
    }

    // TLS: cert and key must both be set or both absent (RFC 712)
    match (&config.server.tls_cert, &config.server.tls_key) {
        (Some(_), None) | (None, Some(_)) => {
            return Err(ConfigError::Validation(
                "server.tls_cert and server.tls_key must both be set or both be absent".into()
            ));
        }
        (Some(_), Some(_)) => {
            #[cfg(not(feature = "tls"))]
            return Err(ConfigError::Validation(
                "server.tls_cert/tls_key is configured but TLS is not available in this build.                  Rebuild with: cargo build --features tls".into()
            ));
        }
        (None, None) => {}
    }

    // Status store validation (RFC 087, 088, 722)
    if config.status.store == "sqlite" {
        if config.status.db_path.is_none() {
            return Err(ConfigError::Validation(
                "status.db_path is required when status.store = \"sqlite\"".into()
            ));
        }
        #[cfg(not(feature = "sqlite"))]
        return Err(ConfigError::Validation(
            "status.store = \"sqlite\" is not available in this build.              Rebuild with: cargo build --features sqlite".into()
        ));
    } else if config.status.store == "redis" {
        if config.status.redis_url.is_none() {
            return Err(ConfigError::Validation(
                "status.redis_url is required when status.store = \"redis\"".into()
            ));
        }
        #[cfg(not(feature = "redis"))]
        return Err(ConfigError::Validation(
            "status.store = \"redis\" is not available in this build.              Rebuild with: cargo build --features redis".into()
        ));
    } else if !matches!(config.status.store.as_str(), "memory") {
        return Err(ConfigError::Validation(
            format!("status.store must be \"memory\", \"sqlite\", or \"redis\"; got \"{}\""
                , config.status.store)
        ));
    }

    // Pipe mode: auth credentials are not applicable
    if config.smtp.mode == "pipe"
        && (config.smtp.auth_user.is_some() || config.smtp.auth_password.is_some())
    {
        return Err(ConfigError::Validation(
            r#"smtp.auth_user/auth_password are not applicable when smtp.mode = "pipe""#.into(),
        ));
    }

    // Rate limits
    if config.rate_limit.global_per_min == 0 || config.rate_limit.per_ip_per_min == 0 {
        return Err(ConfigError::InvalidRateLimit);
    }

    // Log level
    let valid_levels = ["trace", "debug", "info", "warn", "error"];
    if !valid_levels.contains(&config.logging.level.as_str()) {
        return Err(ConfigError::InvalidLogLevel);
    }

    // Log format
    let valid_formats = ["text", "json"];
    if !valid_formats.contains(&config.logging.format.as_str()) {
        return Err(ConfigError::InvalidLogFormat);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_config_str() -> String {
        r#"
[server]
bind_address = "127.0.0.1:8080"

[security]
require_auth = false

[rate_limit]

[mail]
default_from = "noreply@example.com"

[smtp]

[logging]
"#
        .into()
    }

    #[test]
    fn valid_config_parses() {
        let config: AppConfig = toml::from_str(&minimal_config_str()).unwrap();
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn invalid_bind_address() {
        let text = minimal_config_str().replace("127.0.0.1:8080", "notanaddress");
        let config: AppConfig = toml::from_str(&text).unwrap();
        assert!(matches!(validate_config(&config), Err(ConfigError::InvalidBindAddress)));
    }

    #[test]
    fn invalid_default_from() {
        let text = minimal_config_str().replace("noreply@example.com", "notanemail");
        let config: AppConfig = toml::from_str(&text).unwrap();
        assert!(matches!(validate_config(&config), Err(ConfigError::InvalidDefaultFrom)));
    }

    #[test]
    fn require_auth_no_keys() {
        let text = minimal_config_str().replace("require_auth = false", "require_auth = true");
        let config: AppConfig = toml::from_str(&text).unwrap();
        assert!(matches!(validate_config(&config), Err(ConfigError::NoApiKeys)));
    }

    #[test]
    fn secret_string_is_redacted_in_debug() {
        let s = SecretString::new("very-secret");
        assert!(!format!("{:?}", s).contains("very-secret"));
        assert!(!format!("{}", s).contains("very-secret"));
        assert_eq!(s.expose(), "very-secret");
    }

    #[test]
    fn defaults_are_sensible() {
        let config: AppConfig = toml::from_str(&minimal_config_str()).unwrap();
        assert_eq!(config.server.max_request_body_bytes, 1_048_576);
        assert_eq!(config.smtp.port, 25);
        assert_eq!(config.rate_limit.global_per_min, 60);
        assert_eq!(config.logging.format, "text");
    }
}

fn default_max_bulk_messages() -> usize { 10 }
