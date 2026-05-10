//! Integration and security regression tests.
//!
//! Implements RFC 102 (complete SEC-001–017 matrix) and RFC 103 (E2E scenarios).
//! Uses `SmtpStub` for tests that verify SMTP submission end-to-end.

mod smtp_stub;
mod common;

use axum::http::StatusCode;
use tower::ServiceExt;

use serde_json::json;

use common::{send, send_valid, test_router, test_router_no_smtp, RequestBuilder};
use smtp_stub::{SmtpStub, StubConfig};


// RFC 703 — Bulk Send Integration Tests
// ===========================================================================

fn bulk_body(messages: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "messages": messages })
}

fn two_valid_messages() -> serde_json::Value {
    serde_json::json!([
        {"to": "alice@example.com", "subject": "Hi Alice", "body": "Hello."},
        {"to": "bob@example.com",   "subject": "Hi Bob",   "body": "Hello."}
    ])
}

/// BULK-001: two valid messages → 202, both accepted.
#[tokio::test]
async fn bulk_001_two_valid_messages_accepted() {
    let stub = SmtpStub::start(0).await;
    let router = test_router(stub.port());

    let resp = send(&router, RequestBuilder::post("/v1/send-bulk")
        .bearer("primary-secret")
        .json(bulk_body(two_valid_messages()))
        .build()).await;

    resp.assert_status(StatusCode::ACCEPTED);
    assert_eq!(resp.body["total"],    2);
    assert_eq!(resp.body["accepted"], 2);
    assert_eq!(resp.body["rejected"], 0);
    assert_eq!(resp.body["results"].as_array().unwrap().len(), 2);
    assert!(resp.body["bulk_request_id"].as_str().unwrap().starts_with("req_"));

    stub.shutdown().await;
}

/// BULK-002: one valid + one invalid → 202, partial results.
#[tokio::test]
async fn bulk_002_one_valid_one_invalid_partial() {
    let stub = SmtpStub::start(0).await;
    let router = test_router(stub.port());

    let msgs = serde_json::json!([
        {"to": "alice@example.com",          "subject": "OK",  "body": "Hello."},
        {"to": "evil@disallowed.invalid",    "subject": "Bad", "body": "Hello."}
    ]);
    let resp = send(&router, RequestBuilder::post("/v1/send-bulk")
        .bearer("primary-secret")
        .json(bulk_body(msgs))
        .build()).await;

    resp.assert_status(StatusCode::ACCEPTED);
    assert_eq!(resp.body["accepted"], 1);
    assert_eq!(resp.body["rejected"], 1);

    let results = resp.body["results"].as_array().unwrap();
    assert_eq!(results[0]["status"], "accepted");
    assert_eq!(results[1]["status"], "rejected");
    assert_eq!(results[1]["code"],   "validation_failed");

    stub.shutdown().await;
}

