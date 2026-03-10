//! Logging configuration for the CLI.

use tracing_subscriber::{fmt, EnvFilter};

/// Initialize the logging system.
///
/// Uses the `CLEANSER_LOG` environment variable to control log levels.
/// Default level is `warn` for release builds and `info` for debug builds.
///
/// Examples:
/// - `CLEANSER_LOG=debug` - Enable debug logging
/// - `CLEANSER_LOG=cleanser_core=debug` - Enable debug only for core
/// - `CLEANSER_LOG=trace` - Enable trace logging (very verbose)
pub fn init() {
    let default_level = if cfg!(debug_assertions) {
        "info"
    } else {
        "warn"
    };

    let filter =
        EnvFilter::try_from_env("CLEANSER_LOG").unwrap_or_else(|_| EnvFilter::new(default_level));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_level(true)
        .without_time()
        .init();
}
