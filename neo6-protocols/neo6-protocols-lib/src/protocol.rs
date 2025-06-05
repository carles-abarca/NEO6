use serde_json::Value;
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct TransactionMap {
    pub transactions: HashMap<String, TransactionConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TransactionConfig {
    pub protocol: String,
    pub server: String,
    pub parameters: Vec<ParameterConfig>,
    pub expected_response: Option<serde_yaml::Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ParameterConfig {
    pub name: String,
    pub r#type: String,
    pub required: bool,
}

#[async_trait]
pub trait ProtocolHandler: Send + Sync {
    async fn invoke_transaction(&self, transaction_id: &str, parameters: Value) -> Result<Value, String>;
}
