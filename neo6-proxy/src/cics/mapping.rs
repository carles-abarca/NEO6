// COBOL ↔ JSON mapping

use serde::Deserialize;
use std::collections::HashMap;
use tracing::{error, debug};

#[derive(Debug, Deserialize)]
pub struct TransactionMap {
    pub transactions: HashMap<String, TransactionConfig>,
}

#[derive(Debug, Deserialize)]
pub struct TransactionConfig {
    pub protocol: String,
    pub server: String,
    pub parameters: Vec<ParameterConfig>,
    pub expected_response: Option<serde_yaml::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ParameterConfig {
    pub name: String,
    pub r#type: String,
    pub required: bool,
}

pub fn load_transaction_map(path: &str) -> Result<TransactionMap, Box<dyn std::error::Error>> {
    debug!("Cargando mapa de transacciones desde {}", path);
    let file = std::fs::File::open(path).map_err(|e| {
        error!(error = %e, "Error abriendo archivo de transacciones");
        e
    })?;
    let map: TransactionMap = serde_yaml::from_reader(file).map_err(|e| {
        error!(error = %e, "Error parseando YAML de transacciones");
        e
    })?;
    Ok(map)
}