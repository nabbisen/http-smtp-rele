//! Configuration validation logic.
//!
//! Separated from type definitions to keep `config.rs` focused on the schema.

use super::*;

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

    // Status store validation — only when status tracking is enabled (RFC 816)
    if config.status.enabled {
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
    } // status.enabled guard (RFC 816)

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

    // RFC 824: API key secret quality enforcement
    const MIN_SECRET_LEN: usize = 32;
    const BLOCKED: &[&str] = &[
        "your-secret-here", "generate-with-openssl-rand-base64-32",
        "changeme", "secret", "password", "example-secret", "replace-me",
    ];
    for key in &config.security.api_keys {
        let s = key.secret.expose();
        if s.len() < MIN_SECRET_LEN {
            return Err(ConfigError::Validation(format!(
                "api_keys[{}].secret: minimum {} bytes required (got {}).                  Generate with: openssl rand -base64 32",
                key.id, MIN_SECRET_LEN, s.len()
            )));
        }
        if BLOCKED.iter().any(|b| s.contains(b)) {
            return Err(ConfigError::Validation(format!(
                "api_keys[{}].secret: placeholder value detected.                  Replace with: openssl rand -base64 32", key.id
            )));
        }
    }

    // Ensure at least one key is configured and enabled.
    if config.security.api_keys.is_empty() {
        return Err(ConfigError::NoApiKeys);
    }
    if !config.security.api_keys.iter().any(|k| k.enabled) {
        return Err(ConfigError::NoEnabledApiKeys);
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
        // Secret is exactly 32+ bytes: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" (32 'a's)
        r#"
[server]
bind_address = "127.0.0.1:8080"

[[security.api_keys]]
id      = "test"
secret  = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
enabled = true

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

