use neo6_protocols_lib::protocol::ProtocolHandler;
use serde_json::Value;

pub struct Lu62Handler;

impl ProtocolHandler for Lu62Handler {
    fn invoke_transaction(&self, transaction_id: &str, parameters: Value) -> Result<Value, String> {
        // Aquí iría la lógica real de LU6.2
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

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
