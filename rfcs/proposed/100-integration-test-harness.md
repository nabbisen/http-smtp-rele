# RFC 100 — Integration Test Harness

**Status.** Proposed  
**Tracks.** Testing  
**Touches.** `src/tests.rs` or `tests/` directory

## Summary

Define the integration test harness: how to spin up the Axum server in-process, configure
it with a test `AppConfig`, and send HTTP requests using `axum-test` or `reqwest`.

## Motivation

Unit tests cover pure logic. Integration tests verify that the entire request pipeline —
middleware ordering, extractor chains, error mapping — behaves correctly when assembled
(NFR-MNT-003).

## Scope

- Test helpers: `make_test_app(config)`, `make_test_state(config)`.
- In-process test server using `axum::serve` with a bound port.
- Standard test config with a valid API key, localhost bind, fake SMTP port.
- Shared test fixtures: valid `MailRequest`, invalid variants.

## Non-goals

- Fake SMTP server (RFC 101).
- Security regression tests (RFC 102).

## Design

### Test app builder

```rust
// In tests/common.rs or src/tests.rs

pub fn test_config() -> AppConfig {
    AppConfig {
        server: ServerConfig {
            bind_address: "127.0.0.1:0".into(),  // OS assigns port
            ..Default::default()
        },
        api_keys: vec![ApiKeyConfig {
            key_id: "test-key".into(),
            secret: SecretString::new("test-secret".into()),
            enabled: true,
            ..Default::default()
        }],
        smtp: SmtpConfig {
            host: "127.0.0.1".into(),
            port: 2525,  // fake SMTP port (RFC 101)
            ..Default::default()
        },
        ..AppConfig::default()
    }
}

pub async fn make_test_server() -> (SocketAddr, impl Future<Output = ()>) {
    let config = Arc::new(test_config());
    let state = AppState::build(config).await.unwrap();
    let app = crate::app::build_router(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = axum::serve(listener, app).into_future();
    (addr, server)
}
```

### Test request helper

```rust
pub fn valid_send_request() -> serde_json::Value {
    serde_json::json!({
        "to": "user@example.com",
        "subject": "Test subject",
        "body": "Test body"
    })
}

pub fn auth_header(secret: &str) -> (&'static str, String) {
    ("Authorization", format!("Bearer {secret}"))
}
```

### Test pattern

```rust
#[tokio::test]
async fn test_send_valid_request_returns_202() {
    let (addr, server) = make_test_server().await;
    tokio::spawn(server);

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/v1/send"))
        .header("Authorization", "Bearer test-secret")
        .json(&valid_send_request())
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 202);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "accepted");
}
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-100-01 | `make_test_server()` spins up a full Axum app on a random port. |
| AC-100-02 | Integration tests use the harness without custom networking setup. |
| AC-100-03 | Test config uses port 0 for HTTP binding (OS-assigned). |

## Open Questions

None.