/// BULK-003: empty messages array → 400.
#[tokio::test]
async fn bulk_003_empty_array_returns_400() {
    let router = test_router_no_smtp();
    let resp = send(&router, RequestBuilder::post("/v1/send-bulk")
        .bearer("primary-secret")
        .json(bulk_body(serde_json::json!([])))
        .build()).await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

/// BULK-004: array exceeds max_bulk_messages → 400.
#[tokio::test]
async fn bulk_004_exceeds_max_messages_returns_400() {
    use http_smtp_rele::{api, config::*, AppState};
    let mut cfg = common::test_config(1);
    cfg.mail.max_bulk_messages = 2;
    let router = api::build_router(AppState::new(cfg));

    let msgs: Vec<serde_json::Value> = (0..3)
        .map(|i| serde_json::json!({"to": format!("u{}@example.com", i),
                                     "subject": "S", "body": "B"}))
        .collect();
    let resp = send(&router, RequestBuilder::post("/v1/send-bulk")
        .bearer("primary-secret")
        .json(bulk_body(serde_json::json!(msgs)))
        .build()).await;
    resp.assert_status(StatusCode::PAYLOAD_TOO_LARGE);
}

/// BULK-005: each message has a unique request_id.
#[tokio::test]
async fn bulk_005_each_message_has_unique_request_id() {
    let stub = SmtpStub::start(0).await;
    let router = test_router(stub.port());

    let resp = send(&router, RequestBuilder::post("/v1/send-bulk")
        .bearer("primary-secret")
        .json(bulk_body(two_valid_messages()))
        .build()).await;

    resp.assert_status(StatusCode::ACCEPTED);
    let results = resp.body["results"].as_array().unwrap();
    let id0 = results[0]["request_id"].as_str().unwrap();
    let id1 = results[1]["request_id"].as_str().unwrap();
    assert_ne!(id0, id1, "each message must have a distinct request_id");
    assert!(id0.starts_with("req_"));
    assert!(id1.starts_with("req_"));

    stub.shutdown().await;
}

/// BULK-006: per-message request_id queryable via GET /v1/submissions/.
#[tokio::test]
async fn bulk_006_per_message_status_queryable() {
    let stub = SmtpStub::start(0).await;
    let router = test_router(stub.port());

    let bulk_resp = send(&router, RequestBuilder::post("/v1/send-bulk")
        .bearer("primary-secret")
        .json(bulk_body(two_valid_messages()))
        .build()).await;
    bulk_resp.assert_status(StatusCode::ACCEPTED);

    let results = bulk_resp.body["results"].as_array().unwrap();
    for r in results {
        let rid = r["request_id"].as_str().unwrap();
        let status_resp = send(&router,
            RequestBuilder::get(&format!("/v1/submissions/{rid}"))
                .bearer("primary-secret")
                .build()).await;
        status_resp.assert_status(StatusCode::OK);
        assert_eq!(status_resp.body["status"], "smtp_accepted");
    }

    stub.shutdown().await;
}

/// BULK-007: unauthenticated request → 401, no messages processed.
#[tokio::test]
async fn bulk_007_unauthenticated_returns_401() {
    let router = test_router_no_smtp();
    let resp = send(&router, RequestBuilder::post("/v1/send-bulk")
        .no_auth()
        .json(bulk_body(two_valid_messages()))
        .build()).await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
}

/// BULK-008: response contains no mail body / full recipient addresses.
#[tokio::test]
async fn bulk_008_response_excludes_sensitive_data() {
    let stub = SmtpStub::start(0).await;
    let router = test_router(stub.port());

    let msgs = serde_json::json!([{
        "to": "alice@example.com",
        "subject": "SecretSubject123",
        "body": "SecretBody456"
    }]);
    let resp = send(&router, RequestBuilder::post("/v1/send-bulk")
        .bearer("primary-secret")
        .json(bulk_body(msgs))
        .build()).await;

    let body_str = resp.body.to_string();
    assert!(!body_str.contains("SecretSubject123"), "subject must not appear in response");
    assert!(!body_str.contains("SecretBody456"),    "body must not appear in response");
    assert!(!body_str.contains("primary-secret"),   "token must not appear in response");
    assert!(!body_str.contains("alice@example.com"), "full address must not appear in response");

    stub.shutdown().await;
}

/// BULK-009: bulk_request_id is present and has req_ prefix.
#[tokio::test]
async fn bulk_009_bulk_request_id_format() {
    let stub = SmtpStub::start(0).await;
    let router = test_router(stub.port());

    let resp = send(&router, RequestBuilder::post("/v1/send-bulk")
        .bearer("primary-secret")
        .json(bulk_body(two_valid_messages()))
        .build()).await;

    let brid = resp.body["bulk_request_id"].as_str().unwrap_or("");
    assert!(brid.starts_with("req_"), "bulk_request_id must start with req_: {brid}");

    stub.shutdown().await;
}

/// BULK: all-SMTP-failed → 202 with all rejected.
#[tokio::test]
async fn bulk_all_smtp_failed_returns_202_with_rejected() {
    let router = test_router(1); // port 1 has no listener

    let resp = send(&router, RequestBuilder::post("/v1/send-bulk")
        .bearer("primary-secret")
        .json(bulk_body(two_valid_messages()))
        .build()).await;

    resp.assert_status(StatusCode::ACCEPTED);
    assert_eq!(resp.body["accepted"], 0);
    assert_eq!(resp.body["rejected"], 2);
    let results = resp.body["results"].as_array().unwrap();
    for r in results {
        assert_eq!(r["code"], "smtp_unavailable");
    }
}

// ===========================================================================
// RFC 711 — Bulk SMTP Parallelism
// ===========================================================================

/// RFC-711-01: bulk concurrency setting respected — results stay in index order.
#[tokio::test]
async fn bulk_parallelism_results_in_index_order() {
    use http_smtp_rele::{api, config::*, AppState};
    let stub = SmtpStub::start(0).await;
    let mut cfg = common::test_config(stub.port());
    cfg.smtp.bulk_concurrency = 2; // explicit concurrency cap
    let router = api::build_router(AppState::new(cfg));

    let msgs: Vec<serde_json::Value> = (0..4)
        .map(|i| serde_json::json!({
            "to": format!("user{}@example.com", i),
            "subject": format!("Msg {}", i),
            "body": "Hello."
        }))
        .collect();

    let resp = send(&router, RequestBuilder::post("/v1/send-bulk")
        .bearer("primary-secret")
        .json(serde_json::json!({"messages": msgs}))
        .build()).await;

    resp.assert_status(StatusCode::ACCEPTED);
    let results = resp.body["results"].as_array().unwrap();
    assert_eq!(results.len(), 4);
    // Indices must always be 0, 1, 2, 3 in order
    for (i, r) in results.iter().enumerate() {
        assert_eq!(r["index"], i, "result at position {i} must have index {i}");
    }

    stub.shutdown().await;
}

/// RFC-711-02: bulk_concurrency = 0 means unlimited (no semaphore deadlock).
#[tokio::test]
async fn bulk_concurrency_zero_means_unlimited() {
    use http_smtp_rele::{api, config::*, AppState};
    let stub = SmtpStub::start(0).await;
    let mut cfg = common::test_config(stub.port());
    cfg.smtp.bulk_concurrency = 0; // unlimited
    let router = api::build_router(AppState::new(cfg));

    let msgs: Vec<serde_json::Value> = (0..3)
        .map(|i| serde_json::json!({
            "to": format!("u{}@example.com", i),
            "subject": "S", "body": "B"
        }))
        .collect();
    let resp = send(&router, RequestBuilder::post("/v1/send-bulk")
        .bearer("primary-secret")
        .json(serde_json::json!({"messages": msgs}))
        .build()).await;

    resp.assert_status(StatusCode::ACCEPTED);
    assert_eq!(resp.body["accepted"], 3);
    stub.shutdown().await;
}

// ===========================================================================
// RFC 712 — HTTP Server TLS Config Validation
// ===========================================================================

/// RFC-712-01: tls_cert without tls_key rejected at config load.
#[test]
fn tls_cert_without_key_rejected() {
    let toml = r#"
[server]
bind_address = "127.0.0.1:8080"
tls_cert = "/tmp/cert.pem"
[security]
require_auth = false
[[api_keys]]
id = "k"
secret = "s"
[mail]
default_from = "a@example.com"
allowed_recipient_domains = ["example.com"]
[smtp]
host = "127.0.0.1"
port = 25
"#;
    let result = http_smtp_rele::config::load_from_str(toml);
    assert!(result.is_err(), "cert-only must be rejected");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("tls_cert") || msg.contains("tls_key"),
        "error must mention tls fields: {msg}");
}

