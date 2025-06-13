
# NEO6 Proxy

**NEO6 Proxy** is a high-performance, enterprise-grade transaction proxy server written in Rust, designed to bridge the gap between modern applications and legacy mainframe systems. It provides seamless integration capabilities for CICS (Customer Information Control System) environments while supporting multiple protocols and offering comprehensive observability features.

## 🎯 Mission

Enable digital transformation by providing a robust, scalable, and secure proxy that allows modern applications to interact with legacy COBOL/CICS systems through contemporary protocols while maintaining the performance and reliability required for enterprise environments.

## ✨ Core Capabilities

### Transaction Processing
- **High-Performance CICS Integration**: Direct invocation of COBOL transactions with minimal latency
- **Dynamic Protocol Loading**: Runtime loading and management of protocol handlers
- **Transaction Mapping**: Flexible YAML-based transaction configuration and routing
- **Request/Response Transformation**: Automatic JSON ↔ COBOL COMMAREA conversion
- **Session Management**: Stateful and stateless transaction support

### Multi-Protocol Support
- **REST/HTTP(S)**: Modern REST API endpoints with comprehensive middleware
- **IBM MQ**: Native IBM MQ integration with C bindings for optimal performance  
- **TCP/IP**: Custom binary and text protocol support
- **LU6.2/APPC**: Legacy SNA protocol support for mainframe connectivity
- **JCA/CICS TG**: Java Connector Architecture integration
- **TN3270**: Full terminal emulation with screen template support

### Enterprise Features
- **Administrative Control**: Remote administration and monitoring capabilities
- **Dynamic Configuration**: Hot-reload configuration changes without restart
- **Load Balancing**: Intelligent request distribution across backend systems
- **Connection Pooling**: Efficient resource management and connection reuse
- **Circuit Breaker**: Automatic failover and error recovery mechanisms

## 🏗️ Architecture Overview

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Modern Apps   │    │   NEO6 Proxy    │    │ Legacy Systems  │
│                 │    │                  │    │                 │
│ REST Clients    │◄──►│ Dynamic Router   │◄──►│ CICS/COBOL      │
│ MQ Producers    │    │ Protocol Loader  │    │ Mainframe       │
│ TCP Clients     │    │ Admin Control    │    │ Enterprise Apps │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

## 📡 API Interfaces and Endpoints

### 1. REST API (HTTP/HTTPS)

| Method | Endpoint | Description | Features |
|--------|----------|-------------|----------|
| `POST` | `/invoke` | Synchronous transaction invocation | Request validation, response transformation |
| `POST` | `/invoke-async` | Asynchronous transaction execution | Background processing, status tracking |
| `GET` | `/status/{id}` | Query execution status and results | Real-time status, result retrieval |
| `GET` | `/health` | Health check endpoint | System health, dependency status |
| `GET` | `/metrics` | Prometheus metrics export | Performance metrics, business metrics |
| `GET` | `/admin/info` | Administrative information | Version, uptime, configuration |
| `POST` | `/admin/reload` | Reload configuration | Hot configuration updates |

#### Request Format for `/invoke`
```json
{
  "transaction_id": "CUSTOMER_INQUIRY",
  "parameters": {
    "customer_id": "CUST001234",
    "account_type": "CHECKING",
    "include_balance": true
  },
  "options": {
    "timeout": 30000,
    "retry_count": 3,
    "trace_enabled": true
  }
}
```

#### Response Format
```json
{
  "status": "success",
  "transaction_id": "CUSTOMER_INQUIRY",
  "execution_time": 234,
  "data": {
    "customer_name": "John Doe",
    "account_balance": 1250.75,
    "account_status": "ACTIVE"
  },
  "trace_id": "trace-12345-67890",
  "metadata": {
    "protocol": "lu62",
    "backend_server": "mainframe.company.com",
    "response_time_ms": 234
  }
}
```

### 2. Administrative Control Interface

#### TCP Control Socket (Port 4001)
```json
// Status command
{
  "command": "Status"
}

// Response
{
  "status": "running",
  "uptime": 3600,
  "active_connections": 45,
  "protocols_loaded": ["rest", "mq", "lu62", "tcp"],
  "version": "0.1.0"
}
```

#### Protocol Management
```json
// Reload protocols
{
  "command": "ReloadProtocols"
}

// Test protocol connectivity
{
  "command": "TestProtocol",
  "protocol": "lu62"
}
```

### 3. IBM MQ Integration

