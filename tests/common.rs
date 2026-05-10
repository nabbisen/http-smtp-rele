//! Integration test harness for http-smtp-rele.
//!
//! Implements RFC 100: provides helpers for building a fully-assembled test
//! router, constructing requests, and parsing responses without a real TCP
//! connection (using `tower::ServiceExt::oneshot`).
//!
//! Tests that need a real SMTP stub should use `SmtpStub` from `smtp_stub`.

use axum::{body::Body, http::{header, Request, StatusCode}};
use serde_json::Value;
use tower::ServiceExt;

use http_smtp_rele::{
    api,
    config::{
        ApiKeyConfig, AppConfig, LoggingConfig, MailConfig, RateLimitConfig, SecretString,
        SecurityConfig, ServerConfig, SmtpConfig,
    },
    AppState,
};

// ---------------------------------------------------------------------------
// Standard test config
// ---------------------------------------------------------------------------

/// Build a standard AppConfig for integration tests.
///
/// SMTP port is `smtp_port` — set to a real `SmtpStub` port for E2E tests,
/// or to `1` for tests that only exercise layers before SMTP.
pub fn test_config(smtp_port: u16) -> AppConfig {
    AppConfig {
        server: ServerConfig {
            bind_address: "127.0.0.1:0".into(),
            max_request_body_bytes: 65536,
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
                    id: "primary".into(),
                    secret: SecretString::new("primary-secret"),
                    enabled: true,
                    description: None,
                    allowed_recipient_domains: vec!["example.com".into()],
                    rate_limit_per_min: None,
                    allowed_recipients: vec![],
                    burst: 0,
                },
                ApiKeyConfig {
                    id: "hi-rate".into(),
                    secret: SecretString::new("hirate-secret"),
                    enabled: true,
                    description: None,
                    allowed_recipient_domains: vec!["example.com".into()],
                    rate_limit_per_min: Some(600),
                    allowed_recipients: vec![],
                    burst: 0,
                },
                ApiKeyConfig {
                    id: "disabled".into(),
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
            default_from_name: Some("Relay".into()),
            allowed_recipient_domains: vec!["example.com".into()],
            max_subject_chars: 255,
            max_body_bytes: 65536,
        },
        smtp: SmtpConfig {
            mode: "smtp".into(),
            host: "127.0.0.1".into(),
            port: smtp_port,
            connect_timeout_seconds: 2,
            submission_timeout_seconds: 5,
        },
        rate_limit: RateLimitConfig {
            global_per_min: 600,
            per_ip_per_min: 300,
            per_key_per_min: 300,
            global_burst: 50,
            per_ip_burst: 50,
            per_key_burst: 50,
            burst_size: 0,
            ip_table_size: 1000,
        },
        logging: LoggingConfig {
            format: "text".into(),
            level: "error".into(), // suppress tracing output during tests
            mask_recipient: false,
        },
    }
}

/// Build a router for tests where SMTP will always fail (no listener on port 1).
pub fn test_router_no_smtp() -> axum::Router {
    let state = AppState::new(test_config(1));
    api::build_router(state)
}

/// Build a router for tests using the given SMTP stub port.
pub fn test_router(smtp_port: u16) -> axum::Router {
    let state = AppState::new(test_config(smtp_port));
    api::build_router(state)
}

// ---------------------------------------------------------------------------
// Request builders
// ---------------------------------------------------------------------------

pub struct RequestBuilder {
    method: String,
    uri: String,
    auth: Option<String>,
    content_type: Option<String>,
    body: Vec<u8>,
}

impl RequestBuilder {
    pub fn post(uri: &str) -> Self {
        Self {
            method: "POST".into(),
            uri: uri.into(),
            auth: None,
            content_type: Some("application/json".into()),
            body: vec![],
        }
    }

    pub fn get(uri: &str) -> Self {
        Self {
            method: "GET".into(),
            uri: uri.into(),
            auth: None,
            content_type: None,
            body: vec![],
        }
    }

    pub fn bearer(mut self, token: &str) -> Self {
        self.auth = Some(format!("Bearer {token}"));
        self
    }

    pub fn no_auth(mut self) -> Self {
        self.auth = None;
        self
    }

    pub fn json(mut self, body: impl serde::Serialize) -> Self {
        self.body = serde_json::to_vec(&body).unwrap();
        self
    }

    pub fn raw_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    pub fn content_type(mut self, ct: &str) -> Self {
        self.content_type = Some(ct.into());
        self
    }

    pub fn build(self) -> Request<Body> {
        let mut b = Request::builder()
            .method(self.method.as_str())
            .uri(&self.uri);
        if let Some(auth) = self.auth {
            b = b.header(header::AUTHORIZATION, auth);
        }
        if let Some(ct) = self.content_type {
            b = b.header(header::CONTENT_TYPE, ct);
        }
        b.body(Body::from(self.body)).unwrap()
    }
}

// ---------------------------------------------------------------------------
// Response helpers
// ---------------------------------------------------------------------------

pub struct TestResponse {
    pub status: StatusCode,
    pub body: Value,
}

impl TestResponse {
    pub fn assert_status(&self, expected: StatusCode) -> &Self {
        assert_eq!(
            self.status, expected,
            "expected {expected}, got {}\nbody: {}",
            self.status, self.body
        );
        self
    }

    pub fn assert_code(&self, code: &str) -> &Self {
        assert_eq!(
            self.body["code"].as_str().unwrap_or(""),
            code,
            "expected code={code:?}\nbody: {}",
            self.body
        );
        self
    }

    pub fn assert_status_field(&self, value: &str) -> &Self {
        assert_eq!(
            self.body["status"].as_str().unwrap_or(""),
            value,
            "expected status={value:?}\nbody: {}",
            self.body
        );
        self
    }

}

/// Send a request through the router and parse the response.
pub async fn send(router: &axum::Router, req: Request<Body>) -> TestResponse {
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 65536)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::json!({}));
    TestResponse { status, body }
}

// ---------------------------------------------------------------------------
// Standard fixtures
// ---------------------------------------------------------------------------

pub fn valid_mail_body() -> serde_json::Value {
    serde_json::json!({
        "to": "user@example.com",
        "subject": "Integration test",
        "body": "This is a test message."
    })
}

/// POST /v1/send with the primary test key and a valid body.
pub async fn send_valid(router: &axum::Router) -> TestResponse {
    send(
        router,
        RequestBuilder::post("/v1/send")
            .bearer("primary-secret")
            .json(valid_mail_body())
            .build(),
    )
    .await
}