/// RFC-712-02: tls_key without tls_cert rejected.
#[test]
fn tls_key_without_cert_rejected() {
    let toml = r#"
[server]
bind_address = "127.0.0.1:8080"
tls_key = "/tmp/key.pem"
[security]
require_auth = false
[[api_keys]]
id = "k"
secret = "s"
[mail]
default_from = "a@example.com"
allowed_recipient_domains = ["example.com"]
[smtp]
host = "127.0.0.1"
port = 25
"#;
    let result = http_smtp_rele::config::load_from_str(toml);
    assert!(result.is_err(), "key-only must be rejected");
}

/// RFC-712-03: neither tls_cert nor tls_key is valid (plain HTTP).
#[test]
fn no_tls_config_is_valid_plain_http() {
    let toml = r#"
[server]
bind_address = "127.0.0.1:8080"
[security]
require_auth = false
[[api_keys]]
id = "k"
secret = "s"
[mail]
default_from = "a@example.com"
allowed_recipient_domains = ["example.com"]
[smtp]
host = "127.0.0.1"
port = 25
"#;
    let result = http_smtp_rele::config::load_from_str(toml);
    assert!(result.is_ok(), "no TLS config must be valid: {:?}", result);
}

/// RFC-712-04: non-TLS build rejects both-set TLS config.
#[cfg(not(feature = "tls"))]
#[test]
fn non_tls_build_rejects_tls_config() {
    let toml = r#"
[server]
bind_address = "127.0.0.1:8080"
tls_cert = "/tmp/cert.pem"
tls_key  = "/tmp/key.pem"
[security]
require_auth = false
[[api_keys]]
id = "k"
secret = "s"
[mail]
default_from = "a@example.com"
allowed_recipient_domains = ["example.com"]
[smtp]
host = "127.0.0.1"
port = 25
"#;
    let result = http_smtp_rele::config::load_from_str(toml);
    assert!(result.is_err(), "TLS config in non-TLS build must be rejected");
    assert!(result.unwrap_err().to_string().contains("not available"),
        "error must mention build flag");
}

