use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

/// Initializes logging and tracing for the application.
pub fn init_logging() {
    // Set up logging with environment filter (RUST_LOG) and pretty output
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(fmt::layer().pretty())
        .init();
}

/// Initializes logging and tracing for the application with a given log level.
pub fn init_logging_with_level(log_level: &str) {
    let filter = EnvFilter::try_new(log_level).unwrap_or_else(|_| EnvFilter::new("info"));
    
    // Check if we're running in a terminal (TTY) to decide on formatting
    let is_terminal = atty::is(atty::Stream::Stdout);
    
    if is_terminal {
        // Use colored output for terminal
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().pretty())
            .init();
    } else {
        // Use plain output for files/non-terminal
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_ansi(false).compact())
            .init();
    }
}