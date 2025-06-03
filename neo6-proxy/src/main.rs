use axum::{routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber;

mod proxy;
mod logging;

use proxy::router::create_router;

#[tokio::main]
async fn main() {
    // Initialize logging and tracing
    logging::init_logging();

    // Build the Axum app with all routes
    let app = create_router();

    // Define the address to bind to
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));

    // Bind a TcpListener for axum 0.7
    let listener = TcpListener::bind(addr).await.unwrap();
    tracing::info!("Starting neo6-proxy on {}", addr);

    // Start the server (Axum 0.7)
    axum::serve(listener, app).await.unwrap();
}