use std::path::PathBuf;
use clap::Parser;
use tracing::info;
use tracing_subscriber;

mod config;
mod proxy_manager;
mod web_server;

use config::AdminConfig;
use proxy_manager::ProxyManager;
use web_server::WebServer;

#[derive(Parser)]
#[command(name = "neo6-admin")]
#[command(about = "NEO6 Proxy Administration Server")]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config/admin.yaml")]
    config: PathBuf,
    
    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,
    
    /// Start all auto-start proxy instances
    #[arg(long)]
    start_proxies: bool,
    
    /// Stop all managed proxy instances and exit
    #[arg(long)]
    stop_all: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Initialize logging with appropriate formatting
    let is_terminal = atty::is(atty::Stream::Stdout);
    
    tracing_subscriber::fmt()
        .with_env_filter(&args.log_level)
        .with_ansi(is_terminal)  // Only use colors when connected to a terminal
        .with_target(true)       // Include the target (module name)
        .with_thread_ids(false)  // Don't include thread IDs for cleaner logs
        .with_thread_names(false) // Don't include thread names
        .init();
    
    info!("Starting NEO6 Admin Server");
    
    // Load configuration
    let config = AdminConfig::load(&args.config)?;
    info!("Loaded configuration from {:?}", args.config);
    
    // Create proxy manager
    let mut proxy_manager = ProxyManager::new(config.proxy_instances.clone(), config.proxy_defaults.clone());
    
    // Handle stop-all command
    if args.stop_all {
        info!("Stopping all managed proxy instances...");
        proxy_manager.stop_all().await?;
        info!("All proxy instances stopped. Exiting.");
        return Ok(());
    }
    
    // Start auto-start proxy instances if requested
    if args.start_proxies {
        info!("Starting auto-start proxy instances...");
        proxy_manager.start_auto_start_instances().await?;
    }
    
    // Create and start web server
    let web_server = WebServer::new(config.admin.clone(), proxy_manager);
    
    info!("Starting admin web server on {}:{}", 
          config.admin.bind_address, config.admin.port);
    
    web_server.start().await?;
    
    Ok(())
}
