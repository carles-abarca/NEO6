//! Trait común para todos los protocolos soportados por NEO6.

use tracing::{info, debug};

pub trait ProtocolHandler {
    /// Invoca una transacción con parámetros genéricos y devuelve el resultado como JSON.
    fn invoke_transaction(&self, transaction_id: &str, parameters: serde_json::Value) -> Result<serde_json::Value, String>;
}

/// Helper para loggear la invocación de cualquier protocolo
pub fn log_protocol_invoke(proto: &str, tx: &str, params: &serde_json::Value) {
    info!(protocol = %proto, transaction = %tx, "Invocando transacción de protocolo");
    debug!(?params, "Parámetros de la transacción");
}
