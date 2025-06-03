use axum::{routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use std::env;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber;
use tracing::{info, debug};

use neo6_proxy::config::ProxyConfig;
use neo6_proxy::proxy::router::create_router;
use neo6_proxy::logging::{self, init_logging_with_level};

#[tokio::main]
async fn main() {
    // Cargar configuración
    let mut config = ProxyConfig::from_default();
    // Permitir override por CLI
    if let Some(arg_level) = env::args().skip(1).find_map(|arg| {
        if arg.starts_with("--log-level=") {
            Some(arg.replace("--log-level=", ""))
        } else {
            None
        }
    }) {
        config.log_level = arg_level;
    }
    // Inicializar logging con el nivel configurado
    init_logging_with_level(&config.log_level);
    info!(level = %config.log_level, "Inicializando neo6-proxy con nivel de log");

    // Build the Axum app with all routes
    let app = create_router();
    debug!("App Axum construida, iniciando servidor");

    // Define the address to bind to
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));

    // Bind a TcpListener for axum 0.7
    let listener = TcpListener::bind(addr).await.unwrap();
    tracing::info!("Starting neo6-proxy on {}", addr);

    // Start the server (Axum 0.7)
    axum::serve(listener, app).await.unwrap();
}