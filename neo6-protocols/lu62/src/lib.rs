use neo6_protocols_lib::protocol::ProtocolHandler;
use serde_json::Value;
use async_trait::async_trait;

pub struct Lu62Handler;

#[async_trait]
impl ProtocolHandler for Lu62Handler {
    async fn invoke_transaction(&self, transaction_id: &str, parameters: Value) -> Result<Value, String> {
        Ok(serde_json::json!({
            "protocol": "lu62",
            "transaction_id": transaction_id,
            "parameters": parameters,
            "result": "mocked-lu62-response"
        }))
    }
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn it_works() {
        let result = super::add(2, 2);
}
