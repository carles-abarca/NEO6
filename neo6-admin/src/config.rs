// Configuration module for NEO6 Admin Server

use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    pub admin: AdminServerConfig,
    pub proxy_instances: Vec<ProxyInstanceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminServerConfig {
    pub port: u16,
    pub bind_address: String,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyInstanceConfig {
    pub name: String,
    pub protocol: String,
    pub port: u16,
    pub admin_port: u16,
    pub auto_start: bool,
    pub config_path: String,
    pub binary_path: String,
    pub working_directory: String,
    pub log_level: String,
    #[serde(default)]
    pub args: Vec<String>,
}

impl AdminConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: AdminConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}

impl Default for AdminServerConfig {
    fn default() -> Self {
        Self {
            port: 8090,
            bind_address: "0.0.0.0".to_string(),
            log_level: "info".to_string(),
        }
    }
}
