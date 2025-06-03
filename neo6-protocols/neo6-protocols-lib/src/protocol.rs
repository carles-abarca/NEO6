use serde_json::Value;
use async_trait::async_trait;

#[async_trait]
pub trait ProtocolHandler: Send + Sync {
    async fn invoke_transaction(&self, transaction_id: &str, parameters: Value) -> Result<Value, String>;
}
