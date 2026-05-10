//! Integration-level security regression tests.
//!
//! These tests exercise the full Axum router using `tower::ServiceExt::oneshot`.
//! No real TCP connection is made — the router is called in-process.
//!
//! # Coverage
//!
//! | ID      | What                                    | Covered here |
//! |---------|-----------------------------------------|-------------|
//! | SEC-001 | No auth header → 401                    | ✓ |
//! | SEC-002 | Wrong token → 403                       | ✓ |
//! | SEC-003 | Disabled key → 403                      | ✓ |
//! | SEC-004 | CR/LF in subject → 400                  | validation::tests |
//! | SEC-005 | CR/LF in from_name → 400                | validation::tests |
//! | SEC-006 | CR/LF in reply_to → 400                 | validation::tests |
//! | SEC-007 | CR/LF in to → 400                       | validation::tests |
//! | SEC-008 | Unknown field "from" → 400              | ✓ |
//! | SEC-009 | Unknown field "bcc" → 400               | ✓ |
//! | SEC-010 | Unknown field "headers" → 400           | ✓ |
//! | SEC-011 | Body too large → 413                    | ✓ |
//! | SEC-012 | Disallowed domain → 400                 | validation::tests |
//! | SEC-013 | Rate limit exceeded → 429               | rate_limit::tests |
//! | SEC-014 | Forged X-Forwarded-For from untrusted   | auth::tests (unit) |
//! | SEC-015 | Auth log has no token value             | structural (no log sink in unit) |
//! | SEC-016 | Send log has no body value              | structural (skip(payload) enforced) |
//! | SEC-017 | SecretString Debug is redacted          | validation::tests + config::tests |

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;

use crate::{
    api,
    config::{
        ApiKeyConfig, AppConfig, LoggingConfig, MailConfig, RateLimitConfig, SecretString,
        SecurityConfig, ServerConfig, SmtpConfig,
    },
    AppState,
};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Build a minimal, fully-functional AppConfig for tests.
///
/// Uses port 1 for SMTP — connections will always fail, but the transport
/// object is constructed without error (no connection at init time).
fn test_config() -> AppConfig {
    AppConfig {
        server: ServerConfig {
            bind_address: "127.0.0.1:0".into(),
            max_request_body_bytes: 256,  // intentionally small for SEC-011
            request_timeout_seconds: 5,
            shutdown_timeout_seconds: 5,
            concurrency_limit: 0,
        },
        security: SecurityConfig {
            require_auth: true,
            trust_proxy_headers: false,
            trusted_source_cidrs: vec![],
            allowed_source_cidrs: vec![],
            api_keys: vec![
                ApiKeyConfig {
                    id: "enabled-key".into(),
                    secret: SecretString::new("valid-secret"),
                    enabled: true,
                    description: None,
                    allowed_recipient_domains: vec!["example.com".into()],
                    rate_limit_per_min: None,
                    allowed_recipients: vec![],
                    burst: 0,
                },
                ApiKeyConfig {
                    id: "disabled-key".into(),
                    secret: SecretString::new("disabled-secret"),
                    enabled: false,
                    description: None,
                    allowed_recipient_domains: vec![],
                    rate_limit_per_min: None,
                    allowed_recipients: vec![],
                    burst: 0,
                },
            ],
        },
        mail: MailConfig {
            default_from: "relay@example.com".into(),
            default_from_name: None,
            allowed_recipient_domains: vec!["example.com".into()],
            max_subject_chars: 255,
            max_body_bytes: 200,  // intentionally small for SEC-011
            max_recipients: 10,
        },
        smtp: SmtpConfig {
            mode: "smtp".into(),
            host: "127.0.0.1".into(),
            port: 1,  // no listener — SMTP submit will fail, but that's after validation
            connect_timeout_seconds: 1,
            submission_timeout_seconds: 1,
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
            level: "error".into(),  // suppress output during tests
            mask_recipient: true,
        },
    }
}

fn test_router() -> axum::Router {
    let state = AppState::new(test_config());
    api::build_router(state)
}

