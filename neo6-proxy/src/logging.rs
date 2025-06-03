use tracing_subscriber::{fmt, EnvFilter};

/// Initializes logging and tracing for the application.
pub fn init_logging() {
    // Set up logging with environment filter (RUST_LOG) and pretty output
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(fmt::layer().pretty())
        .init();
}