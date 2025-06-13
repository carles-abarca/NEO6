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

impl Default for ProxyConfig {
    fn default() -> Self {
        ProxyConfig {
            log_level: "info".to_string(),
            protocol: None,
            port: None,
        }
    }
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

    pub fn load_from_dir(&mut self, config_dir: &str) {
        let config_path = format!("{}/default.toml", config_dir);
        if std::path::Path::new(&config_path).exists() {
            let builder = Config::builder()
                .add_source(File::with_name(&config_path.replace(".toml", "")));
            if let Ok(settings) = builder.build() {
                if let Ok(log_level) = settings.get_string("log_level") {
                    self.log_level = log_level;
                }
                if let Ok(protocol) = settings.get_string("protocol") {
                    self.protocol = Some(protocol);
                }
                if let Ok(port) = settings.get_int("port") {
                    self.port = Some(port as u16);
                }
            }
        }
    }
}