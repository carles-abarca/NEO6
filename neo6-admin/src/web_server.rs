// Web Server module for NEO6 Admin Server
// Provides REST API and web interface for proxy management

use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::{info, error};

use crate::config::AdminServerConfig;
use crate::proxy_manager::{ProxyManager, ProxyCommand};

#[derive(Clone)]
pub struct AppState {
    pub proxy_manager: Arc<Mutex<ProxyManager>>,
}

pub struct WebServer {
    config: AdminServerConfig,
    app_state: AppState,
}

impl WebServer {
    pub fn new(config: AdminServerConfig, proxy_manager: ProxyManager) -> Self {
        Self {
            config,
            app_state: AppState {
                proxy_manager: Arc::new(Mutex::new(proxy_manager)),
            },
        }
    }
    
    pub async fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("{}:{}", self.config.bind_address, self.config.port);
        let app = self.create_router();
        
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        
        info!("NEO6 Admin web server listening on {}", addr);
        
        axum::serve(listener, app).await?;
        
        Ok(())
    }
    
    fn create_router(self) -> Router {
        Router::new()
            // Web interface routes
            .route("/", get(dashboard_handler))
            .route("/dashboard", get(dashboard_handler))
            .route("/proxy/:name", get(proxy_detail_handler))
            
            // API routes
            .route("/api/status", get(api_status_handler))
            .route("/api/proxies", get(api_list_proxies_handler))
            .route("/api/proxies/:name", get(api_proxy_status_handler))
            .route("/api/proxies/:name/start", post(api_start_proxy_handler))
            .route("/api/proxies/:name/stop", post(api_stop_proxy_handler))
            .route("/api/proxies/:name/command", post(api_send_command_handler))
            .route("/api/proxies/start-all", post(api_start_all_handler))
            .route("/api/proxies/stop-all", post(api_stop_all_handler))
            
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
            )
            .with_state(self.app_state)
    }
}

// Web interface handlers
async fn dashboard_handler() -> Html<&'static str> {
    Html(include_str!("../static/dashboard.html"))
}