/// POST /v1/send with a full valid auth header and JSON body.
async fn send_request(
    router: &axum::Router,
    auth: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/v1/send")
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = auth {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let req = builder.body(Body::from(body.to_string())).unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 8192).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(json!({}));
    (status, json)
}

fn valid_body() -> Value {
    json!({
        "to": "user@example.com",
        "subject": "Test",
        "body": "Hello."
    })
}

// ---------------------------------------------------------------------------
// SEC-001: No Authorization header → 401 unauthorized
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sec_001_no_auth_header_returns_401() {
    let router = test_router();
    let (status, body) = send_request(&router, None, valid_body()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "body={body}");
    assert_eq!(body["code"], "unauthorized");
}

// ---------------------------------------------------------------------------
// SEC-002: Wrong token → 403 forbidden
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sec_002_wrong_token_returns_403() {
    let router = test_router();
    let (status, body) = send_request(&router, Some("completely-wrong"), valid_body()).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body={body}");
    assert_eq!(body["code"], "forbidden");
}

// ---------------------------------------------------------------------------
// SEC-003: Disabled key with correct secret → 403 (not 200)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sec_003_disabled_key_returns_403() {
    let router = test_router();
    let (status, body) = send_request(&router, Some("disabled-secret"), valid_body()).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body={body}");
    assert_eq!(body["code"], "forbidden");
}

// ---------------------------------------------------------------------------
// SEC-008: Unknown field "from" → 400 validation_failed
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sec_008_unknown_field_from_rejected() {
    let router = test_router();
    let bad = json!({
        "to": "user@example.com",
        "subject": "Test",
        "body": "Hello.",
        "from": "evil@evil.com"
    });
    let (status, body) = send_request(&router, Some("valid-secret"), bad).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "body={body}");
}

// ---------------------------------------------------------------------------
// SEC-009: Unknown field "bcc" → 400 / 422
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sec_009_unknown_field_bcc_rejected() {
    let router = test_router();
    let bad = json!({
        "to": "user@example.com",
        "subject": "Test",
        "body": "Hello.",
        "bcc": "spy@evil.com"
    });
    let (status, _) = send_request(&router, Some("valid-secret"), bad).await;
    assert!(
        status == StatusCode::UNPROCESSABLE_ENTITY || status == StatusCode::BAD_REQUEST,
        "expected 422 or 400, got {status}"
    );
}

// ---------------------------------------------------------------------------
// SEC-010: Unknown field "headers" → 400 / 422
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sec_010_unknown_field_headers_rejected() {
    let router = test_router();
    let bad = json!({
        "to": "user@example.com",
        "subject": "Test",
        "body": "Hello.",
        "headers": {"X-Custom": "injected"}
    });
    let (status, _) = send_request(&router, Some("valid-secret"), bad).await;
    assert!(
        status == StatusCode::UNPROCESSABLE_ENTITY || status == StatusCode::BAD_REQUEST,
        "expected 422 or 400, got {status}"
    );
}

// ---------------------------------------------------------------------------
// SEC-011: Body exceeding max_request_body_bytes → 413
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sec_011_oversized_request_body_returns_413() {
    let router = test_router();
    // test_config sets max_request_body_bytes = 256
    let giant = "x".repeat(300);
    let req = Request::builder()
        .method("POST")
        .uri("/v1/send")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer valid-secret")
        .body(Body::from(giant))
        .unwrap();
    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

// ---------------------------------------------------------------------------
// Structural: From is always from config (mail::tests also cover this)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn from_address_cannot_be_overridden_via_extra_field() {
    // The "from" field is rejected by deny_unknown_fields (SEC-008).
    // This test double-checks that even with a crafted payload structure,
    // the router does not accept it.
    let router = test_router();
    let with_from = json!({
        "to": "user@example.com",
        "subject": "Hi",
        "body": "Text.",
        "from": "spoofed@attacker.com"
    });
    let (status, _) = send_request(&router, Some("valid-secret"), with_from).await;
    assert_ne!(
        status,
        StatusCode::ACCEPTED,
        "A request with a 'from' field must never result in 202 Accepted"
    );
}
