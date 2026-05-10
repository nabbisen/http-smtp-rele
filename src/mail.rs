//! Safe plain text mail construction.
//!
//! Implements RFC 060–061: constructs a `lettre::Message` from a
//! `ValidatedMailRequest`. Raw string concatenation is never used.
//!
//! # From address policy (RFC 061)
//!
//! `From` is always taken from `config.mail.default_from`.
//! The request cannot override it.
//! Display name comes from the request's `from_name`, falling back to
//! `config.mail.default_from_name`.

use lettre::{
    message::{Attachment, header::ContentType, Mailbox, Message, MultiPart, SinglePart},
};
use tracing::error;

use crate::{config::AppConfig, error::AppError, validation::ValidatedMailRequest};

/// Build a `lettre::Message` from a validated request and server config.
///
/// # From policy
///
/// The `From` address is always `config.mail.default_from`.
/// The display name is taken from `validated.from_name` if present,
/// otherwise from `config.mail.default_from_name`.
pub fn build_message(validated: &ValidatedMailRequest, config: &AppConfig) -> Result<Message, AppError> {
    let mail_cfg = &config.mail;

    // From address — always from config.
    let from_addr = mail_cfg
        .default_from
        .parse::<lettre::Address>()
        .map_err(|e| {
            error!(error = %e, "invalid default_from config");
            AppError::Internal
        })?;

    // Display name: request > config default.
    let from_name = validated
        .from_name
        .as_deref()
        .or(mail_cfg.default_from_name.as_deref());

    let from_mailbox = match from_name {
        Some(name) => Mailbox::new(Some(name.to_string()), from_addr),
        None => Mailbox::new(None, from_addr),
    };

    // To addresses — one or more (RFC 302).
    let mut builder = Message::builder().from(from_mailbox);
    for addr in &validated.to {
        let to_mailbox: Mailbox = addr.parse().map_err(|e| {
            error!(error = %e, addr = %addr, "invalid to address after validation");
            AppError::Internal
        })?;
        builder = builder.to(to_mailbox);
    }
    let mut builder = builder
        .subject(validated.subject.clone())
        .header(ContentType::TEXT_PLAIN);

    // CC addresses (RFC 404).
    for addr in &validated.cc {
        let cc_mailbox: Mailbox = addr.parse().map_err(|e| {
            error!(error = %e, addr = %addr, "invalid cc address after validation");
            AppError::Internal
        })?;
        builder = builder.cc(cc_mailbox);
    }

    // Reply-To addresses (optional, string or array — RFC 503).
    for reply_to_addr in &validated.reply_to {
        let rt_mailbox: Mailbox = reply_to_addr
            .parse()
            .map_err(|e| {
                error!(error = %e, addr = %reply_to_addr, "invalid reply_to after validation");
                AppError::Internal
            })?;
        builder = builder.reply_to(rt_mailbox);
    }

    // Body part: plain text only, or multipart/alternative if HTML present (RFC 403).
    let body_part = if let Some(ref html) = validated.body_html {
        MultiPart::alternative()
            .singlepart(SinglePart::plain(validated.body.clone()))
            .singlepart(SinglePart::html(html.clone()))
    } else {
        MultiPart::alternative()
            .singlepart(SinglePart::plain(validated.body.clone()))
    };

    // Compose final message (RFC 502: wrap in multipart/mixed when attachments present).
    let message = if validated.attachments.is_empty() && validated.body_html.is_none() {
        // Simplest case: plain text only
        builder
            .body(validated.body.clone())
            .map_err(|e| { error!(error = %e, "failed to build plain message"); AppError::Internal })?
    } else if validated.attachments.is_empty() {
        // No attachments, but has HTML → multipart/alternative
        builder
            .multipart(body_part)
            .map_err(|e| { error!(error = %e, "failed to build alt message"); AppError::Internal })?
    } else {
        // Attachments present → multipart/mixed wrapping the body part
        let mut mixed = MultiPart::mixed().multipart(body_part);
        for att in &validated.attachments {
            let content_type = att.content_type
                .parse::<lettre::message::header::ContentType>()
                .map_err(|e| {
                    error!(error = %e, content_type = %att.content_type, "invalid content_type");
                    AppError::Internal
                })?;
            mixed = mixed.singlepart(
                Attachment::new(att.filename.clone()).body(att.decoded.clone(), content_type)
            );
        }
        builder
            .multipart(mixed)
            .map_err(|e| { error!(error = %e, "failed to build mixed message"); AppError::Internal })?
    };

    Ok(message)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{
            ApiKeyConfig, AppConfig, LoggingConfig, MailConfig, RateLimitConfig, SecretString,
            SecurityConfig, ServerConfig, SmtpConfig,
        },
        validation::ValidatedMailRequest,
    };

    fn minimal_config() -> AppConfig {
        AppConfig {
            server: ServerConfig {
                bind_address: "127.0.0.1:8080".into(),
                max_request_body_bytes: 65536,
                request_timeout_seconds: 30,
                shutdown_timeout_seconds: 30,
                concurrency_limit: 0,
            },
            security: SecurityConfig {
                require_auth: true,
                trust_proxy_headers: false,
                trusted_source_cidrs: vec![],
                api_keys: vec![ApiKeyConfig {
                    id: "test".into(),
                    secret: SecretString::new("tok"),
                    enabled: true,
                    description: None,
                    allowed_recipient_domains: vec![],
                    rate_limit_per_min: None,
                    allowed_recipients: vec![],
                    burst: 0,
                    mask_recipient: None,
                }],
                allowed_source_cidrs: vec![],
            },
            mail: MailConfig {
                default_from: "relay@example.com".into(),
                default_from_name: Some("Relay".into()),
                allowed_recipient_domains: vec![],
                max_subject_chars: 200,
                max_body_bytes: 1_000_000,
                max_recipients: 10,
                max_attachments: 5,
                max_attachment_bytes: 10 * 1024 * 1024,
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

    fn minimal_validated() -> ValidatedMailRequest {
        ValidatedMailRequest {
            to: vec!["user@example.com".into()],
            subject: "Hello".into(),
            body: "Test body.".into(),
            from_name: None,
            reply_to: vec![],
            body_html: None,
            cc: vec![],
            attachments: vec![],
            client_request_id: None,
        }
    }

    #[test]
    fn valid_message_builds() {
        let cfg = minimal_config();
        let v = minimal_validated();
        assert!(build_message(&v, &cfg).is_ok());
    }

    #[test]
    fn from_is_always_from_config() {
        let cfg = minimal_config();
        let v = minimal_validated();
        let msg = build_message(&v, &cfg).unwrap();
        let from = msg.headers().get::<lettre::message::header::From>().unwrap();
        assert!(format!("{:?}", from).contains("relay@example.com"));
    }

    #[test]
    fn from_name_applied_from_request() {
        let cfg = minimal_config();
        let v = ValidatedMailRequest {
            from_name: Some("Custom Name".into()),
            ..minimal_validated()
        };
        let msg = build_message(&v, &cfg).unwrap();
        let from = msg.headers().get::<lettre::message::header::From>().unwrap();
        assert!(format!("{:?}", from).contains("Custom Name"));
    }

    #[test]
    fn reply_to_applied_when_present() {
        let cfg = minimal_config();
        let v = ValidatedMailRequest {
            reply_to: vec!["support@example.com".into()],
            ..minimal_validated()
        };
        let msg = build_message(&v, &cfg).unwrap();
        assert!(msg.headers().get::<lettre::message::header::ReplyTo>().is_some());
    }

    #[test]
    fn no_reply_to_when_absent() {
        let cfg = minimal_config();
        let v = minimal_validated();
        let msg = build_message(&v, &cfg).unwrap();
        assert!(msg.headers().get::<lettre::message::header::ReplyTo>().is_none());
    }
}
