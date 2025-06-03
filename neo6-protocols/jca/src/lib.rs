use neo6_protocols_lib::protocol::ProtocolHandler;
use serde_json::Value;
use async_trait::async_trait;

pub struct JcaHandler;

#[async_trait]
impl ProtocolHandler for JcaHandler {
    async fn invoke_transaction(&self, transaction_id: &str, parameters: Value) -> Result<Value, String> {
        Ok(serde_json::json!({
            "protocol": "jca",
            "transaction_id": transaction_id,
            "parameters": parameters,
            "result": "mocked-jca-response"
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
        assert_eq!(result, 4);
    }
}