async fn proxy_detail_handler(Path(name): Path<String>) -> Html<String> {
    let html = format!(
        r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <title>NEO6 Admin - Proxy {}</title>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <style>
                body {{
                    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
                    margin: 0;
                    padding: 20px;
                    background-color: #f5f5f5;
                }}
                .container {{
                    max-width: 1000px;
                    margin: 0 auto;
                }}
                .header {{
                    background: white;
                    padding: 20px;
                    border-radius: 8px;
                    margin-bottom: 20px;
                    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                }}
                .header h1 {{
                    margin: 0;
                    color: #333;
                }}
                .back-button {{
                    color: #007AFF;
                    text-decoration: none;
                    font-weight: 500;
                }}
                .back-button:hover {{
                    text-decoration: underline;
                }}
                .info-grid {{
                    display: grid;
                    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
                    gap: 20px;
                    margin-bottom: 20px;
                }}
                .info-card {{
                    background: white;
                    padding: 20px;
                    border-radius: 8px;
                    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
                }}
                .info-card h3 {{
                    margin: 0 0 15px 0;
                    color: #333;
                    border-bottom: 1px solid #eee;
                    padding-bottom: 10px;
                }}
                .info-item {{
                    display: flex;
                    justify-content: space-between;
                    margin: 10px 0;
                    padding: 8px 0;
                    border-bottom: 1px solid #f5f5f5;
                }}
                .info-label {{
                    font-weight: 500;
                    color: #666;
                }}
                .info-value {{
                    color: #333;
                    font-family: monospace;
                }}
                .status-badge {{
                    padding: 4px 12px;
                    border-radius: 20px;
                    font-size: 12px;
                    font-weight: 600;
                }}
                .status-running {{ background: #34C759; color: white; }}
                .status-stopped {{ background: #FF3B30; color: white; }}
                .status-starting {{ background: #FF9500; color: white; }}
                .status-error {{ background: #FF3B30; color: white; }}
                .controls {{
                    background: white;
                    padding: 20px;
                    border-radius: 8px;
                    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
                    margin-bottom: 20px;
                }}
                .controls h3 {{
                    margin: 0 0 15px 0;
                    color: #333;
                }}
                .button-group {{
                    display: flex;
                    gap: 10px;
                    flex-wrap: wrap;
                }}
                button {{
                    padding: 10px 20px;
                    border: none;
                    border-radius: 6px;
                    cursor: pointer;
                    font-weight: 500;
                    transition: background-color 0.2s;
                }}
                .btn-primary {{
                    background: #007AFF;
                    color: white;
                }}
                .btn-primary:hover {{ background: #0056CC; }}
                .btn-primary:disabled {{ background: #ccc; cursor: not-allowed; }}
                .btn-danger {{
                    background: #FF3B30;
                    color: white;
                }}
                .btn-danger:hover {{ background: #D70015; }}
                .btn-danger:disabled {{ background: #ccc; cursor: not-allowed; }}
                .btn-secondary {{
                    background: #8E8E93;
                    color: white;
                }}
                .btn-secondary:hover {{ background: #636366; }}
                .protocols-list {{
                    display: flex;
                    gap: 8px;
                    flex-wrap: wrap;
                }}
                .protocol-tag {{
                    background: #E5E5EA;
                    color: #333;
                    padding: 4px 8px;
                    border-radius: 4px;
                    font-size: 12px;
                    font-family: monospace;
                }}
                .loading {{
                    text-align: center;
                    padding: 20px;
                    color: #666;
                }}
                .error {{
                    background: #FFEBEE;
                    color: #C62828;
                    padding: 15px;
                    border-radius: 6px;
                    border-left: 4px solid #FF3B30;
                }}
                .last-updated {{
                    text-align: center;
                    color: #666;
                    font-size: 14px;
                    margin-top: 20px;
                }}
            </style>
        </head>
        <body>
            <div class="container">
                <div class="header">
                    <h1>Proxy: {}</h1>
                    <a href="/" class="back-button">← Back to Dashboard</a>
                </div>
                
                <div id="loading" class="loading">Loading proxy information...</div>
                <div id="error" class="error" style="display: none;"></div>
                
                <div id="content" style="display: none;">
                    <div class="info-grid">
                        <div class="info-card">
                            <h3>📊 Status Information</h3>
                            <div class="info-item">
                                <span class="info-label">Status:</span>
                                <span id="status-badge" class="status-badge">Unknown</span>
                            </div>
                            <div class="info-item">
                                <span class="info-label">Protocol:</span>
                                <span id="protocol" class="info-value">-</span>
                            </div>
                            <div class="info-item">
                                <span class="info-label">Port:</span>
                                <span id="port" class="info-value">-</span>
                            </div>
                            <div class="info-item">
                                <span class="info-label">Admin Port:</span>
                                <span id="admin-port" class="info-value">-</span>
                            </div>
                            <div class="info-item">
                                <span class="info-label">PID:</span>
                                <span id="pid" class="info-value">-</span>
                            </div>
                            <div class="info-item">
                                <span class="info-label">Uptime:</span>
                                <span id="uptime" class="info-value">-</span>
                            </div>
                        </div>
                        
                        <div class="info-card">
                            <h3>🔧 Protocol Details</h3>
                            <div class="info-item">
                                <span class="info-label">Loaded Protocols:</span>
                                <div id="protocols" class="protocols-list">-</div>
                            </div>
                        </div>
                    </div>
                    
                    <div class="controls">
                        <h3>🎛️ Controls</h3>
                        <div class="button-group">
                            <button id="start-btn" class="btn-primary" onclick="startProxy()">
                                ▶️ Start Proxy
                            </button>
                            <button id="stop-btn" class="btn-danger" onclick="stopProxy()">
                                ⏹️ Stop Proxy
                            </button>
                            <button class="btn-secondary" onclick="sendCommand('ReloadConfig')">
                                🔄 Reload Config
                            </button>
                            <button class="btn-secondary" onclick="sendCommand('GetProtocols')">
                                📋 Get Protocols
                            </button>
                            <button class="btn-secondary" onclick="refreshStatus()">
                                🔄 Refresh
                            </button>
                        </div>
                    </div>
                </div>
                
                <div id="last-updated" class="last-updated"></div>
            </div>

            <script>
                const proxyName = '{}';
                let isLoading = false;

                function formatUptime(seconds) {{
                    if (seconds < 60) return `${{seconds}}s`;
                    if (seconds < 3600) return `${{Math.floor(seconds / 60)}}m ${{seconds % 60}}s`;
                    if (seconds < 86400) {{
                        const hours = Math.floor(seconds / 3600);
                        const minutes = Math.floor((seconds % 3600) / 60);
                        return `${{hours}}h ${{minutes}}m`;
                    }}
                    const days = Math.floor(seconds / 86400);
                    const hours = Math.floor((seconds % 86400) / 3600);
                    return `${{days}}d ${{hours}}h`;
                }}

                function updateUI(data) {{
                    // Update status information
                    const statusBadge = document.getElementById('status-badge');
                    statusBadge.textContent = data.status;
                    statusBadge.className = `status-badge status-${{data.status.toLowerCase()}}`;
                    
                    document.getElementById('protocol').textContent = data.protocol || '-';
                    document.getElementById('port').textContent = data.port || '-';
                    document.getElementById('admin-port').textContent = data.admin_port || '-';
                    document.getElementById('pid').textContent = data.pid || 'Not running';
                    
                    // Use live data if available, otherwise fall back to admin data
                    let uptime_seconds = data.uptime_seconds;
                    let protocols_loaded = data.protocols_loaded;
                    
                    // Check if we have live data from the proxy
                    if (data.live_proxy_data && data.proxy_responsive) {{
                        const liveData = data.live_proxy_data;
                        if (liveData.uptime_seconds !== undefined) {{
                            uptime_seconds = liveData.uptime_seconds;
                        }}
                        if (liveData.protocols_loaded) {{
                            protocols_loaded = liveData.protocols_loaded;
                        }}
                        
                        // Add indicator that we have live data
                        statusBadge.title = 'Live data from proxy';
                    }} else if (data.proxy_responsive === false) {{
                        // Proxy is not responding
                        statusBadge.title = 'Proxy not responding';
                        statusBadge.textContent += ' (Unresponsive)';
                    }}
                    
                    // Format uptime
                    const uptime = uptime_seconds ? formatUptime(uptime_seconds) : '-';
                    document.getElementById('uptime').textContent = uptime;
                    
                    // Update protocols list
                    const protocolsContainer = document.getElementById('protocols');
                    if (protocols_loaded && protocols_loaded.length > 0) {{
                        protocolsContainer.innerHTML = protocols_loaded
                            .map(p => `<span class="protocol-tag">${{p}}</span>`)
                            .join('');
                    }} else {{
                        protocolsContainer.innerHTML = '<span class="info-value">None loaded</span>';
                    }}
                    
                    // Update button states
                    const startBtn = document.getElementById('start-btn');
                    const stopBtn = document.getElementById('stop-btn');
                    const isRunning = data.status === 'Running';
                    
                    startBtn.disabled = isRunning;
                    stopBtn.disabled = !isRunning;
                    
                    // Update timestamp with responsiveness indicator
                    let timestampText = `Last updated: ${{new Date().toLocaleTimeString()}}`;
                    if (data.proxy_responsive === true) {{
                        timestampText += ' ✅ Live data';
                    }} else if (data.proxy_responsive === false) {{
                        timestampText += ' ⚠️ Admin data only';
                    }}
                    document.getElementById('last-updated').textContent = timestampText;
                }}

                function showError(message) {{
                    document.getElementById('loading').style.display = 'none';
                    document.getElementById('content').style.display = 'none';
                    document.getElementById('error').style.display = 'block';
                    document.getElementById('error').textContent = `Error: ${{message}}`;
                }}

                function showLoading() {{
                    if (!isLoading) {{
                        document.getElementById('loading').style.display = 'block';
                        document.getElementById('content').style.display = 'none';
                        document.getElementById('error').style.display = 'none';
                    }}
                }}

                function hideLoading() {{
                    document.getElementById('loading').style.display = 'none';
                    document.getElementById('content').style.display = 'block';
                    document.getElementById('error').style.display = 'none';
                }}

                async function refreshStatus() {{
                    if (isLoading) return;
                    isLoading = true;
                    
                    try {{
                        const response = await fetch(`/api/proxies/${{proxyName}}`);
                        if (!response.ok) {{
                            throw new Error(`HTTP ${{response.status}}: ${{response.statusText}}`);
                        }}
                        const data = await response.json();
                        updateUI(data);
                        hideLoading();
                    }} catch (error) {{
                        console.error('Error fetching proxy status:', error);
                        showError(error.message);
                    }} finally {{
                        isLoading = false;
                    }}
                }}

                async function startProxy() {{
                    try {{
                        const response = await fetch(`/api/proxies/${{proxyName}}/start`, {{method: 'POST'}});
                        if (response.ok) {{
                            // Wait a bit for the proxy to start, then refresh
                            setTimeout(refreshStatus, 1500);
                        }} else {{
                            const error = await response.text();
                            showError(`Failed to start proxy: ${{error}}`);
                        }}
                    }} catch (error) {{
                        showError(`Failed to start proxy: ${{error.message}}`);
                    }}
                }}

                async function stopProxy() {{
                    if (confirm('Are you sure you want to stop this proxy?')) {{
                        try {{
                            const response = await fetch(`/api/proxies/${{proxyName}}/stop`, {{method: 'POST'}});
                            if (response.ok) {{
                                // Wait a bit for the proxy to stop, then refresh
                                setTimeout(refreshStatus, 1000);
                            }} else {{
                                const error = await response.text();
                                showError(`Failed to stop proxy: ${{error}}`);
                            }}
                        }} catch (error) {{
                            showError(`Failed to stop proxy: ${{error.message}}`);
                        }}
                    }}
                }}

                async function sendCommand(command) {{
                    try {{
                        const commandData = {{ command: command }};
                        const response = await fetch(`/api/proxies/${{proxyName}}/command`, {{
                            method: 'POST',
                            headers: {{ 'Content-Type': 'application/json' }},
                            body: JSON.stringify(commandData)
                        }});
                        
                        if (response.ok) {{
                            const result = await response.json();
                            alert(`Command result: ${{JSON.stringify(result, null, 2)}}`);
                            // Refresh status after command
                            setTimeout(refreshStatus, 500);
                        }} else {{
                            const error = await response.text();
                            showError(`Command failed: ${{error}}`);
                        }}
                    }} catch (error) {{
                        showError(`Command failed: ${{error.message}}`);
                    }}
                }}

                // Initial load and auto-refresh
                showLoading();
                refreshStatus();
                setInterval(refreshStatus, 10000); // Refresh every 10 seconds
            </script>
        </body>
        </html>
        "#,
        name, name, name
    );
    Html(html)
}

// API handlers
async fn api_status_handler(State(_manager): State<AppState>) -> Json<Value> {
    Json(json!({
        "service": "neo6-admin",
        "version": "0.1.0",
        "status": "running"
    }))
}

async fn api_list_proxies_handler(State(app_state): State<AppState>) -> Json<Value> {
    let mut manager = app_state.proxy_manager.lock().await;
    let proxies = manager.get_all_status();
    Json(json!({ "proxies": proxies }))
}

async fn api_proxy_status_handler(
    Path(name): Path<String>,
    State(app_state): State<AppState>
) -> Result<Json<Value>, StatusCode> {
    let mut manager = app_state.proxy_manager.lock().await;
    
    match manager.get_proxy_status(&name) {
        Some(status) => {
            let mut result = json!(status);
            
            // If proxy is running, try to get live information from the proxy itself
            if matches!(status.status, crate::proxy_manager::ProxyState::Running) {
                match manager.send_command_to_proxy(&name, crate::proxy_manager::ProxyCommand::Status).await {
                    Ok(response) => {
                        // Merge live data with basic status
                        if let crate::proxy_manager::ProxyResponse::Success { data } = response {
                            // Add live data to our result
                            result["live_proxy_data"] = data;
                            result["proxy_responsive"] = json!(true);
                        }
                    }
                    Err(_) => {
                        // If we can't reach the proxy, it might not be responding
                        result["proxy_responsive"] = json!(false);
                        result["live_proxy_data"] = json!(null);
                    }
                }
            } else {
                result["proxy_responsive"] = json!(false);
                result["live_proxy_data"] = json!(null);
            }
            
            Ok(Json(result))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn api_start_proxy_handler(
    Path(name): Path<String>,
    State(app_state): State<AppState>
) -> Result<Json<Value>, StatusCode> {
    let mut manager = app_state.proxy_manager.lock().await;
    
    match manager.start_proxy(&name).await {
        Ok(_) => Ok(Json(json!({"message": format!("Proxy '{}' started", name)}))),
        Err(e) => {
            error!("Failed to start proxy '{}': {}", name, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn api_stop_proxy_handler(
    Path(name): Path<String>,
    State(app_state): State<AppState>
) -> Result<Json<Value>, StatusCode> {
    let mut manager = app_state.proxy_manager.lock().await;
    
    match manager.stop_proxy(&name).await {
        Ok(_) => Ok(Json(json!({"message": format!("Proxy '{}' stopped", name)}))),
        Err(e) => {
            error!("Failed to stop proxy '{}': {}", name, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn api_send_command_handler(
    Path(name): Path<String>,
    State(app_state): State<AppState>,
    Json(command): Json<ProxyCommand>
) -> Result<Json<Value>, StatusCode> {
    let manager = app_state.proxy_manager.lock().await;
    
    match manager.send_command_to_proxy(&name, command).await {
        Ok(response) => Ok(Json(json!(response))),
        Err(e) => {
            error!("Failed to send command to proxy '{}': {}", name, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn api_start_all_handler(State(app_state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    let mut manager = app_state.proxy_manager.lock().await;
    
    match manager.start_auto_start_instances().await {
        Ok(_) => Ok(Json(json!({"message": "Auto-start instances started"}))),
        Err(e) => {
            error!("Failed to start auto-start instances: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn api_stop_all_handler(State(app_state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    let mut manager = app_state.proxy_manager.lock().await;
    
    match manager.stop_all().await {
        Ok(_) => Ok(Json(json!({"message": "All proxies stopped"}))),
        Err(e) => {
            error!("Failed to stop all proxies: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