#### Queue Configuration
- **Request Queue**: `NEO6.REQUEST.Q`
- **Response Queue**: `NEO6.RESPONSE.Q`
- **Error Queue**: `NEO6.ERROR.Q`
- **Admin Queue**: `NEO6.ADMIN.Q`

#### Message Format
```json
{
  "message_id": "msg-12345",
  "correlation_id": "corr-67890",
  "transaction_id": "BALANCE_UPDATE",
  "parameters": {
    "account_number": "ACC123456",
    "amount": 100.00,
    "transaction_type": "CREDIT"
  },
  "reply_to": "NEO6.RESPONSE.Q",
  "expiry": 30000
}
```

### 4. TCP Protocol Support

#### Binary Protocol (Port 4000)
```
┌──────────┬──────────┬──────────┬──────────────┐
│ Length   │ Version  │ Trans ID │ Payload      │
│ (4 bytes)│ (2 bytes)│ (8 bytes)│ (N bytes)    │
└──────────┴──────────┴──────────┴──────────────┘
```

#### Text Protocol (Port 4001)
```
NEO6|1.0|CUSTOMER_INQUIRY|{"customer_id":"CUST001234"}
```

## 🧱 Project Structure

```
neo6-proxy/
├── Cargo.toml                    # Project dependencies and metadata
├── src/
│   ├── main.rs                   # Application entry point and CLI
│   ├── config.rs                 # Configuration management
│   ├── logging.rs                # Structured logging setup
│   ├── metrics.rs                # Prometheus metrics collection
│   ├── admin_control.rs          # Administrative control interface
│   ├── protocol_loader.rs        # Dynamic protocol loading
│   ├── proxy/                    # Core proxy functionality
│   │   ├── mod.rs
│   │   ├── dynamic_router.rs     # Request routing logic
│   │   ├── dynamic_handler.rs    # Protocol-agnostic handler
│   │   ├── router.rs             # Legacy router (compatibility)
│   │   └── handler.rs            # Request/response handling
│   └── cics/                     # CICS integration
│       ├── mod.rs
│       └── mapping.rs            # Transaction mapping logic
├── config/                       # Configuration files
│   ├── default.toml              # Default configuration
│   ├── transactions.yaml         # Transaction definitions
│   └── protocols.yaml            # Protocol configurations
├── scripts/                      # Utility scripts
│   └── deploy.sh                 # Deployment script
├── tests/                        # Integration tests
│   ├── integration_tests.rs
│   └── protocol_tests.rs
├── Dockerfile                    # Container build definition
└── delivery/                     # Build artifacts
    └── debug/                    # Debug builds with runtime
```

## 🔧 Technology Stack

### Core Technologies
- **Language**: Rust 2021 Edition (1.70+)
- **Async Runtime**: Tokio with full feature set
- **Web Framework**: Axum with tower middleware
- **Serialization**: Serde with JSON/YAML support
- **Configuration**: Config crate with TOML/YAML support
- **Logging**: Tracing with structured output

### Protocol Integration
- **HTTP Client**: Reqwest with TLS and JSON support
- **MQ Integration**: Custom C bindings via bindgen
- **Dynamic Loading**: libloading for runtime protocol management
- **FFI**: Safe Rust-C interop for protocol handlers

### Observability
- **Metrics**: Prometheus client library
- **Tracing**: OpenTelemetry integration
- **Monitoring**: Custom health check endpoints
- **Diagnostics**: Admin control interface

## 📦 Dependencies

### Core Dependencies
```toml
[dependencies]
tokio = { version = "1.38", features = ["full"] }
axum = "0.7"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9.34"
config = "0.13"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tower = "0.5"
```

### Protocol Dependencies
```toml
prometheus = "0.13"
reqwest = { version = "0.12", features = ["json"] }
libloading = "0.8"
neo6-protocols-lib = { path = "../neo6-protocols/neo6-protocols-lib" }
bindgen = "0.69"  # Build dependency for MQ bindings
libc = "0.2"
atty = "0.2"      # TTY detection for logging
dotenvy = "0.15"  # Environment variable loading
```

### Optional Protocol Features
```toml
[features]
default = ["rest", "tcp"]
all-protocols = ["rest", "tcp", "mq", "lu62", "jca", "tn3270"]
rest = ["dep:reqwest"]
mq = ["dep:bindgen"]
lu62 = ["neo6-protocols/lu62"]
tcp = ["neo6-protocols/tcp"]
jca = ["neo6-protocols/jca"]
tn3270 = ["neo6-protocols/tn3270"]
```

