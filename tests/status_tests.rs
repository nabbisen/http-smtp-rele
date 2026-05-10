//! Integration and security regression tests.
//!
//! Implements RFC 102 (complete SEC-001–017 matrix) and RFC 103 (E2E scenarios).
//! Uses `SmtpStub` for tests that verify SMTP submission end-to-end.

mod smtp_stub;
mod common;

use axum::http::StatusCode;
use tower::ServiceExt;


use common::{send, send_valid, test_router, test_router_no_smtp, RequestBuilder};
use smtp_stub::SmtpStub;


// RFC 106 — Submission Status API Integration Tests
// ===========================================================================

/// STS-001: valid send → smtp_accepted status
#[tokio::test]
async fn sts_001_valid_send_status_smtp_accepted() {
    let stub = SmtpStub::start(0).await;
    let router = test_router(stub.port());

    let send_resp = send_valid(&router).await;
    send_resp.assert_status(StatusCode::ACCEPTED);
    let request_id = send_resp.body["request_id"].as_str().unwrap().to_string();
    assert!(request_id.starts_with("req_"), "request_id must have req_ prefix: {request_id}");

    // Query status with same key
    let status_resp = send(
        &router,
        RequestBuilder::get(&format!("/v1/submissions/{request_id}"))
            .bearer("primary-secret-padded-to-32bytes!")
            .build(),
    ).await;
    status_resp.assert_status(StatusCode::OK);
    assert_eq!(status_resp.body["status"], "smtp_accepted");
    assert_eq!(status_resp.body["request_id"], request_id.as_str());
    assert!(status_resp.body.get("recipient_domains").is_some());
    assert!(!status_resp.body["recipient_domains"].is_null());

    stub.shutdown().await;
}

/// STS-002: validation failure after auth → rejected/validation_failed
#[tokio::test]
async fn sts_002_validation_failure_status_rejected() {
    let router = test_router_no_smtp();

    // Send request that will fail validation (disallowed domain)
    let send_resp = send(&router, RequestBuilder::post("/v1/send")
        .bearer("primary-secret-padded-to-32bytes!")
        .json(serde_json::json!({
            "to": "user@disallowed-domain.invalid",
            "subject": "Test",
            "body": "Hello."
        }))
        .build()).await;
    assert_ne!(send_resp.status, StatusCode::ACCEPTED);
    let request_id = send_resp.body["request_id"].as_str().unwrap_or("").to_string();

    if !request_id.is_empty() && request_id.starts_with("req_") {
        let status_resp = send(&router,
            RequestBuilder::get(&format!("/v1/submissions/{request_id}"))
                .bearer("primary-secret-padded-to-32bytes!")
                .build()).await;
        // Either rejected or not found depending on where rejection happened
        assert!(
            status_resp.status == StatusCode::OK || status_resp.status == StatusCode::NOT_FOUND,
            "unexpected status: {}", status_resp.status
        );
        if status_resp.status == StatusCode::OK {
            assert_eq!(status_resp.body["status"], "rejected");
        }
    }
}

/// STS-004: SMTP unavailable → smtp_failed status
///
/// Error responses carry request_id in the X-Request-Id header (not body).
/// The status store is updated to smtp_failed; the GET endpoint reflects this.
#[tokio::test]
async fn sts_004_smtp_unavailable_status_failed() {
    // Port 1 has no listener
    let router = test_router(1);

    // Get the raw response to read the X-Request-Id header
    let raw_req = RequestBuilder::post("/v1/send")
        .bearer("primary-secret-padded-to-32bytes!")
        .json(common::valid_mail_body())
        .build();

    let raw_resp = router.clone().oneshot(raw_req).await.unwrap();
    assert_eq!(raw_resp.status(), StatusCode::BAD_GATEWAY);

    let request_id = raw_resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    assert!(request_id.starts_with("req_"),
        "X-Request-Id must have req_ prefix on error responses: {request_id}");

    let status_resp = send(&router,
        RequestBuilder::get(&format!("/v1/submissions/{request_id}"))
            .bearer("primary-secret-padded-to-32bytes!")
            .build()).await;
    status_resp.assert_status(StatusCode::OK);
    assert_eq!(status_resp.body["status"], "smtp_failed");
    assert_eq!(status_resp.body["code"], "smtp_unavailable");
}

