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
use tower_http::{trace::TraceLayer, services::ServeDir};
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
            
            // Static files routes
            .nest_service("/static", ServeDir::new("static"))
            .fallback_service(ServeDir::new("static"))
            
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
    // Read the template from static file
    let template = include_str!("../static/proxy-detail.html");
    
    // Replace placeholders with the proxy name
    let html = template.replace("{{PROXY_NAME}}", &name);
    
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
