use serde_json::Value;

pub trait ProtocolHandler {
    fn invoke_transaction(&self, transaction_id: &str, parameters: Value) -> Result<Value, String>;
}