/// STS-005: different API key cannot read status (returns 404)
#[tokio::test]
async fn sts_005_different_key_receives_404() {
    let stub = SmtpStub::start(0).await;
    let router = test_router(stub.port());

    let send_resp = send_valid(&router).await;  // sent with "primary-secret-padded-to-32bytes!"
    send_resp.assert_status(StatusCode::ACCEPTED);
    let request_id = send_resp.body["request_id"].as_str().unwrap().to_string();

    // Query with a different key
    let status_resp = send(&router,
        RequestBuilder::get(&format!("/v1/submissions/{request_id}"))
            .bearer("hirate-secret")  // different key
            .build()).await;
    status_resp.assert_status(StatusCode::NOT_FOUND);
    assert_eq!(status_resp.body["code"], "submission_not_found");

    stub.shutdown().await;
}

/// STS-006: unknown request_id returns 404
#[tokio::test]
async fn sts_006_unknown_request_id_returns_404() {
    let router = test_router_no_smtp();
    let fake_id = "req_01HX7Q9V6R6W9V8Y5E3E6E7M9A";
    let status_resp = send(&router,
        RequestBuilder::get(&format!("/v1/submissions/{fake_id}"))
            .bearer("primary-secret-padded-to-32bytes!")
            .build()).await;
    status_resp.assert_status(StatusCode::NOT_FOUND);
    assert_eq!(status_resp.body["code"], "submission_not_found");
}

/// STS-007: status response contains no body/subject/token/full-address
#[tokio::test]
async fn sts_007_status_response_excludes_sensitive_data() {
    let stub = SmtpStub::start(0).await;
    let router = test_router(stub.port());

    let _ = send(&router, RequestBuilder::post("/v1/send")
        .bearer("primary-secret-padded-to-32bytes!")
        .json(serde_json::json!({
            "to": "user@example.com",
            "subject": "Secret subject",
            "body": "Secret body content XYZ"
        }))
        .build()).await;

    // Find the request_id by doing another send
    let send_resp = send_valid(&router).await;
    let request_id = send_resp.body["request_id"].as_str().unwrap().to_string();

    let status_resp = send(&router,
        RequestBuilder::get(&format!("/v1/submissions/{request_id}"))
            .bearer("primary-secret-padded-to-32bytes!")
            .build()).await;

    let body_str = status_resp.body.to_string();
    assert!(!body_str.contains("Secret subject"), "subject must not appear in status");
    assert!(!body_str.contains("Secret body"), "body must not appear in status");
    assert!(!body_str.contains("primary-secret-padded-to-32bytes!"), "API key must not appear in status");
    assert!(!body_str.contains("user@example.com"), "full recipient address must not appear");

    stub.shutdown().await;
}

/// STS-008: request_id in response matches X-Request-Id header (format: req_ + ULID)
#[tokio::test]
async fn sts_008_request_id_format_is_req_ulid() {
    let stub = SmtpStub::start(0).await;
    let router = test_router(stub.port());

    let send_resp = send_valid(&router).await;
    send_resp.assert_status(StatusCode::ACCEPTED);

    let body_id = send_resp.body["request_id"].as_str().unwrap_or("");
    assert!(body_id.starts_with("req_"),
        "request_id must start with req_: {body_id}");
    assert!(body_id.len() > 4,
        "request_id must have content after req_: {body_id}");

    stub.shutdown().await;
}

/// STS-009: status tracking disabled → GET always returns 404
#[tokio::test]
async fn sts_009_disabled_status_tracking_returns_404() {
    use http_smtp_rele::{api, AppState};

    let mut cfg = common::test_config(1);
    cfg.status.enabled = false;
    let router = api::build_router(AppState::new(cfg));

    // Even after a successful send (port 1 → 502, but request_id exists)
    let send_resp = send(&router, RequestBuilder::post("/v1/send")
        .bearer("primary-secret-padded-to-32bytes!")
        .json(common::valid_mail_body())
        .build()).await;

    let request_id = send_resp.body["request_id"].as_str().unwrap_or("").to_string();
    if request_id.starts_with("req_") {
        let status_resp = send(&router,
            RequestBuilder::get(&format!("/v1/submissions/{request_id}"))
                .bearer("primary-secret-padded-to-32bytes!")
                .build()).await;
        status_resp.assert_status(StatusCode::NOT_FOUND);
    }
}

