// Configuration module for NEO6 Admin Server

use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    pub admin: AdminServerConfig,
    pub proxy_defaults: ProxyDefaults,
    pub proxy_instances: Vec<ProxyInstanceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminServerConfig {
    pub port: u16,
    pub bind_address: String,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyDefaults {
    pub library_path: String,
    pub config_path: String,
    pub binary_path: String,
    pub working_directory: String,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyInstanceConfig {
    pub name: String,
    pub protocol: String,
    pub port: u16,
    pub admin_port: u16,
    pub auto_start: bool,
    // Optional overrides for defaults
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_level: Option<String>,
}

impl ProxyInstanceConfig {
    /// Generate command line arguments automatically from configuration fields
    pub fn generate_args(&self, defaults: &ProxyDefaults) -> Vec<String> {
        let mut args = Vec::new();
        
        // Add config directory (use override or default)
        args.push("--config-dir".to_string());
        args.push(self.config_path.as_ref().unwrap_or(&defaults.config_path).clone());
        
        // Add library path
        args.push("--library-path".to_string());
        args.push(defaults.library_path.clone());
        
        // Add log level (use override or default)
        args.push("--log-level".to_string());
        args.push(self.log_level.as_ref().unwrap_or(&defaults.log_level).clone());
        
        // Add port
        args.push("--port".to_string());
        args.push(self.port.to_string());
        
        // Add admin port
        args.push("--admin-port".to_string());
        args.push(self.admin_port.to_string());
        
        // Add protocol
        args.push("--protocol".to_string());
        args.push(self.protocol.clone());
        
        args
    }

    /// Get the effective binary path (override or default)
    pub fn get_binary_path(&self, defaults: &ProxyDefaults) -> String {
        self.binary_path.as_ref().unwrap_or(&defaults.binary_path).clone()
    }

    /// Get the effective working directory (override or default)
    pub fn get_working_directory(&self, defaults: &ProxyDefaults) -> String {
        self.working_directory.as_ref().unwrap_or(&defaults.working_directory).clone()
    }
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

impl Default for ProxyDefaults {
    fn default() -> Self {
        Self {
            library_path: "./lib".to_string(),
            config_path: "./config/proxy".to_string(),
            binary_path: "./bin/neo6-proxy".to_string(),
            working_directory: ".".to_string(),
            log_level: "info".to_string(),
        }
    }
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            admin: AdminServerConfig::default(),
            proxy_defaults: ProxyDefaults::default(),
            proxy_instances: Vec::new(),
        }
    }
}
