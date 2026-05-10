//! Authentication and access control.
//!
//! Implements RFC 040: API key authentication via Axum `FromRequestParts`.
//!
//! # Flow
//!
//! ```text
//! Request
//!   -> resolve client IP (socket peer / X-Forwarded-For via trusted proxy)
//!   -> extract token from Authorization: Bearer or X-API-Key header
//!   -> constant-time compare against each enabled api_key secret
//!   -> check source CIDR allowlist (if configured)
//!   -> produce AuthContext on success
//!   -> return 401 / 403 on failure
//! ```
//!
//! # Security notes
//!
//! - Token comparison always uses constant-time equality to prevent timing attacks.
//! - Tokens are never logged; only `key_id` (non-secret) is propagated.
//! - Forwarded headers are only trusted when the peer IP is in `trusted_source_cidrs`.
//! - Source IP allowlist is enforced via `allowed_source_cidrs` (distinct field).

use std::{net::IpAddr, sync::Arc};

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
};
use ipnet::IpNet;
use subtle::ConstantTimeEq;
use tracing::warn;

use crate::{config::ApiKeyConfig, AppState};

// ---------------------------------------------------------------------------
// AuthContext
// ---------------------------------------------------------------------------

/// Proof of successful authentication for a single request.
///
/// Produced by the `AuthContext` Axum extractor.
/// Only `key_id` is stored — the secret is never retained after comparison.
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// Non-secret identifier for the matched API key (suitable for logging).
    pub key_id: String,
    /// Resolved client IP after trusted-proxy handling.
    pub client_ip: IpAddr,
    /// Per-key rate limit override (tokens/minute). None = use global default.
    pub key_rate_limit_per_min: Option<u32>,
    /// Per-key burst override. 0 = use global default.
    pub key_burst: u32,
}

// ---------------------------------------------------------------------------
// Axum extractor
// ---------------------------------------------------------------------------