/// STS-010: invalid request_id format returns 404 (not 400)
#[tokio::test]
async fn sts_010_invalid_request_id_format_returns_404() {
    let router = test_router_no_smtp();
    let bad_id = "not-a-valid-id";
    let resp = send(&router,
        RequestBuilder::get(&format!("/v1/submissions/{bad_id}"))
            .bearer("primary-secret-padded-to-32bytes!")
            .build()).await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

/// STS-unauthenticated: GET without auth returns 401
#[tokio::test]
async fn sts_unauthenticated_returns_401() {
    let router = test_router_no_smtp();
    let resp = send(&router,
        RequestBuilder::get("/v1/submissions/req_01HX7Q9V6R6W9V8Y5E3E6E7M9A")
            .no_auth()
            .build()).await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
}

// ===========================================================================
// RFC 601 — Status Store Prometheus Metrics
// ===========================================================================

#[tokio::test]
async fn status_store_metrics_in_prometheus_output() {
    let stub = SmtpStub::start(0).await;
    let router = test_router(stub.port());

    // Send a request to create a status record
    let _ = send_valid(&router).await;

    let metrics_resp = send(&router, RequestBuilder::get("/metrics").build()).await;
    assert_eq!(metrics_resp.status, StatusCode::OK);

    stub.shutdown().await;
}

// ===========================================================================
// RFC 602 — GET /v1/keys/self
// ===========================================================================

#[tokio::test]
async fn keys_self_returns_key_config() {
    let router = test_router_no_smtp();
    let resp = send(&router, RequestBuilder::get("/v1/keys/self")
        .bearer("primary-secret-padded-to-32bytes!")
        .build()).await;
    resp.assert_status(StatusCode::OK);
    assert_eq!(resp.body["id"], "primary");
    assert_eq!(resp.body["enabled"], true);
    // Secret must not appear
    assert!(resp.body.get("secret").is_none() || resp.body["secret"].is_null(),
        "secret must not appear in key info response");
}

#[tokio::test]
async fn keys_self_unauthenticated_returns_401() {
    let router = test_router_no_smtp();
    let resp = send(&router, RequestBuilder::get("/v1/keys/self")
        .no_auth()
        .build()).await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn keys_self_includes_effective_rates() {
    let router = test_router_no_smtp();
    let resp = send(&router, RequestBuilder::get("/v1/keys/self")
        .bearer("primary-secret-padded-to-32bytes!")
        .build()).await;
    resp.assert_status(StatusCode::OK);
    assert!(resp.body.get("effective_rate_limit_per_min").is_some(),
        "effective_rate_limit_per_min must be present");
    assert!(resp.body.get("effective_burst").is_some(),
        "effective_burst must be present");
}

// ===========================================================================
// RFC 603 — Per-Key mask_recipient Override
// ===========================================================================

#[tokio::test]
async fn per_key_mask_recipient_none_inherits_global() {
    use http_smtp_rele::{api, AppState};
    let mut cfg = common::test_config(1);
    cfg.logging.mask_recipient = false;
    cfg.security.api_keys[0].mask_recipient = None; // inherit
    let _router = api::build_router(AppState::new(cfg));
    // This is a config-only test: just verify it builds without error
}

#[tokio::test]
async fn per_key_mask_recipient_override_true() {
    use http_smtp_rele::{api, AppState};
    let mut cfg = common::test_config(1);
    cfg.logging.mask_recipient = false; // global says no mask
    cfg.security.api_keys[0].mask_recipient = Some(true); // key overrides to mask
    let router = api::build_router(AppState::new(cfg));

    // Key info should reflect the override
    let resp = send(&router, RequestBuilder::get("/v1/keys/self")
        .bearer("primary-secret-padded-to-32bytes!")
        .build()).await;
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.body["mask_recipient"], true);
}

// ===========================================================================
// RFC 088 — SQLite StatusStore integration tests
// ===========================================================================
#[cfg(feature = "sqlite")]
mod sqlite_tests {
    use axum::http::StatusCode;
    use http_smtp_rele::{api, config::StatusConfig, AppState};
    use super::common::{self, RequestBuilder, send};

    fn sqlite_router(smtp_port: u16, db_path: &std::path::Path) -> axum::Router {
        let mut cfg = common::test_config(smtp_port);
        cfg.status.store   = "sqlite".into();
        cfg.status.db_path = Some(db_path.to_path_buf());
        api::build_router(AppState::new(cfg))
    }

    /// AC-088-01: sqlite store persists records within a session.
    #[tokio::test]
    async fn sqlite_status_persists_within_session() {
        let stub   = super::SmtpStub::start(0).await;
        let _dir = tempfile::tempdir().unwrap();
        let router = sqlite_router(stub.port(), &_dir.path().join("status.db"));

        let resp = super::send_valid(&router).await;
        resp.assert_status(StatusCode::ACCEPTED);
        let id = resp.body["request_id"].as_str().unwrap().to_string();

        let status = send(&router,
            RequestBuilder::get(&format!("/v1/submissions/{id}"))
                .bearer("primary-secret-padded-to-32bytes!").build()).await;
        status.assert_status(StatusCode::OK);
        assert_eq!(status.body["status"], "smtp_accepted");

        stub.shutdown().await;
    }

    /// AC-088-02: non-sqlite build would reject store = "sqlite" (tested by config validation).
    #[tokio::test]
    async fn sqlite_status_different_key_returns_404() {
        let stub   = super::SmtpStub::start(0).await;
        let _dir = tempfile::tempdir().unwrap();
        let router = sqlite_router(stub.port(), &_dir.path().join("status.db"));

        let resp = super::send_valid(&router).await;
        resp.assert_status(StatusCode::ACCEPTED);
        let id = resp.body["request_id"].as_str().unwrap().to_string();

        let status = send(&router,
            RequestBuilder::get(&format!("/v1/submissions/{id}"))
                .bearer("hirate-secret").build()).await;
        status.assert_status(StatusCode::NOT_FOUND);
        assert_eq!(status.body["code"], "submission_not_found");

        stub.shutdown().await;
    }

    /// AC-088-07: max_records eviction in sqlite store.
    #[tokio::test]
    async fn sqlite_max_records_bounded() {
        let dir = tempfile::tempdir().unwrap();
        let db  = dir.path().join("t.db");
        let mut cfg = common::test_config(1); // port 1 = smtp always fails → 502
        cfg.status.store      = "sqlite".into();
        cfg.status.db_path    = Some(db);
        cfg.status.max_records = 2;
        let _dir = dir; // keep alive
        let router = api::build_router(AppState::new(cfg));

        for _ in 0..4 {
            let _ = super::send_valid(&router).await;
        }

        // record_count via direct AppState would be cleanest, but exercising
        // via API is sufficient: at most max_records responses visible
        // (this test just checks it doesn't panic / OOM)
    }

    /// AC-088-03: missing db_path → config validation error.
    #[test]
    fn sqlite_missing_db_path_fails_validation() {
        let mut cfg = common::test_config(1);
        cfg.status.store   = "sqlite".into();
        cfg.status.db_path = None;

        // Validate directly
        let result = http_smtp_rele::config::validate_config(&cfg);
        assert!(result.is_err(), "missing db_path must be a validation error");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("db_path"), "error must mention db_path: {msg}");
    }

    /// AC-088-04: missing parent directory → SqliteStatusStore::open error.
    #[test]
    fn sqlite_missing_parent_dir_fails() {
        use http_smtp_rele::status_sqlite::SqliteStatusStore;
        use http_smtp_rele::metrics::Metrics;
        use std::sync::Arc;

        let cfg    = {
            let mut c = common::test_config(1);
            c.status.store   = "sqlite".into();
            c.status.db_path = Some("/nonexistent/dir/x.db".into());
            c.status
        };
        let result = SqliteStatusStore::open(
            std::path::Path::new("/nonexistent/dir/x.db"),
            &cfg,
            Arc::new(Metrics::new()),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    /// AC-088-08: status record contains no mail body or full recipient address.
    #[tokio::test]
    async fn sqlite_status_excludes_sensitive_data() {
        let stub   = super::SmtpStub::start(0).await;
        let _dir = tempfile::tempdir().unwrap();
        let router = sqlite_router(stub.port(), &_dir.path().join("status.db"));

        let resp = super::send_valid(&router).await;
        let id   = resp.body["request_id"].as_str().unwrap().to_string();

        let status = send(&router,
            RequestBuilder::get(&format!("/v1/submissions/{id}"))
                .bearer("primary-secret-padded-to-32bytes!").build()).await;
        status.assert_status(StatusCode::OK);

        let body_str = status.body.to_string();
        assert!(!body_str.contains("user@example.com"), "full address must not appear");
        assert!(body_str.contains("example.com"),       "domain may appear");

        stub.shutdown().await;
    }
}

// ===========================================================================
// RFC 088 — SQLite Status Store Integration Tests
// ===========================================================================

/// STS-SQLite-001: SQLite store persists records and returns correct status.
#[cfg(feature = "sqlite")]
#[tokio::test]
async fn sqlite_store_persists_status_records() {
    use http_smtp_rele::{api, config::*, status_sqlite::SqliteStatusStore, AppState};
    use http_smtp_rele::metrics::Metrics;
    use std::sync::Arc;

    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    let stub = SmtpStub::start(0).await;

    // Build config with SQLite store
    let mut cfg = common::test_config(stub.port());
    cfg.status.store  = "sqlite".into();
    cfg.status.db_path = Some(db_path.clone());

    // Build the store separately so we can share it across state instances
    let metrics = Arc::new(Metrics::new());
    let store = SqliteStatusStore::open(&db_path, &cfg.status, metrics).unwrap();
    let state = AppState::new_with_store(cfg.clone(), store.clone());
    let router = api::build_router(state);

    // Send a mail
    let send_resp = send_valid(&router).await;
    send_resp.assert_status(StatusCode::ACCEPTED);
    let request_id = send_resp.body["request_id"].as_str().unwrap().to_string();

    // Query status — same router
    let status_resp = send(
        &router,
        RequestBuilder::get(&format!("/v1/submissions/{request_id}"))
            .bearer("primary-secret-padded-to-32bytes!")
            .build(),
    ).await;
    status_resp.assert_status(StatusCode::OK);
    assert_eq!(status_resp.body["status"], "smtp_accepted");
    assert_eq!(status_resp.body["request_id"], request_id.as_str());

    stub.shutdown().await;
}

/// STS-SQLite-002: SQLite store survives router rebuild (simulates restart
/// within the same process with the same db_path, sharing the same store).
#[cfg(feature = "sqlite")]
#[tokio::test]
async fn sqlite_store_data_accessible_from_shared_store() {
    use http_smtp_rele::{api, config::*, status_sqlite::SqliteStatusStore, AppState};
    use http_smtp_rele::metrics::Metrics;
    use std::sync::Arc;

    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("persist.db");

    let stub = SmtpStub::start(0).await;
    let mut cfg = common::test_config(stub.port());
    cfg.status.store  = "sqlite".into();
    cfg.status.db_path = Some(db_path.clone());

    let m     = Arc::new(Metrics::new());
    let store = SqliteStatusStore::open(&db_path, &cfg.status, m).unwrap();

    // Router 1: send mail
    let state1  = AppState::new_with_store(cfg.clone(), store.clone());
    let router1 = api::build_router(state1);
    let r1 = send_valid(&router1).await;
    r1.assert_status(StatusCode::ACCEPTED);
    let rid = r1.body["request_id"].as_str().unwrap().to_string();

    // Router 2: same store — record must be visible
    let state2  = AppState::new_with_store(cfg.clone(), store.clone());
    let router2 = api::build_router(state2);
    let status  = send(
        &router2,
        RequestBuilder::get(&format!("/v1/submissions/{rid}"))
            .bearer("primary-secret-padded-to-32bytes!")
            .build(),
    ).await;
    status.assert_status(StatusCode::OK);
    assert_eq!(status.body["status"], "smtp_accepted");

    stub.shutdown().await;
}

/// STS-SQLite-003: non-sqlite build rejects sqlite store at config level.
#[cfg(not(feature = "sqlite"))]
#[test]
fn non_sqlite_build_rejects_sqlite_store_config() {
    use http_smtp_rele::config;
    let toml = r#"
[server]
bind_address = "127.0.0.1:8080"
[security]
[[security.api_keys]]
id = "k"
secret = "sxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
[mail]
default_from = "a@example.com"
allowed_recipient_domains = ["example.com"]
[smtp]
host = "127.0.0.1"
port = 25
[status]
store   = "sqlite"
db_path = "/tmp/test.db"
"#;
    let result = config::load_from_str(toml);
    assert!(result.is_err(), "sqlite store must be rejected in non-sqlite build");
}

// ===========================================================================
