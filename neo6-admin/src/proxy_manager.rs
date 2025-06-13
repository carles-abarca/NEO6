// Proxy Manager module for NEO6 Admin Server
// Manages multiple proxy instances: start, stop, monitor, control

use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use tokio::net::TcpStream;
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};
use serde::{Deserialize, Serialize};
use serde_json;
use tracing::{debug, error, info, warn};

use crate::config::ProxyInstanceConfig;

#[derive(Debug, Clone, Serialize)]
pub struct ProxyStatus {
    pub name: String,
    pub protocol: String,
    pub port: u16,
    pub admin_port: u16,
    pub status: ProxyState,
    pub pid: Option<u32>,
    pub uptime_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub enum ProxyState {
    Stopped,
    Starting,
    Running,
    Error(String),
}

#[derive(Debug)]
struct ManagedProxy {
    config: ProxyInstanceConfig,
    process: Option<Child>,
    status: ProxyState,
    start_time: Option<std::time::SystemTime>,
}

pub struct ProxyManager {
    proxies: HashMap<String, ManagedProxy>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "command")]
pub enum ProxyCommand {
    Status,
    Shutdown,
    ReloadConfig,
    SetLogLevel { level: String },
    GetProtocols,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum ProxyResponse {
    Success { data: serde_json::Value },
    Error { message: String },
}

impl ProxyManager {
    pub fn new(configs: Vec<ProxyInstanceConfig>) -> Self {
        let mut proxies = HashMap::new();
        
        for config in configs {
            let proxy = ManagedProxy {
                config: config.clone(),
                process: None,
                status: ProxyState::Stopped,
                start_time: None,
            };
            proxies.insert(config.name.clone(), proxy);
        }
        
        Self { proxies }
    }
    
    pub async fn start_proxy(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let proxy = self.proxies.get_mut(name)
            .ok_or_else(|| format!("Proxy '{}' not found", name))?;
        
        if matches!(proxy.status, ProxyState::Running) {
            return Err(format!("Proxy '{}' is already running", name).into());
        }
        
        info!("Starting proxy instance: {}", name);
        proxy.status = ProxyState::Starting;
        
        // Create log file paths
        let log_dir = std::path::Path::new("logs");
        if !log_dir.exists() {
            std::fs::create_dir_all(log_dir)?;
        }
        
        let stdout_log_path = format!("logs/{}.log", name);
        let stderr_log_path = format!("logs/{}.error.log", name);
        
        // Create log files
        let stdout_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&stdout_log_path)?;
        let stderr_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&stderr_log_path)?;
        
        // Build command to start the proxy
        let mut cmd = Command::new(&proxy.config.binary_path);
        cmd.arg("--protocol").arg(&proxy.config.protocol)
           .arg("--port").arg(proxy.config.port.to_string())
           .arg("--log-level").arg(&proxy.config.log_level)
           .current_dir(&proxy.config.working_directory)
           .stdout(Stdio::from(stdout_file))
           .stderr(Stdio::from(stderr_file));
        
        // Add additional arguments from config
        for arg in &proxy.config.args {
            cmd.arg(arg);
        }
        
        // Set environment variables
        cmd.env("DYLD_LIBRARY_PATH", "./lib");
        
        info!("Starting proxy '{}' with logs: {} / {}", name, stdout_log_path, stderr_log_path);
        
