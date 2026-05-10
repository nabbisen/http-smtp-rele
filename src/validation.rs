//! Request validation pipeline.
//!
//! Implements RFC 050: strict DTO deserialization, field validation,
//! header-injection prevention, and recipient domain policy.
//!
//! # Flow
//!
//! ```text
//! MailRequest (raw JSON DTO)
//!   -> validate_mail_request()
//!   -> ValidatedMailRequest (type-level proof of safety)
//! ```
//!
//! # Policy
//!
//! - All fields entering mail headers are checked for CR/LF and control chars.
//! - Unknown JSON fields are rejected at deserialization time (`deny_unknown_fields`).
//! - `from`, `cc`, `bcc`, `headers` are explicitly absent from the DTO.
//! - Reject; never silently rewrite.

use serde::Deserialize;

use crate::{
    auth::AuthContext,
    policy,
    sanitize,
    config::AppConfig,
    error::AppError,
    sanitize::{contains_control_chars, contains_header_injection},
};

// ---------------------------------------------------------------------------
// Public request DTO
// ---------------------------------------------------------------------------

/// One or more recipient addresses.
///
/// Accepts a single string (`"alice@example.com"`) or an array
/// (`["alice@example.com", "bob@example.com"]`) from JSON.
#[derive(Debug, Clone)]
pub struct Recipients(pub Vec<String>);

impl<'de> serde::Deserialize<'de> for Recipients {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum OneOrMany {
            One(String),
            Many(Vec<String>),
        }
        match OneOrMany::deserialize(de)? {
            OneOrMany::One(s)  => Ok(Recipients(vec![s])),
            OneOrMany::Many(v) => Ok(Recipients(v)),
        }
    }
}

/// A single attachment as supplied in the JSON request body (RFC 502).
#[derive(Debug, Deserialize)]
pub struct AttachmentSpec {
    /// Display filename (e.g. `report.pdf`). No path separators allowed.
    pub filename: String,
    /// MIME content-type (e.g. `application/pdf`).
    pub content_type: String,
    /// Base64-encoded file content.
    pub data: String,
}

/// A validated, decoded attachment ready for inclusion in the mail message.
#[derive(Debug, Clone)]
pub struct ValidatedAttachment {
    pub filename: String,
    pub content_type: String,
    pub decoded: Vec<u8>,
}


/// Raw mail request as received from the HTTP client.
///
/// `deny_unknown_fields` ensures that any field not listed here (e.g., `from`,
/// `cc`, `bcc`, `headers`) causes immediate deserialization failure → 422.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MailRequest {
    pub to: Recipients,
    pub subject: String,
    pub body: String,
    pub from_name: Option<String>,
    /// Reply-To address(es): string or array (RFC 503).
    pub reply_to: Option<Recipients>,
    /// Optional HTML body. Combined with `body` to create multipart/alternative (RFC 403).
    pub body_html: Option<String>,
    /// Optional CC recipients (string or array, RFC 404).
    pub cc: Option<Recipients>,
    /// File attachments (RFC 502). Each entry is base64-encoded.
    pub attachments: Option<Vec<AttachmentSpec>>,
    /// Opaque client metadata (logged for correlation; not reflected in mail).
    pub metadata: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Validated DTO
// ---------------------------------------------------------------------------

