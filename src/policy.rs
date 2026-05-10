//! Recipient domain and API key permission policy.
//!
//! This module provides policy lookup helpers used by the validation pipeline.
//! The actual enforcement logic lives in `validation.rs`; this module provides
//! the data structures and query helpers that can be used independently.

use crate::config::{ApiKeyConfig, AppConfig};

/// Look up the API key config by `key_id`.
pub fn find_key<'a>(config: &'a AppConfig, key_id: &str) -> Option<&'a ApiKeyConfig> {
    config.security.api_keys.iter().find(|k| k.id == key_id)
}

/// Return `true` if the given domain is permitted by the global policy.
///
/// An empty `allowed_recipient_domains` list means "allow all domains".
pub fn domain_permitted_globally(config: &AppConfig, domain: &str) -> bool {
    config.mail.allowed_recipient_domains.is_empty()
        || config
            .mail
            .allowed_recipient_domains
            .iter()
            .any(|d| d.eq_ignore_ascii_case(domain))
}

/// Return `true` if the given domain is permitted for the specified API key.
///
/// If the key has no domain restriction (empty list), all domains are allowed
/// (subject to the global policy).
pub fn domain_permitted_for_key(key: &ApiKeyConfig, domain: &str) -> bool {
    key.allowed_recipient_domains.is_empty()
        || key
            .allowed_recipient_domains
            .iter()
            .any(|d| d.eq_ignore_ascii_case(domain))
}

/// Return `true` if the given address is permitted for the specified API key,
/// considering both the per-address allowlist and the per-domain allowlist.
///
/// Precedence (RFC 204):
/// 1. If `allowed_recipients` is non-empty, the full address must be present
///    (case-insensitive comparison).
/// 2. Otherwise, fall through to `domain_permitted_for_key`.
pub fn address_permitted_for_key(key: &ApiKeyConfig, address: &str) -> bool {
    if !key.allowed_recipients.is_empty() {
        return key
            .allowed_recipients
            .iter()
            .any(|r| r.eq_ignore_ascii_case(address));
    }
    domain_permitted_for_key(key, extract_domain(address))
}

fn extract_domain(address: &str) -> &str {
    address.rfind('@').map(|i| &address[i + 1..]).unwrap_or("")
}
