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
//! 8. Spawn SIGHUP config-reload handler (RFC 305)
//! 9. Log `app.started`
//! 10. Serve

use std::{path::PathBuf, sync::Arc};

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

    // 7. Apply runtime security restrictions based on smtp.mode (RFC 304)
    let runtime_mode = if config.smtp.mode == "pipe" {
        security::RuntimeMode::SendmailPipe {
            pipe_command: config.smtp.pipe_command.clone(),
        }
    } else {
        security::RuntimeMode::SmtpRelay
    };
    if let Err(e) = security::apply_runtime_restrictions(runtime_mode) {
        tracing::error!(error = %e, "runtime security restriction failed");
        std::process::exit(1);
    }

    // 8. SIGHUP config-reload handler (RFC 305).
    //
    // On SIGHUP: reload config from the same path; if valid, swap atomically.
    // Invalid config is logged and the current config is kept.
    //
    // Note: pledge("stdio inet") on OpenBSD excludes rpath, so SIGHUP reload
    // cannot re-read the config file after pledge is applied.  The handler is
    // installed but will fail silently on OpenBSD; it is primarily for Linux.
    #[cfg(unix)]
    {
        let reload_state = Arc::clone(&state);
        let reload_path  = config_path.clone();
        tokio::spawn(async move {
            use tokio::signal::unix::{signal, SignalKind};
            let mut stream = match signal(SignalKind::hangup()) {
                Ok(s)  => s,
                Err(e) => {
                    tracing::warn!(error = %e, "SIGHUP handler could not be registered");
                    return;
                }
            };
            loop {
                stream.recv().await;
                tracing::info!(event = "sighup_received", "reloading config");
                match config::load(&reload_path) {
                    Ok(new_cfg) => {
                        reload_state.reload_config(new_cfg);
                        tracing::info!(event = "config_reloaded", "config reloaded successfully");
                    }
                    Err(e) => {
                        tracing::error!(
                            event = "config_reload_failed",
                            error = %e,
                            "SIGHUP reload failed; keeping current config"
                        );
                    }
                }
            }
        });
    }

    // 9. Log startup
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        bind_address = %bind_addr,
        "app.started"
    );

    // 10. Serve
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