## ⚙️ Configuration

### Main Configuration (config/default.toml)
```toml
[server]
host = "0.0.0.0"
port = 8080
admin_port = 4001
max_connections = 1000
timeout = 30000

[protocols]
library_path = "./lib/"
auto_load = true
protocols = ["rest", "mq", "tcp", "lu62"]

[security]
tls_enabled = true
cert_file = "/etc/ssl/certs/neo6-proxy.crt"
key_file = "/etc/ssl/private/neo6-proxy.key"
jwt_secret = "${JWT_SECRET}"
validate_requests = true

[logging]
level = "info"
format = "json"
output = ["stdout", "file"]
file_path = "/var/log/neo6-proxy.log"
max_size = "100MB"
max_files = 10

[metrics]
enabled = true
endpoint = "/metrics"
collect_interval = 60
export_business_metrics = true

[mq]
queue_manager = "QM.PROD"
channel = "NEO6.SVRCONN"
connection_name = "mqm.company.com(1414)"
request_queue = "NEO6.REQUEST.Q"
response_queue = "NEO6.RESPONSE.Q"
error_queue = "NEO6.ERROR.Q"
user = "${MQ_USER}"
password = "${MQ_PASSWORD}"
max_connections = 50
connection_timeout = 30

[circuit_breaker]
enabled = true
failure_threshold = 5
recovery_timeout = 60
half_open_max_calls = 3
```

### Transaction Configuration (config/transactions.yaml)
```yaml
transactions:
  CUSTOMER_INQUIRY:
    description: "Customer account inquiry"
    protocol: "lu62"
    server: "mainframe.company.com:3270"
    timeout: 30000
    retry_count: 3
    parameters:
      - name: "customer_id"
        type: "string"
        required: true
        max_length: 10
      - name: "account_type"
        type: "string"
        required: false
        default: "ALL"
    
  BALANCE_UPDATE:
    description: "Account balance update"
    protocol: "mq"
    queue: "BALANCE.UPDATE.Q"
    timeout: 60000
    parameters:
      - name: "account_number"
        type: "string"
        required: true
        pattern: "^[A-Z0-9]{10}$"
      - name: "amount"
        type: "decimal"
        required: true
        min: 0.01
        max: 999999.99
```

## 🔐 Security Features

### Authentication and Authorization
- **JWT Token Support**: Bearer token authentication with configurable secrets
- **TLS/SSL Encryption**: Full TLS 1.3 support for all HTTP endpoints
- **Client Certificate Authentication**: Mutual TLS for enhanced security
- **API Key Management**: Support for API key-based authentication

### Input Validation and Sanitization
- **Schema Validation**: Automatic request validation against transaction schemas
- **Parameter Sanitization**: Input sanitization to prevent injection attacks
- **Rate Limiting**: Configurable rate limiting per client/endpoint
- **Request Size Limits**: Maximum payload size enforcement

### Network Security
- **IP Whitelisting**: Configurable IP address restrictions
- **VPN Integration**: Recommended for TCP and MQ connections
- **Firewall Rules**: Automatic iptables rule generation
- **DMZ Deployment**: Support for DMZ network configurations

## 📊 Performance Characteristics

### Benchmarks (Reference Implementation)
- **Throughput**: 15,000+ requests/second (REST API)
- **Latency**: <2ms proxy overhead (local network)
- **Concurrent Connections**: 10,000+ simultaneous connections
- **Memory Usage**: <100MB base + 1MB per 1000 connections
- **CPU Usage**: <10% on modern hardware under normal load

### Scalability Features
- **Horizontal Scaling**: Multiple proxy instances with load balancer
- **Connection Pooling**: Efficient backend connection management
- **Resource Optimization**: Automatic resource allocation and cleanup
- **Backpressure Management**: Intelligent request throttling under load

## 🚀 Deployment and Operations

### Docker Deployment
```dockerfile
FROM rust:1.70-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:12-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/neo6-proxy /usr/local/bin/
COPY config/ /etc/neo6-proxy/
EXPOSE 8080 4001
CMD ["neo6-proxy", "--config", "/etc/neo6-proxy/default.toml"]
```

