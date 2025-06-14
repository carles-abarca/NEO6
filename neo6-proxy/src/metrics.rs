// Metrics collection module for neo6-proxy
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Connection metrics (serializable for API responses)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetrics {
    pub connection_id: String,
    pub protocol: String,
    pub remote_address: String,
    pub connected_at_timestamp: u64, // Unix timestamp
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub requests_processed: u64,
    pub last_activity_timestamp: u64, // Unix timestamp
}

/// Internal connection tracking (with Instant for performance)
#[derive(Debug, Clone)]
pub struct InternalConnectionMetrics {
    pub connection_id: String,
    pub protocol: String,
    pub remote_address: String,
    pub connected_at: Instant,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub requests_processed: u64,
    pub last_activity: Instant,
    pub connected_at_timestamp: u64,
    pub last_activity_timestamp: u64,
}

/// Protocol metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMetrics {
    pub protocol_name: String,
    pub total_connections: u64,
    pub active_connections: u64,
    pub failed_connections: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub total_requests: u64,
    pub avg_response_time_ms: f64,
    pub uptime_seconds: u64,
}

/// Overall proxy metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyMetrics {
    pub uptime_seconds: u64,
    pub total_connections: u64,
    pub active_connections: u64,
    pub protocols: HashMap<String, ProtocolMetrics>,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
}

/// Metrics collector
pub struct MetricsCollector {
    start_time: Instant,
    connections: Arc<RwLock<HashMap<String, InternalConnectionMetrics>>>,
    protocol_stats: Arc<RwLock<HashMap<String, ProtocolMetrics>>>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            connections: Arc::new(RwLock::new(HashMap::new())),
            protocol_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new connection
    pub async fn register_connection(&self, connection_id: &str, protocol: String, remote_address: String) {
        let now = Instant::now();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let metrics = InternalConnectionMetrics {
            connection_id: connection_id.to_string(),
            protocol: protocol.clone(),
            remote_address,
            connected_at: now,
            bytes_sent: 0,
            bytes_received: 0,
            requests_processed: 0,
            last_activity: now,
            connected_at_timestamp: timestamp,
            last_activity_timestamp: timestamp,
        };

        {
            let mut connections = self.connections.write().await;
            connections.insert(connection_id.to_string(), metrics);
        }

        // Update protocol stats
        {
            let mut protocol_stats = self.protocol_stats.write().await;
            let stats = protocol_stats.entry(protocol.clone()).or_insert_with(|| ProtocolMetrics {
                protocol_name: protocol,
                total_connections: 0,
                active_connections: 0,
                failed_connections: 0,
                total_bytes_sent: 0,
                total_bytes_received: 0,
                total_requests: 0,
                avg_response_time_ms: 0.0,
                uptime_seconds: 0,
            });
            stats.total_connections += 1;
            stats.active_connections += 1;
        }

        debug!("Registered connection: {}", connection_id);
    }

    /// Update connection activity
    pub async fn update_connection_activity(&self, connection_id: &str, bytes_sent: u64, bytes_received: u64) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.get_mut(connection_id) {
            conn.bytes_sent += bytes_sent;
            conn.bytes_received += bytes_received;
            conn.requests_processed += 1;
            conn.last_activity = Instant::now();
            conn.last_activity_timestamp = timestamp;

            // Update protocol stats
            let protocol = conn.protocol.clone();
            drop(connections);

            let mut protocol_stats = self.protocol_stats.write().await;
            if let Some(stats) = protocol_stats.get_mut(&protocol) {
                stats.total_bytes_sent += bytes_sent;
                stats.total_bytes_received += bytes_received;
                stats.total_requests += 1;
            }
        }
    }

    /// Remove a connection
    pub async fn remove_connection(&self, connection_id: &str) {
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.remove(connection_id) {
            let protocol = conn.protocol.clone();
            drop(connections);

            // Update protocol stats
            let mut protocol_stats = self.protocol_stats.write().await;
            if let Some(stats) = protocol_stats.get_mut(&protocol) {
                stats.active_connections = stats.active_connections.saturating_sub(1);
            }

            debug!("Removed connection: {}", connection_id);
        }
    }

    /// Mark a connection as failed
    pub async fn mark_connection_failed(&self, protocol: String) {
        let mut protocol_stats = self.protocol_stats.write().await;
        let stats = protocol_stats.entry(protocol.clone()).or_insert_with(|| ProtocolMetrics {
            protocol_name: protocol,
            total_connections: 0,
            active_connections: 0,
            failed_connections: 0,
            total_bytes_sent: 0,
            total_bytes_received: 0,
            total_requests: 0,
            avg_response_time_ms: 0.0,
            uptime_seconds: 0,
        });
        stats.failed_connections += 1;
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> ProxyMetrics {
        let connections = self.connections.read().await;
        let protocol_stats = self.protocol_stats.read().await;

        let uptime_seconds = self.start_time.elapsed().as_secs();
        let total_connections = protocol_stats.values().map(|p| p.total_connections).sum();
        let active_connections = connections.len() as u64;

        // Update protocol uptime
        let mut protocols = HashMap::new();
        for (name, mut stats) in protocol_stats.clone() {
            stats.uptime_seconds = uptime_seconds;
            protocols.insert(name, stats);
        }

        ProxyMetrics {
            uptime_seconds,
            total_connections,
            active_connections,
            protocols,
            memory_usage_mb: get_memory_usage_mb(),
            cpu_usage_percent: get_cpu_usage(),
        }
    }

    /// Get active connections (serializable format)
    pub async fn get_connections(&self) -> Vec<ConnectionMetrics> {
        let connections = self.connections.read().await;
        connections.values().map(|conn| ConnectionMetrics {
            connection_id: conn.connection_id.clone(),
            protocol: conn.protocol.clone(),
            remote_address: conn.remote_address.clone(),
            connected_at_timestamp: conn.connected_at_timestamp,
            bytes_sent: conn.bytes_sent,
            bytes_received: conn.bytes_received,
            requests_processed: conn.requests_processed,
            last_activity_timestamp: conn.last_activity_timestamp,
        }).collect()
    }

    /// Kill a specific connection
    pub async fn kill_connection(&self, connection_id: &str) -> bool {
        // For now, just remove from metrics
        // In a real implementation, this would also close the actual connection
        let mut connections = self.connections.write().await;
        connections.remove(connection_id).is_some()
    }
}

// Helper functions for system metrics
fn get_memory_usage_mb() -> f64 {
    // Simple approximation - in a real implementation, you'd use system APIs
    // or libraries like `sysinfo` to get actual memory usage
    0.0
}

fn get_cpu_usage() -> f64 {
    // Simple approximation - in a real implementation, you'd use system APIs
    // or libraries like `sysinfo` to get actual CPU usage
    0.0
}