impl<S> FromRequestParts<S> for AuthContext
where
    Arc<AppState>: axum::extract::FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, axum::Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = Arc::<AppState>::from_ref(state);
        let cfg = app_state.config();
        let security = &cfg.security;

        // ------------------------------------------------------------------
        // 1. Resolve client IP
        // ------------------------------------------------------------------
        let peer_ip = resolve_peer_ip(parts);
        let client_ip = if security.trust_proxy_headers {
            resolve_client_ip(parts, peer_ip, &security.trusted_source_cidrs)
        } else {
            peer_ip
        };

        // ------------------------------------------------------------------
        // 2. Source CIDR allowlist (empty = allow all)
        //
        // `allowed_source_cidrs` controls which resolved client IPs may proceed.
        // This is distinct from `trusted_source_cidrs` (proxy header trust).
        // ------------------------------------------------------------------
        if !security.allowed_source_cidrs.is_empty()
            && !ip_in_cidrs(client_ip, &security.allowed_source_cidrs)
        {
            warn!(
                client_ip = %client_ip,
                "auth: client IP not in allowed_source_cidrs"
            );
            return Err(forbidden());
        }

        // ------------------------------------------------------------------
        // 3. Extract token from headers
        // ------------------------------------------------------------------
        let token = match extract_token(parts) {
            Some(t) => t,
            None => {
                warn!(client_ip = %client_ip, "auth: missing or malformed token");
                return Err(unauthorized());
            }
        };

        // ------------------------------------------------------------------
        // 4. Constant-time match against api_keys
        // ------------------------------------------------------------------
        match find_matching_key(&security.api_keys, token) {
            MatchResult::Matched(key_id, key_rate_limit_per_min, key_burst) => Ok(AuthContext { key_id, client_ip, key_rate_limit_per_min, key_burst }),
            MatchResult::Disabled(key_id) => {
                warn!(
                    client_ip = %client_ip,
                    key_id = %key_id,
                    "auth: key is disabled"
                );
                Err(forbidden())
            }
            MatchResult::NotFound => {
                warn!(client_ip = %client_ip, "auth: token not matched");
                Err(forbidden())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Token extraction
// ---------------------------------------------------------------------------

/// Extract token string from `Authorization: Bearer <token>` or `X-API-Key: <token>`.
///
/// Priority: `Authorization` > `X-API-Key`.
/// Returns `None` if neither header is present or if `Authorization` is
/// present but malformed (not `Bearer `-prefixed).
fn extract_token(parts: &Parts) -> Option<&str> {
    // Authorization: Bearer <token>
    if let Some(auth) = parts
        .headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        return auth.strip_prefix("Bearer ");
        // Explicit return: if Authorization is present but malformed, reject.
        // Do NOT fall through to X-API-Key.
    }

    // X-API-Key: <token>  (fallback)
    parts
        .headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
}

// ---------------------------------------------------------------------------
// Key matching
// ---------------------------------------------------------------------------

enum MatchResult {
    /// key_id, rate_limit_per_min, burst
    Matched(String, Option<u32>, u32),
    Disabled(String),
    NotFound,
}

/// Compare `token` against every configured API key using constant-time equality.
///
/// All comparisons are performed regardless of early match to avoid
/// timing-based enumeration of which keys are configured.
fn find_matching_key(keys: &[ApiKeyConfig], token: &str) -> MatchResult {
    let token_bytes = token.as_bytes();
    let mut matched_key: Option<&ApiKeyConfig> = None;

    for key in keys {
        let secret_bytes = key.secret.expose().as_bytes();
        if ct_eq_bytes(token_bytes, secret_bytes) {
            matched_key = Some(key);
            // Continue loop — do not break — to avoid timing difference.
        }
    }

    match matched_key {
        Some(k) if k.enabled => MatchResult::Matched(k.id.clone(), k.rate_limit_per_min, k.burst),
        Some(k) => MatchResult::Disabled(k.id.clone()),
        None => MatchResult::NotFound,
    }
}

/// Constant-time byte-slice comparison.
///
/// Length mismatch is detected before comparison (leaking only length, not content).
/// This is acceptable since token lengths are not secret.
fn ct_eq_bytes(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}

// ---------------------------------------------------------------------------
// Client IP resolution
// ---------------------------------------------------------------------------

/// Return the socket peer IP address.
///
/// Falls back to `127.0.0.1` if the peer address is unavailable (e.g., in tests).
fn resolve_peer_ip(parts: &Parts) -> IpAddr {
    parts
        .extensions
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip())
        .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST))
}

/// Resolve the effective client IP, honouring proxy headers only when
/// the peer IP is in the trusted proxy CIDR list.
///
/// Priority (RFC 303):
/// 1. `Forwarded: for=<addr>` — RFC 7239 standard header
/// 2. `X-Forwarded-For: <addr>` — de-facto standard, leftmost entry
/// 3. Socket peer IP
fn resolve_client_ip(parts: &Parts, peer_ip: IpAddr, trusted_cidrs: &[String]) -> IpAddr {
    if !ip_in_cidrs(peer_ip, trusted_cidrs) {
        return peer_ip;
    }
    // Try RFC 7239 `Forwarded` header first.
    if let Some(ip) = parse_forwarded_for(&parts.headers) {
        return ip;
    }
    // Fall back to the leftmost entry in `X-Forwarded-For`.
    parts
        .headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .and_then(|s| s.trim().parse::<IpAddr>().ok())
        .unwrap_or(peer_ip)
}

