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

pub struct WebServer {
    config: AdminServerConfig,
    proxy_manager: Arc<Mutex<ProxyManager>>,
}

type AppState = Arc<Mutex<ProxyManager>>;

impl WebServer {
    pub fn new(config: AdminServerConfig, proxy_manager: ProxyManager) -> Self {
        Self {
            config,
            proxy_manager: Arc::new(Mutex::new(proxy_manager)),
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
            .with_state(self.proxy_manager)
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
        <html>
        <head>
            <title>NEO6 Admin - Proxy {}</title>
            <meta charset="UTF-8">
            <style>
                body {{ font-family: Arial, sans-serif; margin: 40px; }}
                .header {{ border-bottom: 1px solid #ccc; padding-bottom: 10px; }}
                .status {{ margin: 20px 0; }}
                .controls {{ margin: 20px 0; }}
                button {{ margin: 5px; padding: 10px; }}
            </style>
        </head>
        <body>
            <div class="header">
                <h1>NEO6 Admin - Proxy {}</h1>
                <a href="/">← Back to Dashboard</a>
            </div>
            <div id="proxy-status" class="status">Loading...</div>
            <div class="controls">
                <button onclick="startProxy()">Start</button>
                <button onclick="stopProxy()">Stop</button>
                <button onclick="reloadConfig()">Reload Config</button>
                <button onclick="getStatus()">Refresh Status</button>
            </div>
            <script>
                const proxyName = '{}';
                // Add JavaScript functions here
                function refreshStatus() {{
                    fetch(`/api/proxies/${{proxyName}}`)
                        .then(r => r.json())
                        .then(data => {{
                            document.getElementById('proxy-status').innerHTML = 
                                '<pre>' + JSON.stringify(data, null, 2) + '</pre>';
                        }});
                }}
                function startProxy() {{
                    fetch(`/api/proxies/${{proxyName}}/start`, {{method: 'POST'}})
                        .then(() => setTimeout(refreshStatus, 1000));
                }}
                function stopProxy() {{
                    fetch(`/api/proxies/${{proxyName}}/stop`, {{method: 'POST'}})
                        .then(() => setTimeout(refreshStatus, 1000));
                }}
                refreshStatus();
                setInterval(refreshStatus, 5000);
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

async fn api_list_proxies_handler(State(manager): State<AppState>) -> Json<Value> {
    let mut manager = manager.lock().await;
    let proxies = manager.get_all_status();
    Json(json!({ "proxies": proxies }))
}

async fn api_proxy_status_handler(
    Path(name): Path<String>,
    State(manager): State<AppState>
) -> Result<Json<Value>, StatusCode> {
    let mut manager = manager.lock().await;
    
    match manager.get_proxy_status(&name) {
        Some(status) => Ok(Json(json!(status))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn api_start_proxy_handler(
    Path(name): Path<String>,
    State(manager): State<AppState>
) -> Result<Json<Value>, StatusCode> {
    let mut manager = manager.lock().await;
    
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
    State(manager): State<AppState>
) -> Result<Json<Value>, StatusCode> {
    let mut manager = manager.lock().await;
    
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
    State(manager): State<AppState>,
    Json(command): Json<ProxyCommand>
) -> Result<Json<Value>, StatusCode> {
    let manager = manager.lock().await;
    
    match manager.send_command_to_proxy(&name, command).await {
        Ok(response) => Ok(Json(json!(response))),
        Err(e) => {
            error!("Failed to send command to proxy '{}': {}", name, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn api_start_all_handler(State(manager): State<AppState>) -> Result<Json<Value>, StatusCode> {
    let mut manager = manager.lock().await;
    
    match manager.start_auto_start_instances().await {
        Ok(_) => Ok(Json(json!({"message": "Auto-start instances started"}))),
        Err(e) => {
            error!("Failed to start auto-start instances: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn api_stop_all_handler(State(manager): State<AppState>) -> Result<Json<Value>, StatusCode> {
    let mut manager = manager.lock().await;
    
    match manager.stop_all().await {
        Ok(_) => Ok(Json(json!({"message": "All proxies stopped"}))),
        Err(e) => {
            error!("Failed to stop all proxies: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
