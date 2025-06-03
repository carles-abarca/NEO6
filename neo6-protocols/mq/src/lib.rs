use neo6_protocols_lib::protocol::ProtocolHandler;
use serde_json::Value;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

pub struct MqHandler;

impl ProtocolHandler for MqHandler {
    fn invoke_transaction(&self, transaction_id: &str, parameters: Value) -> Result<Value, String> {
        Ok(serde_json::json!({
            "protocol": "mq",
            "transaction_id": transaction_id,
            "parameters": parameters,
            "result": "mocked-mq-response"
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