/// Type-safe proof that a `MailRequest` has passed all validation checks.
///
/// Only `validate_mail_request()` can construct this type.
/// Downstream modules (`mail`, `smtp`) accept only this type.
#[derive(Debug)]
pub struct ValidatedMailRequest {
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,
    pub from_name: Option<String>,
    pub reply_to: Vec<String>,
    pub body_html: Option<String>,
    pub cc: Vec<String>,
    pub attachments: Vec<ValidatedAttachment>,
    pub client_request_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Validation entry point
// ---------------------------------------------------------------------------

/// Validate a raw `MailRequest` against config policy and auth context.
///
/// Returns `ValidatedMailRequest` on success, or an `AppError::Validation`
/// describing the first failure encountered.
///
/// # Validation order
///
/// 1. `to` — format + domain policy
/// 2. `subject` — empty, length, header-injection
/// 3. `body` — NUL, length
/// 4. `from_name` — length, header-injection (optional)
/// 5. `reply_to` — format, header-injection (optional)
/// 6. `metadata` — extract client request_id
pub fn validate_mail_request(
    req: MailRequest,
    config: &AppConfig,
    auth: &AuthContext,
) -> Result<ValidatedMailRequest, AppError> {
    let mail_cfg = &config.mail;

    // 1. `to` — validate each recipient (RFC 302)
    {
        let recipients = &req.to.0;
        if recipients.is_empty() {
            return Err(AppError::Validation("to: at least one recipient is required".into()));
        }
        if recipients.len() > config.mail.max_recipients {
            return Err(AppError::Validation(format!(
                "to: too many recipients (max {})",
                config.mail.max_recipients
            )));
        }
        for addr in recipients {
            validate_email_address(addr, "to")?;
            sanitize::reject_header_crlf("to", addr)?;
            check_recipient_domain_or_address(addr, config, auth)?;
        }
    }
    let to = req.to.0;

    // 1b. `cc` — validate each CC recipient (RFC 404)
    let cc: Vec<String> = if let Some(cc_recipients) = req.cc {
        let cc_addrs = cc_recipients.0;
        // Combined to + cc must not exceed max_recipients
        let total = to.len() + cc_addrs.len();
        if total > config.mail.max_recipients {
            return Err(AppError::Validation(format!(
                "to + cc: too many recipients (max {})",
                config.mail.max_recipients
            )));
        }
        for addr in &cc_addrs {
            validate_email_address(addr, "cc")?;
            sanitize::reject_header_crlf("cc", addr)?;
            check_recipient_domain_or_address(addr, config, auth)?;
        }
        cc_addrs
    } else {
        vec![]
    };

    // 2. `subject`
    let subject = validate_subject(&req.subject, mail_cfg.max_subject_chars)?;

    // 3. `body`
    let body = validate_body(&req.body, mail_cfg.max_body_bytes)?;

    // 3b. `body_html` — size and NUL check (RFC 403)
    if let Some(ref html) = req.body_html {
        if html.contains('\0') {
            return Err(AppError::Validation("body_html: contains NUL character".into()));
        }
        if html.len() > mail_cfg.max_body_bytes {
            return Err(AppError::Validation(format!(
                "body_html: exceeds maximum of {} bytes",
                mail_cfg.max_body_bytes
            )));
        }
    }

    // 4. `from_name` (optional)
    let from_name = req
        .from_name
        .as_deref()
        .map(|n| validate_display_name(n, "from_name"))
        .transpose()?;

    // 5. `reply_to` (optional, string or array — RFC 503)
    let reply_to: Vec<String> = if let Some(recipients) = req.reply_to {
        let addrs = recipients.0;
        for addr in &addrs {
            validate_email_address(addr, "reply_to")?;
            sanitize::reject_header_crlf("reply_to", addr)?;
        }
        addrs
    } else {
        vec![]
    };

    // 6. client request_id from metadata
    let client_request_id = req
        .metadata
        .as_ref()
        .and_then(|m| m.get("request_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // 7. Attachments (RFC 502)
    let attachments: Vec<ValidatedAttachment> = {
        use base64::Engine as _;
        let specs = req.attachments.unwrap_or_default();
        if specs.len() > mail_cfg.max_attachments {
            return Err(AppError::Validation(format!(
                "attachments: too many (max {})", mail_cfg.max_attachments
            )));
        }
        let mut validated = Vec::with_capacity(specs.len());
        for spec in specs {
            // Filename safety
            if spec.filename.is_empty() || spec.filename.len() > 255 {
                return Err(AppError::Validation("attachments[].filename: must be 1–255 chars".into()));
            }
            if spec.filename.contains('/') || spec.filename.contains('\\') || spec.filename.contains(' ') {
                return Err(AppError::Validation("attachments[].filename: path separators not allowed".into()));
            }
            sanitize::reject_header_crlf("attachments[].filename", &spec.filename)?;

            // Content-type: basic non-empty check
            if spec.content_type.is_empty() || !spec.content_type.contains('/') {
                return Err(AppError::Validation("attachments[].content_type: must be a valid MIME type".into()));
            }

            // Decode base64
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(&spec.data)
                .map_err(|_| AppError::Validation("attachments[].data: invalid base64".into()))?;

            // Size check
            if decoded.len() > mail_cfg.max_attachment_bytes {
                return Err(AppError::Validation(format!(
                    "attachments[].data: decoded size {} exceeds maximum {}",
                    decoded.len(), mail_cfg.max_attachment_bytes
                )));
            }

            validated.push(ValidatedAttachment {
                filename: spec.filename,
                content_type: spec.content_type,
                decoded,
            });
        }
        validated
    };

    Ok(ValidatedMailRequest {
        to,
        subject,
        body,
        body_html: req.body_html,
        from_name,
        reply_to,
        cc,
        attachments,
        client_request_id,
    })
}

// ---------------------------------------------------------------------------
// Field validators
// ---------------------------------------------------------------------------

/// Validate an email address string using `lettre`'s address parser.
fn validate_email_address(raw: &str, field: &str) -> Result<String, AppError> {
    use lettre::address::Address;
    if contains_header_injection(raw) {
        return Err(AppError::Validation(format!(
            "field `{field}` contains illegal line break"
        )));
    }
    raw.parse::<Address>()
        .map(|a| a.to_string())
        .map_err(|_| AppError::Validation(format!("field `{field}` is not a valid email address")))
}

fn validate_subject(raw: &str, max_len: usize) -> Result<String, AppError> {
    if raw.trim().is_empty() {
        return Err(AppError::Validation(
            "field `subject` must not be empty".into(),
        ));
    }
    if contains_header_injection(raw) {
        return Err(AppError::Validation(
            "field `subject` contains illegal line break".into(),
        ));
    }
    if contains_control_chars(raw) {
        return Err(AppError::Validation(
            "field `subject` contains illegal control character".into(),
        ));
    }
    if raw.chars().count() > max_len {
        return Err(AppError::Validation(format!(
            "field `subject` exceeds maximum length of {max_len} characters"
        )));
    }
    Ok(raw.to_string())
}

fn validate_body(raw: &str, max_len: usize) -> Result<String, AppError> {
    if raw.contains('\0') {
        return Err(AppError::Validation(
            "field `body` contains NUL byte".into(),
        ));
    }
    if raw.len() > max_len {
        return Err(AppError::Validation(format!(
            "field `body` exceeds maximum size of {max_len} bytes"
        )));
    }
    Ok(raw.to_string())
}

fn validate_display_name(raw: &str, field: &str) -> Result<String, AppError> {
    if contains_header_injection(raw) {
        return Err(AppError::Validation(format!(
            "field `{field}` contains illegal line break"
        )));
    }
    Ok(raw.to_string())
}

// ---------------------------------------------------------------------------
// Recipient domain policy
// ---------------------------------------------------------------------------

/// Check that the `to` address domain is permitted by both the global config
/// and the API key's per-key policy.
///
/// If the global `allowed_recipient_domains` list is empty, all domains are
/// permitted at the global level (per-key policy still applies if set).
/// Check recipient against both per-address allowlist (RFC 204) and domain allowlist.
fn check_recipient_domain_or_address(
    addr: &str,
    config: &AppConfig,
    auth: &AuthContext,
) -> Result<(), AppError> {
    // Per-address allowlist (key-level, RFC 204)
    if let Some(key_cfg) = config.security.api_keys.iter().find(|k| k.id == auth.key_id) {
        if !policy::address_permitted_for_key(key_cfg, addr) {
            return Err(AppError::Validation(
                "to: recipient is not permitted for this API key".into(),
            ));
        }
    }
    // Global domain policy
    check_recipient_domain(addr, config, auth)
}

fn check_recipient_domain(
    to: &str,
    config: &AppConfig,
    auth: &AuthContext,
) -> Result<(), AppError> {
    let domain = extract_domain(to)?;

    // Global allowlist (empty = allow all)
    if !config.mail.allowed_recipient_domains.is_empty()
        && !config
            .mail
            .allowed_recipient_domains
            .iter()
            .any(|d| d.eq_ignore_ascii_case(&domain))
    {
        return Err(AppError::Validation(format!(
            "recipient domain `{domain}` is not permitted"
        )));
    }

    // Per-key allowlist (empty = no additional restriction)
    let key_cfg = config
        .security
        .api_keys
        .iter()
        .find(|k| k.id == auth.key_id);
    if let Some(key) = key_cfg {
        if !key.allowed_recipient_domains.is_empty()
            && !key
                .allowed_recipient_domains
                .iter()
                .any(|d| d.eq_ignore_ascii_case(&domain))
        {
            return Err(AppError::Validation(format!(
                "recipient domain `{domain}` is not permitted for this API key"
            )));
        }
    }

    Ok(())
}

fn extract_domain(email: &str) -> Result<String, AppError> {
    email
        .rsplit_once('@')
        .map(|(_, d)| d.to_lowercase())
        .ok_or_else(|| AppError::Validation("could not extract domain from email address".into()))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        auth::AuthContext,
        config::{
            ApiKeyConfig, AppConfig, LoggingConfig, MailConfig, RateLimitConfig, SecretString,
            SecurityConfig, ServerConfig, SmtpConfig,
        },
    };
    use std::net::IpAddr;

    fn make_auth(key_id: &str) -> AuthContext {
        AuthContext {
            key_id: key_id.to_string(),
            client_ip: IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            key_rate_limit_per_min: None,
            key_burst: 0,
        }
    }

    fn minimal_config() -> AppConfig {
        AppConfig {
            server: ServerConfig {
                bind_address: "127.0.0.1:8080".into(),
                max_request_body_bytes: 65536,
                request_timeout_seconds: 30,
                shutdown_timeout_seconds: 30,
                concurrency_limit: 0,
                tls_cert: None,
                tls_key: None,
            },
            security: SecurityConfig {
                require_auth: true,
                trust_proxy_headers: false,
                trusted_source_cidrs: vec![],
                    allowed_source_cidrs: vec![],
                api_keys: vec![ApiKeyConfig {
                    id: "test-key".into(),
                    secret: SecretString::new("tok"),
                    enabled: true,
                    description: None,
                    allowed_recipient_domains: vec![],
                    allowed_recipients: vec![],
                    rate_limit_per_min: None,
                    burst: 0,
                    mask_recipient: None,
                }],
            },
            mail: MailConfig {
                default_from: "relay@example.com".into(),
                default_from_name: None,
                allowed_recipient_domains: vec![],
                max_subject_chars: 200,
                max_body_bytes: 1_000_000,
                max_recipients: 10,
                max_attachments: 5,
                max_attachment_bytes: 10 * 1024 * 1024,
            max_bulk_messages: 10,
            },
            smtp: SmtpConfig {
                mode: "smtp".into(),
                host: "127.0.0.1".into(),
                port: 25,
                connect_timeout_seconds: 5,
                submission_timeout_seconds: 30,
                auth_user: None,
                auth_password: None,
                pipe_command: "/usr/sbin/sendmail".into(),
                tls: "none".into(),
                bulk_concurrency: 5,
            },
            rate_limit: RateLimitConfig {
                global_per_min: 60,
                per_ip_per_min: 20,
                per_key_per_min: 30,
                global_burst: 5,
                per_ip_burst: 5,
                per_key_burst: 5,
                burst_size: 0,
                ip_table_size: 100,
            },
            logging: LoggingConfig {
                format: "text".into(),
                level: "info".into(),
                mask_recipient: false,
            },
            status: Default::default(),
        }
    }

    fn minimal_request() -> MailRequest {
        MailRequest {
            to: crate::validation::Recipients(vec!["user@example.com".into()]),
            subject: "Hello".into(),
            body: "Test body".into(),
            from_name: None,
            reply_to: None,
            body_html: None,
            cc: None,
            attachments: None,
            metadata: None,
        }
    }

    #[test]
    fn valid_request_passes() {
        let cfg = minimal_config();
        let auth = make_auth("test-key");
        let req = minimal_request();
        assert!(validate_mail_request(req, &cfg, &auth).is_ok());
    }

    #[test]
    fn invalid_email_rejected() {
        let cfg = minimal_config();
        let auth = make_auth("test-key");
        let req = MailRequest {
            to: crate::validation::Recipients(vec!["not-an-email".into()]),
            ..minimal_request()
        };
        assert!(matches!(
            validate_mail_request(req, &cfg, &auth),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn crlf_in_subject_rejected() {
        let cfg = minimal_config();
        let auth = make_auth("test-key");
        let req = MailRequest {
            subject: "Hello\r\nBcc: evil@x.com".into(),
            ..minimal_request()
        };
        assert!(matches!(
            validate_mail_request(req, &cfg, &auth),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn empty_subject_rejected() {
        let cfg = minimal_config();
        let auth = make_auth("test-key");
        let req = MailRequest {
            subject: "   ".into(),
            ..minimal_request()
        };
        assert!(matches!(
            validate_mail_request(req, &cfg, &auth),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn oversized_subject_rejected() {
        let cfg = minimal_config();
        let auth = make_auth("test-key");
        let req = MailRequest {
            subject: "a".repeat(201),
            ..minimal_request()
        };
        assert!(matches!(
            validate_mail_request(req, &cfg, &auth),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn nul_in_body_rejected() {
        let cfg = minimal_config();
        let auth = make_auth("test-key");
        let req = MailRequest {
            body: "Hello\0world".into(),
            ..minimal_request()
        };
        assert!(matches!(
            validate_mail_request(req, &cfg, &auth),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn crlf_in_from_name_rejected() {
        let cfg = minimal_config();
        let auth = make_auth("test-key");
        let req = MailRequest {
            from_name: Some("Evil\r\nBcc: attacker@evil.com".into()),
            ..minimal_request()
        };
        assert!(matches!(
            validate_mail_request(req, &cfg, &auth),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn disallowed_domain_rejected() {
        let mut cfg = minimal_config();
        cfg.mail.allowed_recipient_domains = vec!["allowed.com".into()];
        let auth = make_auth("test-key");
        let req = MailRequest {
            to: crate::validation::Recipients(vec!["user@other.com".into()]),
            ..minimal_request()
        };
        assert!(matches!(
            validate_mail_request(req, &cfg, &auth),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn allowed_domain_passes() {
        let mut cfg = minimal_config();
        cfg.mail.allowed_recipient_domains = vec!["example.com".into()];
        let auth = make_auth("test-key");
        let req = minimal_request(); // to = user@example.com
        assert!(validate_mail_request(req, &cfg, &auth).is_ok());
    }

    #[test]
    fn per_key_domain_restriction_works() {
        let mut cfg = minimal_config();
        cfg.security.api_keys[0].allowed_recipient_domains = vec!["allowed.com".into()];
        let auth = make_auth("test-key");
        let req = minimal_request(); // to = user@example.com (not allowed)
        assert!(matches!(
            validate_mail_request(req, &cfg, &auth),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn metadata_client_request_id_extracted() {
        let cfg = minimal_config();
        let auth = make_auth("test-key");
        let req = MailRequest {
            metadata: Some(serde_json::json!({"request_id": "client-123"})),
            ..minimal_request()
        };
        let v = validate_mail_request(req, &cfg, &auth).unwrap();
        assert_eq!(v.client_request_id.as_deref(), Some("client-123"));
    }

    /// SEC-006: CR/LF in `reply_to` is rejected before SMTP.
    #[test]
    fn crlf_in_reply_to_rejected() {
        let cfg = minimal_config();
        let auth = make_auth("test-key");
        for bad in &[
            "user@example.com
Bcc: evil@evil.com",
            "user@example.com
X-Header: injected",
        ] {
            let req = MailRequest {
                reply_to: Some(crate::validation::Recipients(vec![bad.to_string()])),
                ..minimal_request()
            };
            assert!(
                matches!(validate_mail_request(req, &cfg, &auth), Err(AppError::Validation(_))),
                "expected Validation error for reply_to={bad:?}"
            );
        }
    }

    /// SEC-007: CR/LF in `to` is rejected before SMTP.
    #[test]
    fn crlf_in_to_rejected() {
        let cfg = minimal_config();
        let auth = make_auth("test-key");
        let req = MailRequest {
            to: crate::validation::Recipients(vec!["user@example.com\nBcc: attacker@evil.com".to_string()]),
            ..minimal_request()
        };
        assert!(matches!(
            validate_mail_request(req, &cfg, &auth),
            Err(AppError::Validation(_))
        ));
    }

    /// SEC-017 (unit): SecretString never exposes its value through Debug.
    #[test]
    fn secret_string_redacted_in_debug() {
        use crate::config::SecretString;
        let s = SecretString::new("super-secret-token-value");
        let debug = format!("{s:?}");
        assert!(
            !debug.contains("super-secret-token-value"),
            "SecretString Debug must not expose secret; got: {debug}"
        );
    }
}
