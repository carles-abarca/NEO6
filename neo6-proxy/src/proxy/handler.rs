// Handler module for all proxy endpoints (REST API)
use axum::{extract::Path, Json};
use serde::{Deserialize, Serialize};
use crate::cics::mapping::{self, TransactionMap};
use std::sync::OnceLock;
use lu62::Lu62Handler;
use mq::MqHandler;
use rest::RestHandler;
use tcp::TcpHandler;
use jca::JcaHandler;
use neo6_protocols_lib::protocol::ProtocolHandler;
use tracing::{error, info, debug};

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

// Returns a reference to the loaded transaction map (loads once on first call)
fn get_transaction_map() -> &'static TransactionMap {
    TRANSACTION_MAP.get_or_init(|| {
        mapping::load_transaction_map("config/transactions.yaml").expect("Failed to load transaction map")
    })
}

// Returns a boxed protocol handler for the given protocol string, or None if unsupported
fn get_protocol_handler(protocol: &str) -> Option<Box<dyn ProtocolHandler>> {
    match protocol.to_lowercase().as_str() {
        "lu62" => Some(Box::new(Lu62Handler)),
        "mq" => Some(Box::new(MqHandler)),
        "rest" => Some(Box::new(RestHandler)),
        "tcp" => Some(Box::new(TcpHandler)),
        "jca" => Some(Box::new(JcaHandler)),
        _ => None,
    }
}

// POST /invoke: Synchronous transaction invocation endpoint
pub async fn invoke_handler(Json(payload): Json<InvokeRequest>) -> Json<InvokeResponse> {
    let map = get_transaction_map();
    debug!(?payload, "Recibida petición /invoke");
    // Look up transaction config by id
    if let Some(tx) = map.transactions.get(&payload.transaction_id) {
        info!(tx = %payload.transaction_id, protocol = %tx.protocol, "Ejecutando transacción");
        // Get protocol handler for this transaction
        if let Some(handler) = get_protocol_handler(&tx.protocol) {
            // Call protocol handler
            let result = handler.invoke_transaction(&payload.transaction_id, payload.parameters.clone());
            match result {
                Ok(res) => {
                    info!(tx = %payload.transaction_id, "Transacción ejecutada correctamente");
                    debug!(?res, "Respuesta de la transacción");
                    Json(InvokeResponse {
                        status: "ok".to_string(),
                        result: Some(res),
                        id: Some("tx123".to_string()), // TODO: generate real execution id
                    })
                },
                Err(e) => {
                    error!(tx = %payload.transaction_id, error = %e, "Error al ejecutar transacción");
                    Json(InvokeResponse {
                        status: "error".to_string(),
                        result: Some(serde_json::json!({"error": e})),
                        id: None,
                    })
                }
            }
        } else {
            // Protocol not supported
            error!(protocol = %tx.protocol, "Protocolo no soportado");
            Json(InvokeResponse {
                status: "error".to_string(),
                result: Some(serde_json::json!({"error": format!("Unsupported protocol: {}", tx.protocol)})),
                id: None,
            })
        }
    } else {
        // Unknown transaction id
        error!(tx = %payload.transaction_id, "transaction_id desconocido");
        Json(InvokeResponse {
            status: "error".to_string(),
            result: Some(serde_json::json!({"error": "Unknown transaction_id"})),
            id: None,
        })
    }
}

// POST /invoke-async: Asynchronous transaction invocation endpoint (returns immediately)
pub async fn invoke_async_handler(Json(payload): Json<InvokeRequest>) -> Json<InvokeResponse> {
    info!(?payload, "Invocación asíncrona recibida");
    // TODO: Start async invocation and return execution id
    Json(InvokeResponse {
        status: "pending".to_string(),
        result: None,
        id: Some("tx123".to_string()), // TODO: generate real execution id
    })
}

// GET /status/:id: Returns the status/result for a given async transaction id
pub async fn status_handler(Path(id): Path<String>) -> Json<InvokeResponse> {
    info!(id = %id, "Consulta de estado de transacción");
    // TODO: Look up real status/result for the given id
    Json(InvokeResponse {
        status: "completed".to_string(),
        result: Some(serde_json::json!({"message": "Result for ", "id": id})),
        id: Some(id),
    })
}

// GET /health: Basic health check endpoint
pub async fn health_handler() -> &'static str {
    debug!("Health check solicitado");
    "OK"
}

// GET /metrics: Prometheus metrics endpoint (placeholder)
pub async fn metrics_handler() -> &'static str {
    debug!("Métricas solicitadas");
    "# metrics"
}