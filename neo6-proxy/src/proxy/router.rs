use axum::{Router, routing::{get, post}};
use super::handler::{invoke_handler, invoke_async_handler, status_handler, health_handler, metrics_handler};
use tracing::debug;

pub fn create_router() -> Router {
    debug!("Creando router de endpoints Axum");
    Router::new()
        .route("/invoke", post(invoke_handler))
        .route("/invoke-async", post(invoke_async_handler))
        .route("/status/:id", get(status_handler))
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
}