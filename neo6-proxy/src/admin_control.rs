// Administrative control module for neo6-proxy
// Provides a control socket interface for external administration

use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};
use serde_json;
use tracing::{debug, error, info};

/// Commands that can be sent to the proxy via the control socket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command")]
pub enum AdminCommand {
    /// Get current status of the proxy
    Status,
    /// Shutdown the proxy gracefully
    Shutdown,
    /// Reload configuration
    ReloadConfig,
    /// Get metrics
    GetMetrics,
    /// Change log level
    SetLogLevel { level: String },
    /// Get loaded protocols
    GetProtocols,
    /// Test protocol connectivity
    TestProtocol { protocol: String },
}

/// Response from the proxy to admin commands
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum AdminResponse {
    Success { data: serde_json::Value },
    Error { message: String },
}

/// Control channel messages
#[derive(Debug)]
pub enum ControlMessage {
    Shutdown,
    ReloadConfig,
    SetLogLevel(String),
}

/// Administrative control server
pub struct AdminControlServer {
    control_port: u16,
    control_tx: mpsc::Sender<ControlMessage>,
    proxy_info: ProxyInfo,
}

/// Information about the proxy instance
#[derive(Debug, Clone, Serialize)]
pub struct ProxyInfo {
    pub protocol: String,
    pub port: u16,
    pub status: String,
    pub uptime: std::time::SystemTime,
    pub protocols_loaded: Vec<String>,
}

impl AdminControlServer {
    /// Create a new admin control server
    pub fn new(control_port: u16, proxy_info: ProxyInfo) -> (Self, mpsc::Receiver<ControlMessage>) {
        let (control_tx, control_rx) = mpsc::channel(32);
        
        let server = AdminControlServer {
            control_port,
            control_tx,
            proxy_info,
        };
        
        (server, control_rx)
    }

    /// Start the admin control server
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.control_port));
        let listener = TcpListener::bind(addr).await?;
        
        info!("Admin control server listening on {}", addr);
        
        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    debug!("Admin connection from {}", peer_addr);
                    let proxy_info = self.proxy_info.clone();
                    let control_tx = self.control_tx.clone();
                    
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_admin_connection(stream, proxy_info, control_tx).await {
                            error!("Error handling admin connection from {}: {}", peer_addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting admin connection: {}", e);
                }
            }
        }
    }

    /// Handle a single admin connection
    async fn handle_admin_connection(
        mut stream: TcpStream,
        proxy_info: ProxyInfo,
        control_tx: mpsc::Sender<ControlMessage>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (reader, writer) = stream.split();
        let mut reader = BufReader::new(reader);
        let mut writer = writer;
        let mut line = String::new();

        // Send welcome message
        let welcome = serde_json::json!({
            "message": "NEO6 Proxy Admin Control",
            "version": "0.1.0",
            "proxy": proxy_info
        });
        writer.write_all(format!("{}\n", welcome).as_bytes()).await?;

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // Connection closed
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    debug!("Admin command received: {}", trimmed);
                    
                    let response = match serde_json::from_str::<AdminCommand>(trimmed) {
                        Ok(command) => Self::process_command(command, &proxy_info, &control_tx).await,
                        Err(e) => AdminResponse::Error {
                            message: format!("Invalid command format: {}", e),
                        },
                    };

                    let response_json = serde_json::to_string(&response)?;
                    writer.write_all(format!("{}\n", response_json).as_bytes()).await?;
                }
                Err(e) => {
                    error!("Error reading admin command: {}", e);
                    break;
                }
            }
        }

        debug!("Admin connection closed");
        Ok(())
    }

    /// Process an admin command
    async fn process_command(
        command: AdminCommand,
        proxy_info: &ProxyInfo,
        control_tx: &mpsc::Sender<ControlMessage>,
    ) -> AdminResponse {
        match command {
            AdminCommand::Status => {
                let uptime_secs = proxy_info.uptime.elapsed()
                    .unwrap_or(std::time::Duration::from_secs(0))
                    .as_secs();
                
                AdminResponse::Success {
                    data: serde_json::json!({
                        "protocol": proxy_info.protocol,
                        "port": proxy_info.port,
                        "status": proxy_info.status,
                        "uptime_seconds": uptime_secs,
                        "protocols_loaded": proxy_info.protocols_loaded
                    }),
                }
            }
            AdminCommand::Shutdown => {
                info!("Shutdown command received via admin control");
                if let Err(e) = control_tx.send(ControlMessage::Shutdown).await {
                    error!("Error sending shutdown message: {}", e);
                    AdminResponse::Error {
                        message: format!("Error sending shutdown command: {}", e),
                    }
                } else {
                    AdminResponse::Success {
                        data: serde_json::json!({"message": "Shutdown initiated"}),
                    }
                }
            }
            AdminCommand::ReloadConfig => {
                info!("Reload config command received via admin control");
                if let Err(e) = control_tx.send(ControlMessage::ReloadConfig).await {
                    error!("Error sending reload config message: {}", e);
                    AdminResponse::Error {
                        message: format!("Error sending reload config command: {}", e),
                    }
                } else {
                    AdminResponse::Success {
                        data: serde_json::json!({"message": "Config reload initiated"}),
                    }
                }
            }
            AdminCommand::SetLogLevel { level } => {
                info!("Set log level command received: {}", level);
                if let Err(e) = control_tx.send(ControlMessage::SetLogLevel(level.clone())).await {
                    error!("Error sending set log level message: {}", e);
                    AdminResponse::Error {
                        message: format!("Error sending set log level command: {}", e),
                    }
                } else {
                    AdminResponse::Success {
                        data: serde_json::json!({"message": format!("Log level set to {}", level)}),
                    }
                }
            }
            AdminCommand::GetMetrics => {
                // TODO: Implement metrics collection
                AdminResponse::Success {
                    data: serde_json::json!({
                        "metrics": "not_implemented_yet",
                        "message": "Metrics collection not yet implemented"
                    }),
                }
            }
            AdminCommand::GetProtocols => {
                AdminResponse::Success {
                    data: serde_json::json!({
                        "protocols": proxy_info.protocols_loaded
                    }),
                }
            }
            AdminCommand::TestProtocol { protocol } => {
                // TODO: Implement protocol testing
                AdminResponse::Success {
                    data: serde_json::json!({
                        "protocol": protocol,
                        "test_result": "not_implemented_yet",
                        "message": "Protocol testing not yet implemented"
                    }),
                }
            }
        }
    }
}