        match cmd.spawn() {
            Ok(mut child) => {
                proxy.start_time = Some(std::time::SystemTime::now());
                proxy.status = ProxyState::Running;
                info!("Proxy '{}' started successfully", name);
                
                // Give it a moment to start up
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                
                // Check if the process is still running
                match child.try_wait() {
                    Ok(Some(exit_status)) => {
                        let error_msg = format!("Proxy '{}' exited unexpectedly with status: {}", name, exit_status);
                        error!("{}", error_msg);
                        error!("Check logs at: {} / {}", stdout_log_path, stderr_log_path);
                        proxy.status = ProxyState::Error(error_msg.clone());
                        proxy.process = None;
                        Err(error_msg.into())
                    }
                    Ok(None) => {
                        // Process is still running
                        proxy.process = Some(child);
                        info!("Proxy '{}' is running and responsive", name);
                        Ok(())
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to check proxy '{}' status: {}", name, e);
                        error!("{}", error_msg);
                        proxy.status = ProxyState::Error(error_msg.clone());
                        proxy.process = Some(child); // Keep the process reference just in case
                        Err(error_msg.into())
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to start proxy '{}': {}", name, e);
                error!("{}", error_msg);
                proxy.status = ProxyState::Error(error_msg.clone());
                Err(error_msg.into())
            }
        }
    }
    
    pub async fn stop_proxy(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        // First check if proxy exists and get its status
        let proxy_exists = self.proxies.contains_key(name);
        if !proxy_exists {
            return Err(format!("Proxy '{}' not found", name).into());
        }

        let is_stopped = if let Some(proxy) = self.proxies.get(name) {
            matches!(proxy.status, ProxyState::Stopped)
        } else {
            return Err(format!("Proxy '{}' not found", name).into());
        };

        if is_stopped {
            return Ok(());
        }
        
        info!("Stopping proxy instance: {}", name);
        
        // Try to send shutdown command via admin control first
        if let Err(e) = self.send_command_to_proxy(name, ProxyCommand::Shutdown).await {
            warn!("Failed to send shutdown command to proxy '{}': {}", name, e);
        }
        
        // Wait a bit for graceful shutdown
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Now get mutable access to the proxy for process handling
        let proxy = self.proxies.get_mut(name)
            .ok_or_else(|| format!("Proxy '{}' not found", name))?;
        
        // Force kill if still running
        if let Some(mut process) = proxy.process.take() {
            match process.try_wait() {
                Ok(Some(_)) => {
                    info!("Proxy '{}' shut down gracefully", name);
                }
                Ok(None) => {
                    warn!("Proxy '{}' still running, force killing", name);
                    process.kill()?;
                    process.wait()?;
                }
                Err(e) => {
                    error!("Error checking proxy '{}' status: {}", name, e);
                }
            }
        }
        
        proxy.status = ProxyState::Stopped;
        proxy.start_time = None;
        
        Ok(())
    }
    
    pub async fn start_auto_start_instances(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let auto_start_names: Vec<String> = self.proxies
            .iter()
            .filter_map(|(name, proxy)| {
                if proxy.config.auto_start {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();
        
        for name in auto_start_names {
            if let Err(e) = self.start_proxy(&name).await {
                error!("Failed to start auto-start proxy '{}': {}", name, e);
            }
        }
        
        Ok(())
    }
    
    pub async fn stop_all(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let names: Vec<String> = self.proxies.keys().cloned().collect();
        
        for name in names {
            if let Err(e) = self.stop_proxy(&name).await {
                error!("Failed to stop proxy '{}': {}", name, e);
            }
        }
        
        Ok(())
    }
    
    pub fn get_proxy_status(&mut self, name: &str) -> Option<ProxyStatus> {
        let proxy = self.proxies.get_mut(name)?;
        
        // Check if process is still running
        if let Some(ref mut process) = proxy.process {
            match process.try_wait() {
                Ok(Some(_)) => {
                    // Process has exited
                    proxy.status = ProxyState::Stopped;
                    proxy.process = None;
                    proxy.start_time = None;
                }
                Ok(None) => {
                    // Process is still running
                    if !matches!(proxy.status, ProxyState::Running) {
                        proxy.status = ProxyState::Running;
                    }
                }
                Err(_) => {
                    // Error checking process
                    proxy.status = ProxyState::Error("Process check failed".to_string());
                }
            }
        }
        
        let uptime_seconds = proxy.start_time
            .and_then(|start| start.elapsed().ok())
            .map(|duration| duration.as_secs());
        
        let pid = proxy.process.as_ref().map(|p| p.id());
        
        Some(ProxyStatus {
            name: proxy.config.name.clone(),
            protocol: proxy.config.protocol.clone(),
            port: proxy.config.port,
            admin_port: proxy.config.admin_port,
            status: proxy.status.clone(),
            pid,
            uptime_seconds,
        })
    }
    
    pub fn get_all_status(&mut self) -> Vec<ProxyStatus> {
        let names: Vec<String> = self.proxies.keys().cloned().collect();
        names.into_iter()
            .filter_map(|name| self.get_proxy_status(&name))
            .collect()
    }
    
    pub async fn send_command_to_proxy(
        &self,
        name: &str,
        command: ProxyCommand,
    ) -> Result<ProxyResponse, Box<dyn std::error::Error>> {
        let proxy = self.proxies.get(name)
            .ok_or_else(|| format!("Proxy '{}' not found", name))?;
        
        let addr = format!("127.0.0.1:{}", proxy.config.admin_port);
        let mut stream = TcpStream::connect(&addr).await?;
        
        // Read welcome message
        let (reader, writer) = stream.split();
        let mut reader = BufReader::new(reader);
        let mut writer = writer;
        let mut line = String::new();
        
        // Read welcome message
        reader.read_line(&mut line).await?;
        debug!("Welcome message from {}: {}", name, line.trim());
        
        // Send command
        let command_json = serde_json::to_string(&command)?;
        writer.write_all(format!("{}\n", command_json).as_bytes()).await?;
        
        // Read response
        line.clear();
        reader.read_line(&mut line).await?;
        
        let response: ProxyResponse = serde_json::from_str(line.trim())?;
        Ok(response)
    }
}
