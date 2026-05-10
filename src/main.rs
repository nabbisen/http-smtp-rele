//! Application entry point.
//!
//! Startup sequence:
//! 1. Parse CLI args (`--config` path override)
//! 2. Apply initial security restrictions (OpenBSD: unveil + initial pledge)
//! 3. Load and validate TOML config
//! 4. Initialize tracing subscriber
//! 5. Build AppState
//! 6. Bind TCP listener
//! 7. Apply runtime security restrictions (OpenBSD: drop rpath)
//! 8. Log `app.started`
//! 9. Serve

use std::path::PathBuf;

use http_smtp_rele::{api, config, logging, security, AppState};

const DEFAULT_CONFIG_PATH: &str = "/etc/http-smtp-rele.toml";

#[tokio::main]
async fn main() {
    // 1. Parse CLI args
    let config_path = parse_config_path();

    // 2. Apply initial security restrictions (OpenBSD only; no-op elsewhere)
    if let Err(e) = security::apply_initial_restrictions(&config_path) {
        eprintln!("fatal: security restriction failed: {}", e);
        std::process::exit(1);
    }

    // 3. Load and validate config (fail-fast on error)
    let config = config::load(&config_path).unwrap_or_else(|e| {
        eprintln!("fatal: {}", e);
        std::process::exit(1);
    });

    // 4. Initialize tracing
    logging::init_logging(&config.logging);

    // 5. Build AppState
    let state = AppState::new(config.clone());

    // 6. Bind TCP listener
    let bind_addr = config.server.bind_address.clone();
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|e| {
            tracing::error!(address = %bind_addr, error = %e, "failed to bind");
            std::process::exit(1);
        });

    // 7. Apply runtime security restrictions
    if let Err(e) = security::apply_runtime_restrictions(security::RuntimeMode::SmtpRelay) {
        tracing::error!(error = %e, "runtime security restriction failed");
        std::process::exit(1);
    }

    // 8. Log startup
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        bind_address = %bind_addr,
        "app.started"
    );

    // 9. Serve
    let router = api::build_router(state);
    axum::serve(listener, router).await.unwrap_or_else(|e| {
        tracing::error!(error = %e, "server error");
        std::process::exit(1);
    });

    tracing::info!("app.stopped");
}

/// Extract `--config <path>` from CLI args, defaulting to the standard path.
fn parse_config_path() -> PathBuf {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--config" {
            if let Some(path) = args.get(i + 1) {
                return PathBuf::from(path);
            }
        }
        i += 1;
    }
    PathBuf::from(DEFAULT_CONFIG_PATH)
}