/// Parse the `for` directive from an RFC 7239 `Forwarded` header.
///
/// Handles:
/// - `Forwarded: for=1.2.3.4`
/// - `Forwarded: for="[::1]"`  (IPv6 in brackets)
/// - `Forwarded: for=1.2.3.4;proto=https;host=example.com` (multiple params)
/// - `Forwarded: for=a.b.c.d, for=e.f.g.h` (multiple list items — uses first)
fn parse_forwarded_for(headers: &axum::http::HeaderMap) -> Option<IpAddr> {
    let value = headers.get("forwarded")?.to_str().ok()?;
    // Multiple list items separated by commas — use the first (leftmost = original client).
    let first_item = value.split(',').next()?;
    // Find the `for=` directive within this item.
    for part in first_item.split(';') {
        let part = part.trim();
        let lower = part.to_ascii_lowercase();
        if let Some(addr_part) = lower.strip_prefix("for=") {
            // Strip surrounding quotes and IPv6 brackets.
            let addr_str = part[part.len() - addr_part.len()..]
                .trim_matches('"')
                .trim_matches('[')
                .trim_matches(']');
            if let Ok(ip) = addr_str.parse::<IpAddr>() {
                return Some(ip);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// CIDR helpers
// ---------------------------------------------------------------------------

fn ip_in_cidrs(ip: IpAddr, cidrs: &[String]) -> bool {
    cidrs
        .iter()
        .filter_map(|s| s.parse::<IpNet>().ok())
        .any(|net| net.contains(&ip))
}

// ---------------------------------------------------------------------------
// Error responses
// ---------------------------------------------------------------------------

fn unauthorized() -> (StatusCode, axum::Json<serde_json::Value>) {
    (
        StatusCode::UNAUTHORIZED,
        axum::Json(serde_json::json!({
            "status": "error",
            "code": "unauthorized",
            "message": "Authentication required"
        })),
    )
}

fn forbidden() -> (StatusCode, axum::Json<serde_json::Value>) {
    (
        StatusCode::FORBIDDEN,
        axum::Json(serde_json::json!({
            "status": "error",
            "code": "forbidden",
            "message": "Access denied"
        })),
    )
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ApiKeyConfig, SecretString};

    fn make_key(id: &str, secret: &str, enabled: bool) -> ApiKeyConfig {
        ApiKeyConfig {
            id: id.to_string(),
            secret: SecretString::new(secret),
            enabled,
            description: None,
            allowed_recipient_domains: vec![],
            rate_limit_per_min: None,
            allowed_recipients: vec![],
            burst: 0,
        }
    }

    #[test]
    fn matching_key_returns_key_id() {
        let keys = vec![make_key("svc-a", "secret-a", true)];
        match find_matching_key(&keys, "secret-a") {
            MatchResult::Matched(id, _, _) => assert_eq!(id, "svc-a"),
            _ => panic!("expected Matched"),
        }
    }

    #[test]
    fn wrong_token_returns_not_found() {
        let keys = vec![make_key("svc-a", "secret-a", true)];
        assert!(matches!(
            find_matching_key(&keys, "wrong"),
            MatchResult::NotFound
        ));
    }

    #[test]
    fn disabled_key_returns_disabled() {
        let keys = vec![make_key("svc-a", "secret-a", false)];
        assert!(matches!(
            find_matching_key(&keys, "secret-a"),
            MatchResult::Disabled(_)
        ));
    }

    #[test]
    fn multiple_keys_correct_one_matches() {
        let keys = vec![
            make_key("svc-a", "token-aaa", true),
            make_key("svc-b", "token-bbb", true),
        ];
        match find_matching_key(&keys, "token-bbb") {
            MatchResult::Matched(id, _, _) => assert_eq!(id, "svc-b"),
            _ => panic!("expected Matched for svc-b"),
        }
    }

    #[test]
    fn ip_in_cidrs_loopback() {
        let cidrs = vec!["127.0.0.1/32".to_string()];
        assert!(ip_in_cidrs("127.0.0.1".parse().unwrap(), &cidrs));
        assert!(!ip_in_cidrs("10.0.0.1".parse().unwrap(), &cidrs));
    }

    #[test]
    fn ip_in_cidrs_range() {
        let cidrs = vec!["10.0.0.0/8".to_string()];
        assert!(ip_in_cidrs("10.1.2.3".parse().unwrap(), &cidrs));
        assert!(!ip_in_cidrs("192.168.1.1".parse().unwrap(), &cidrs));
    }

    #[test]
    fn empty_cidr_list_returns_false() {
        assert!(!ip_in_cidrs("127.0.0.1".parse().unwrap(), &[]));
    }

    #[test]
    fn different_length_tokens_do_not_match() {
        assert!(!ct_eq_bytes(b"short", b"longer-token"));
    }
}
