//! Logging initialization using `tracing` and `tracing-subscriber`.

use crate::config::LoggingConfig;

/// Initialize the global tracing subscriber.
///
/// Log level is controlled by the `RUST_LOG` environment variable, falling
/// back to `config.level`. Format is controlled by `config.format`:
/// - `"json"` — structured JSON output for production log aggregators.
/// - `"text"` (default) — human-readable output for development.
///
/// Must be called once at startup before any `tracing::*` calls.
pub fn init_logging(config: &LoggingConfig) {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.level));

    match config.format.as_str() {
        "json" => {
            tracing_subscriber::fmt()
                .json()
                .with_env_filter(env_filter)
                .init();
        }
        _ => {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .init();
        }
    }
}
