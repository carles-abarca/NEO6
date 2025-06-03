use neo6_protocols_lib::protocol::ProtocolHandler;
use reqwest::Client;
use reqwest::header::{HeaderMap, CONTENT_TYPE};
use serde_json::Value;
use async_trait::async_trait;

pub struct RestHandler {
    pub base_url: String,
    pub default_headers: Option<HeaderMap>,
}

impl RestHandler {
    pub fn new(base_url: String, default_headers: Option<HeaderMap>) -> Self {
        RestHandler { base_url, default_headers }
    }
}

#[async_trait]
impl ProtocolHandler for RestHandler {
    async fn invoke_transaction(&self, transaction_id: &str, parameters: Value) -> Result<Value, String> {
        let client = Client::new();
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), transaction_id);
        let mut req = client.post(&url);
        if let Some(headers) = &self.default_headers {
            req = req.headers(headers.clone());
        }
        req = req.header(CONTENT_TYPE, "application/json");
        let resp = req.json(&parameters).send().await.map_err(|e| format!("HTTP error: {}", e))?;
        let status = resp.status();
        let json: Value = resp.json().await.map_err(|e| format!("Invalid JSON response: {}", e))?;
        if status.is_success() {
            Ok(json)
        } else {
            Err(format!("REST error {}: {}", status, json))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio;

    #[tokio::test]
    async fn it_works() {
        let result = super::add(2, 2);
        assert_eq!(result, 4);
    }

    #[tokio::test]
    async fn test_rest_handler_mock() {
        let handler = RestHandler::new("http://localhost:8080/api".to_string(), None);
        assert_eq!(handler.base_url, "http://localhost:8080/api");
    }
}

// Suma de ejemplo (puede eliminarse si no se usa)
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}
