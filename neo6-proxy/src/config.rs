// Configuration loader

use crate::cics::mapping;
use config::{Config, File};
use serde::Deserialize;

pub fn load_transaction_map(path: &str) -> Result<mapping::TransactionMap, Box<dyn std::error::Error>> {
    mapping::load_transaction_map(path)
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProxyConfig {
    pub log_level: String,
}

impl ProxyConfig {
    pub fn from_default() -> Self {
        let mut settings = Config::default();
        settings
            .merge(File::with_name("config/default")).unwrap();
        let log_level = settings.get_string("log_level").unwrap_or_else(|_| "info".to_string());
        ProxyConfig { log_level }
    }
}