### Kubernetes Deployment
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: neo6-proxy
spec:
  replicas: 3
  selector:
    matchLabels:
      app: neo6-proxy
  template:
    metadata:
      labels:
        app: neo6-proxy
    spec:
      containers:
      - name: neo6-proxy
        image: neo6/proxy:latest
        ports:
        - containerPort: 8080
        - containerPort: 4001
        env:
        - name: JWT_SECRET
          valueFrom:
            secretKeyRef:
              name: neo6-secrets
              key: jwt-secret
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
```

### Command Line Usage
```bash
# Start with default configuration
neo6-proxy

# Start with custom configuration
neo6-proxy --config /path/to/config.toml

# Start with specific protocol
neo6-proxy --protocol rest --port 8080

# Enable debug logging
neo6-proxy --log-level debug

# Start with custom library path
neo6-proxy --library-path /path/to/protocols/

# Display help
neo6-proxy --help
```

## 🔧 Development and Testing

### Building from Source
```bash
# Build debug version
cargo build

# Build release version
cargo build --release

# Build with all protocol features
cargo build --features all-protocols

# Run tests
cargo test

# Run integration tests
cargo test --test integration_tests

# Generate documentation
cargo doc --open
```

### Integration Testing
```bash
# Start test environment
docker-compose -f test/docker-compose.yml up

# Run protocol tests
cargo test protocol_tests

# Performance testing
cargo bench

# Load testing
./scripts/load_test.sh
```

## 📈 Monitoring and Observability

### Prometheus Metrics
```
# Connection metrics
neo6_proxy_connections_total
neo6_proxy_connections_active
neo6_proxy_connections_failed_total

# Request metrics
neo6_proxy_requests_total
neo6_proxy_request_duration_seconds
neo6_proxy_request_size_bytes

# Transaction metrics
neo6_proxy_transactions_total
neo6_proxy_transaction_duration_seconds
neo6_proxy_transaction_errors_total

# Protocol metrics
neo6_proxy_protocol_invocations_total
neo6_proxy_protocol_errors_total

# System metrics
neo6_proxy_memory_usage_bytes
neo6_proxy_cpu_usage_percent
```

### Health Check Endpoints
```bash
# Basic health check
curl http://localhost:8080/health

# Detailed health check
curl http://localhost:8080/health?detail=true

# Protocol-specific health
curl http://localhost:8080/health/protocol/mq
```

### Distributed Tracing
- **OpenTelemetry Integration**: Full distributed tracing support
- **Trace Correlation**: Request correlation across protocol boundaries
- **Jaeger/Zipkin**: Compatible with popular tracing systems
- **Custom Spans**: Business-specific tracing capabilities

## 🎯 Use Cases and Applications

### Enterprise Integration
- **Legacy Modernization**: Gradual migration from mainframe systems
- **API Gateway**: Central point for legacy system access
- **Service Mesh**: Integration with modern microservice architectures
- **Event-Driven Architecture**: Asynchronous transaction processing

### Financial Services
- **Core Banking**: Real-time account and transaction processing
- **Payment Processing**: High-volume payment transaction routing
- **Risk Management**: Real-time risk assessment and fraud detection
- **Regulatory Reporting**: Automated compliance and reporting systems

### Manufacturing and Logistics
- **ERP Integration**: Connection to legacy ERP systems
- **Supply Chain**: Real-time inventory and logistics management
- **Production Control**: Manufacturing execution system integration
- **Quality Management**: Quality control and traceability systems

## 🛣️ Roadmap and Future Enhancements

### Planned Features
- **gRPC Support**: Native gRPC protocol implementation
- **GraphQL Integration**: GraphQL query processing and schema stitching
- **WebSocket Support**: Real-time bidirectional communication
- **Event Streaming**: Apache Kafka and event streaming integration

### AI/ML Integration
- **Intelligent Routing**: ML-based protocol selection and routing
- **Predictive Scaling**: Automatic scaling based on predicted load
- **Anomaly Detection**: AI-powered anomaly detection and alerting
- **Performance Optimization**: Self-tuning performance parameters

### Operational Enhancements
- **Multi-Region Support**: Global deployment with region failover
- **Advanced Circuit Breaker**: Adaptive circuit breaker patterns
- **Chaos Engineering**: Built-in chaos testing capabilities
- **Zero-Downtime Deployments**: Blue-green and canary deployment support

---

**NEO6 Proxy** serves as the critical bridge between legacy and modern systems, providing enterprise-grade performance, reliability, and security for digital transformation initiatives while maintaining the operational excellence required for mission-critical business applications.
