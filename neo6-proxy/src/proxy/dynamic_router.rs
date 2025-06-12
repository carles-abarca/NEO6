// Dynamic router that uses dynamic protocol loading
use axum::{
    routing::{get, post},
    Router,
};
use crate::proxy::dynamic_handler;

pub fn create_dynamic_router() -> Router {
    Router::new()
        // Core transaction endpoints
        .route("/invoke", post(dynamic_handler::invoke_transaction))
        .route("/invoke-async", post(dynamic_handler::invoke_transaction_async))
        .route("/status/:id", get(dynamic_handler::check_transaction_status))
        
        // Protocol management endpoints
        .route("/protocols", get(dynamic_handler::list_protocols))
        .route("/protocols/:protocol/load", post(dynamic_handler::load_protocol))
        .route("/protocols/:protocol/unload", post(dynamic_handler::unload_protocol))
        
        // Health check
        .route("/health", get(|| async { "OK" }))
}