// ===========================================================================
// RFC 721 — OpenBSD SIGHUP rpath (config validation test)
// ===========================================================================

/// RFC-721: SIGHUP reload configuration is tested at config level.
/// The actual pledge/unveil is OpenBSD-only and not testable in CI;
/// we verify that the config structure supports reload.
#[test]
fn sighup_reload_config_structure_valid() {
    // load_from_str exercises validate_config, which is the
    // same path used in the SIGHUP handler
    let toml = r#"
[server]
bind_address = "127.0.0.1:8080"
[security]
require_auth = false
[[api_keys]]
id = "k"
secret = "s"
[mail]
default_from = "a@example.com"
allowed_recipient_domains = ["example.com"]
[smtp]
host = "127.0.0.1"
port = 25
"#;
    let result = http_smtp_rele::config::load_from_str(toml);
    assert!(result.is_ok(), "basic config must parse for SIGHUP reload: {:?}", result);
}

// ===========================================================================
// RFC 722 — Redis config validation (no running Redis required)
// ===========================================================================

/// redis_url is required when store = redis.
#[test]
fn redis_store_without_url_rejected() {
    let toml = r#"
[server]
bind_address = "127.0.0.1:8080"
[security]
require_auth = false
[[api_keys]]
id = "k"
secret = "s"
[mail]
default_from = "a@example.com"
allowed_recipient_domains = ["example.com"]
[smtp]
host = "127.0.0.1"
port = 25
[status]
store = "redis"
"#;
    let result = http_smtp_rele::config::load_from_str(toml);
    assert!(result.is_err(), "redis without redis_url must be rejected");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("redis_url"), "error must mention redis_url: {msg}");
}

/// Non-redis build rejects store = redis.
#[cfg(not(feature = "redis"))]
#[test]
fn non_redis_build_rejects_redis_store() {
    let toml = r#"
[server]
bind_address = "127.0.0.1:8080"
[security]
require_auth = false
[[api_keys]]
id = "k"
secret = "s"
[mail]
default_from = "a@example.com"
allowed_recipient_domains = ["example.com"]
[smtp]
host = "127.0.0.1"
port = 25
[status]
store = "redis"
redis_url = "redis://127.0.0.1:6379/0"
"#;
    let result = http_smtp_rele::config::load_from_str(toml);
    assert!(result.is_err(), "redis store in non-redis build must be rejected");
    assert!(result.unwrap_err().to_string().contains("not available"));
}

/// Unknown store value is rejected.
#[test]
fn unknown_store_value_rejected() {
    let toml = r#"
[server]
bind_address = "127.0.0.1:8080"
[security]
require_auth = false
[[api_keys]]
id = "k"
secret = "s"
[mail]
default_from = "a@example.com"
allowed_recipient_domains = ["example.com"]
[smtp]
host = "127.0.0.1"
port = 25
[status]
store = "cassandra"
"#;
    let result = http_smtp_rele::config::load_from_str(toml);
    assert!(result.is_err(), "unknown store must be rejected");
}

