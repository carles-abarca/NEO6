// Configuration loader

use crate::cics::mapping;
use config::{Config, File};
use serde::Deserialize;

pub fn load_transaction_map(path: &str) -> Result<mapping::TransactionMap, Box<dyn std::error::Error>> {
    crate::cics::mapping::load_transaction_map(path)
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProxyConfig {
    pub log_level: String,
    pub protocol: Option<String>, // Protocolo a escuchar (rest, lu62, mq, tcp, jca, tn3270)
    pub port: Option<u16>,        // Puerto a escuchar
}

impl ProxyConfig {
    pub fn from_default() -> Self {
        let builder = Config::builder()
            .add_source(File::with_name("config/default.toml"));
        let settings = builder.build().unwrap();
        let log_level = settings.get_string("log_level").unwrap_or_else(|_| "info".to_string());
        let protocol = settings.get_string("protocol").ok();
        let port = settings.get_int("port").ok().map(|v| v as u16);
        ProxyConfig { log_level, protocol, port }
    }
}