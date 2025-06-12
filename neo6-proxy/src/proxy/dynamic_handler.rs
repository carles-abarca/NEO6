// Dynamic handler module for all proxy endpoints using dynamic loading
use axum::{extract::Path, Json};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tracing::{error, info, debug};
use crate::cics::mapping::TransactionMap;
use crate::protocol_loader::ProtocolLoader;

// Request and response types for /invoke and /invoke-async endpoints
#[derive(Debug, Deserialize)]
pub struct InvokeRequest {
    pub transaction_id: String, // Transaction identifier (e.g. "TX01")
    pub parameters: serde_json::Value, // Arbitrary parameters for the transaction
}

#[derive(Serialize)]
pub struct InvokeResponse {
    pub status: String, // "ok", "error", "pending", etc.
    pub result: Option<serde_json::Value>, // Result or error details
    pub id: Option<String>, // Transaction execution id (for async/status)
}

// Singleton for transaction map loaded from config/transactions.yaml
static TRANSACTION_MAP: OnceLock<TransactionMap> = OnceLock::new();

// Singleton for protocol loader
static PROTOCOL_LOADER: OnceLock<ProtocolLoader> = OnceLock::new();

// Returns a reference to the loaded transaction map (loads once on first call)
fn get_transaction_map() -> &'static TransactionMap {
    TRANSACTION_MAP.get_or_init(|| {
        crate::cics::mapping::load_transaction_map("config/transactions.yaml").expect("Failed to load transaction map")
    })
}

// Returns a reference to the protocol loader (loads once on first call)
fn get_protocol_loader() -> &'static ProtocolLoader {
    PROTOCOL_LOADER.get_or_init(|| {
        ProtocolLoader::new()
    })
}

// Synchronous /invoke endpoint - invokes a transaction and returns the result
pub async fn invoke_transaction(Json(req): Json<InvokeRequest>) -> Json<InvokeResponse> {
    info!("Invoking transaction: {}", req.transaction_id);
    debug!("Parameters: {:?}", req.parameters);

    let transaction_map = get_transaction_map();
    let protocol_loader = get_protocol_loader();

    // Look up the transaction configuration
    let tx_config = match transaction_map.get(&req.transaction_id) {
        Some(config) => config,
        None => {
            error!("Transaction not found: {}", req.transaction_id);
            return Json(InvokeResponse {
                status: "error".to_string(),
                result: Some(serde_json::json!({"error": format!("Transaction not found: {}", req.transaction_id)})),
                id: None,
            });
        }
    };

    // Load the protocol handler dynamically
    let protocol_handler = match protocol_loader.load_protocol(&tx_config.protocol) {
        Ok(handler) => handler,
        Err(e) => {
            error!("Failed to load protocol {}: {}", tx_config.protocol, e);
            return Json(InvokeResponse {
                status: "error".to_string(),
                result: Some(serde_json::json!({"error": format!("Failed to load protocol: {}", e)})),
                id: None,
            });
        }
    };

    // Invoke the transaction
    match protocol_handler.invoke_transaction(&req.transaction_id, req.parameters) {
        Ok(result) => {
            info!("Transaction {} completed successfully", req.transaction_id);
            Json(InvokeResponse {
                status: "ok".to_string(),
                result: Some(result),
                id: None,
            })
        }
        Err(e) => {
            error!("Transaction {} failed: {}", req.transaction_id, e);
            Json(InvokeResponse {
                status: "error".to_string(),
                result: Some(serde_json::json!({"error": e})),
                id: None,
            })
        }
    }
}

// Asynchronous /invoke-async endpoint - starts a transaction and returns an execution id
pub async fn invoke_transaction_async(Json(req): Json<InvokeRequest>) -> Json<InvokeResponse> {  
    info!("Starting async transaction: {}", req.transaction_id);
    
    // For now, we'll just call the synchronous version
    // In a real implementation, you'd want to spawn a task and return immediately
    invoke_transaction(Json(req)).await
}

// /status endpoint - checks the status of an asynchronous transaction
pub async fn check_transaction_status(Path(id): Path<String>) -> Json<InvokeResponse> {
    // In a real implementation, you'd look up the transaction status by ID
    info!("Checking status for transaction ID: {}", id);
    
    Json(InvokeResponse {
        status: "completed".to_string(),
        result: Some(serde_json::json!({"message": "Transaction completed"})),
        id: Some(id),
    })
}

// /protocols endpoint - lists loaded protocols
pub async fn list_protocols() -> Json<serde_json::Value> {
    let protocol_loader = get_protocol_loader();
    let loaded_protocols = protocol_loader.list_loaded_protocols();
    
    Json(serde_json::json!({
        "loaded_protocols": loaded_protocols,
        "available_protocols": ["tn3270", "lu62", "mq", "rest", "tcp", "jca"]
    }))
}

// /protocols/{protocol}/load endpoint - loads a specific protocol
pub async fn load_protocol(Path(protocol): Path<String>) -> Json<serde_json::Value> {
    let protocol_loader = get_protocol_loader();
    
    match protocol_loader.load_protocol(&protocol) {
        Ok(_) => {
            info!("Successfully loaded protocol: {}", protocol);
            Json(serde_json::json!({
                "status": "ok",
                "message": format!("Protocol {} loaded successfully", protocol)
            }))
        }
        Err(e) => {
            error!("Failed to load protocol {}: {}", protocol, e);
            Json(serde_json::json!({
                "status": "error",
                "message": format!("Failed to load protocol {}: {}", protocol, e)
            }))
        }
    }
}

// /protocols/{protocol}/unload endpoint - unloads a specific protocol
pub async fn unload_protocol(Path(protocol): Path<String>) -> Json<serde_json::Value> {
    let protocol_loader = get_protocol_loader();
    
    if protocol_loader.unload_protocol(&protocol) {
        info!("Successfully unloaded protocol: {}", protocol);
        Json(serde_json::json!({
            "status": "ok",
            "message": format!("Protocol {} unloaded successfully", protocol)
        }))
    } else {
        Json(serde_json::json!({
            "status": "error",
            "message": format!("Protocol {} was not loaded", protocol)
        }))
    }
}
