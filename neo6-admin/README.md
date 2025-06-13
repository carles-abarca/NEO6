# NEO6 Admin

**NEO6 Admin** is the central administrative server for managing and monitoring NEO6 proxy instances. It provides a comprehensive web-based dashboard, RESTful API, and command-line interface for orchestrating multiple proxy servers, monitoring system health, and managing configurations across the entire NEO6 ecosystem.

## 🎯 Purpose

The NEO6 Admin server serves as the control plane for NEO6 deployments, enabling:
- **Centralized Management**: Single point of control for multiple proxy instances
- **Real-Time Monitoring**: Live monitoring of system health and performance
- **Configuration Management**: Dynamic configuration updates without service interruption
- **Operational Oversight**: Comprehensive operational dashboards and reporting
- **Automated Operations**: Intelligent automation and self-healing capabilities

## 🚀 Key Features

### Administrative Dashboard
- **Web-Based Interface**: Modern, responsive web dashboard for system management
- **Real-Time Metrics**: Live performance monitoring with interactive charts
- **System Health**: Comprehensive health checks and status reporting
- **Configuration Editor**: In-browser configuration editing with validation
- **Log Viewer**: Centralized log viewing and filtering capabilities

### Proxy Management
- **Lifecycle Management**: Start, stop, restart, and configure proxy instances
- **Dynamic Scaling**: Automatic scaling based on load and performance metrics
- **Load Balancing**: Intelligent load distribution across proxy instances
- **Health Monitoring**: Continuous health monitoring with automatic recovery
- **Version Management**: Rolling updates and version management

### API Endpoints
- **RESTful API**: Complete REST API for programmatic access
- **Webhook Support**: Event-driven integrations with external systems
- **Batch Operations**: Bulk operations for managing multiple instances
- **Status Reporting**: Detailed status and metrics reporting
- **Configuration API**: Dynamic configuration management through API

### Operational Features
- **Audit Logging**: Comprehensive audit trail for all administrative actions
- **Role-Based Access**: Configurable access control and user management
- **Backup Management**: Automated backup and recovery procedures
- **Performance Analytics**: Historical performance analysis and trending
- **Alerting System**: Configurable alerts and notification channels

## 🏗️ Architecture Components

### Web Server
- **Framework**: Built on Axum with async/await support
- **Static Assets**: Serves dashboard HTML, CSS, and JavaScript
- **API Router**: RESTful API endpoints with middleware support
- **WebSocket Support**: Real-time updates for dashboard components
- **Security**: TLS support, authentication, and authorization

### Proxy Manager
- **Process Management**: Manages proxy instance lifecycles
- **Configuration Management**: Handles proxy configuration updates
- **Health Monitoring**: Monitors proxy health and performance
- **Load Balancer**: Distributes load across healthy instances
- **Service Discovery**: Automatic discovery and registration of services

### Monitoring Engine
- **Metrics Collection**: Gathers metrics from all managed components
- **Health Checks**: Performs regular health assessments
- **Performance Monitoring**: Tracks performance trends and anomalies
- **Alert Generation**: Generates alerts based on configurable rules
- **Data Aggregation**: Aggregates and stores historical data

## 📡 API Reference

### Proxy Management Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/proxies` | List all managed proxy instances |
| `POST` | `/api/proxies` | Create new proxy instance |
| `GET` | `/api/proxies/{id}` | Get specific proxy details |
| `PUT` | `/api/proxies/{id}` | Update proxy configuration |
| `DELETE` | `/api/proxies/{id}` | Remove proxy instance |
| `POST` | `/api/proxies/{id}/start` | Start proxy instance |
| `POST` | `/api/proxies/{id}/stop` | Stop proxy instance |
| `POST` | `/api/proxies/{id}/restart` | Restart proxy instance |
| `GET` | `/api/proxies/{id}/status` | Get proxy status |
| `GET` | `/api/proxies/{id}/logs` | Get proxy logs |

### System Management Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/system/status` | Overall system status |
| `GET` | `/api/system/health` | System health check |
| `GET` | `/api/system/metrics` | System metrics |
| `POST` | `/api/system/reload` | Reload system configuration |
| `GET` | `/api/system/info` | System information |
| `POST` | `/api/system/backup` | Create system backup |
| `POST` | `/api/system/restore` | Restore from backup |

### Configuration Management Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/config` | Get current configuration |
| `PUT` | `/api/config` | Update configuration |
| `POST` | `/api/config/validate` | Validate configuration |
| `GET` | `/api/config/templates` | Get configuration templates |
| `GET` | `/api/config/history` | Configuration change history |

## 🛠️ Configuration

### Main Configuration (config/admin.yaml)
```yaml
# NEO6 Admin Server Configuration
admin:
  port: 8090                      # Web server port
  bind_address: "0.0.0.0"         # Bind address
  log_level: "info"               # Logging level
  max_connections: 100            # Maximum concurrent connections
  dashboard_enabled: true         # Enable web dashboard
  api_enabled: true               # Enable REST API

# Static file serving
static:
  enabled: true
  path: "./static"                # Static files directory
  cache_max_age: 3600             # Cache timeout in seconds

# Security configuration
security:
  tls_enabled: false              # Enable TLS/HTTPS
  cert_file: ""                   # TLS certificate file
  key_file: ""                    # TLS private key file
  auth_enabled: false             # Enable authentication
  jwt_secret: ""                  # JWT signing secret
  session_timeout: 3600           # Session timeout in seconds

# Protocol library configuration
library_path: "./lib"             # Protocol libraries directory

# Proxy instance definitions
proxy_instances:
  - name: "primary-proxy"
    description: "Primary REST proxy"
    protocol: "rest"
    port: 8080
    admin_port: 4001
    auto_start: true
    config_path: "./config/proxy"
    binary_path: "./bin/neo6-proxy"
    environment:
      LOG_LEVEL: "info"
      PROTOCOL_LIB_PATH: "./lib"
    
  - name: "tn3270-proxy"
    description: "TN3270 terminal proxy"
    protocol: "tn3270"
    port: 2323
    admin_port: 3323
    auto_start: true
    config_path: "./config/proxy"
    binary_path: "./bin/neo6-proxy"
    environment:
      LOG_LEVEL: "info"
      TN3270_SCREENS_PATH: "./config/proxy/screens"

# Monitoring configuration
monitoring:
  enabled: true
  metrics_enabled: true
  health_check_interval: 30       # Health check interval in seconds
  metrics_collection_interval: 60 # Metrics collection interval
  retention_days: 30              # Data retention period
  
# Logging configuration
logging:
  level: "info"
  format: "json"                  # Log format: "json" or "text"
  output: ["stdout", "file"]      # Output destinations
  file_path: "./logs/neo6-admin.log"
  max_size: "100MB"               # Maximum log file size
  max_files: 10                   # Maximum number of log files
  
# Alerting configuration
alerting:
  enabled: true
  smtp_server: ""                 # SMTP server for email alerts
  email_from: ""                  # From email address
  email_to: []                    # Default email recipients
  slack_webhook: ""               # Slack webhook URL
  alert_rules:
    - name: "proxy_down"
      condition: "proxy_status == 'down'"
      severity: "critical"
      cooldown: 300               # Cooldown period in seconds
    - name: "high_cpu"
      condition: "cpu_usage > 80"
      severity: "warning"
      cooldown: 600

# Backup configuration
backup:
  enabled: true
  schedule: "0 2 * * *"           # Cron expression for backup schedule
  retention_days: 7               # Backup retention period
  backup_path: "./backups"        # Backup storage directory
  include:
    - "config/"
    - "logs/"
    - "data/"
```

### Proxy Instance Configuration
```yaml
# Individual proxy instance configuration
proxy_instance:
  name: "example-proxy"
  description: "Example proxy instance"
  protocol: "rest"
  
  # Network configuration
  network:
    port: 8080
    admin_port: 4001
    bind_address: "0.0.0.0"
    max_connections: 1000
    
  # Resource limits
  resources:
    memory_limit: "1GB"
    cpu_limit: "1000m"
    file_descriptors: 1024
    
  # Health check configuration
  health_check:
    enabled: true
    interval: 30
    timeout: 10
    retries: 3
    endpoint: "/health"
    
  # Auto-scaling configuration
  auto_scaling:
    enabled: false
    min_instances: 1
    max_instances: 5
    cpu_threshold: 70
    memory_threshold: 80
    
  # Environment variables
  environment:
    LOG_LEVEL: "info"
    RUST_BACKTRACE: "1"
    PROTOCOL_LIB_PATH: "./lib"
```

## 🖥️ Web Dashboard

### Dashboard Features
- **System Overview**: High-level system status and metrics
- **Proxy Management**: Visual proxy instance management
- **Real-Time Monitoring**: Live charts and metrics
- **Configuration Editor**: In-browser configuration editing
- **Log Viewer**: Centralized log viewing and search
- **Alert Management**: Alert configuration and history

### Dashboard Sections

#### System Status Panel
```
┌─────────────────────────────────────────────────────┐
│ NEO6 System Overview                                │
├─────────────────────────────────────────────────────┤
│ Status: ● Healthy        Uptime: 2d 4h 23m         │
│ Proxies: 3 running       Memory: 256MB / 1GB       │
│ Requests: 1,234/min      CPU: 12%                  │
│ Errors: 0.1%             Disk: 2.1GB / 10GB        │
└─────────────────────────────────────────────────────┘
```

#### Proxy Instance Management
```
┌─────────────────────────────────────────────────────┐
│ Proxy Instances                          [+ Add]    │
├─────────────────────────────────────────────────────┤
│ ● primary-proxy     REST      :8080    [Actions ▼] │
│ ● tn3270-proxy     TN3270     :2323    [Actions ▼] │
│ ● mq-proxy         MQ         :4000    [Actions ▼] │
└─────────────────────────────────────────────────────┘
```

#### Metrics Dashboard
- **Request Rate**: Real-time request throughput
- **Response Time**: Latency percentiles and averages
- **Error Rate**: Error rate tracking and alerting
- **Resource Usage**: CPU, memory, and disk utilization
- **Connection Count**: Active connection monitoring

## 🔧 Command Line Interface

### Basic Operations
```bash
# Start NEO6 Admin server
cargo run -- --config config/admin.yaml

# Start with custom configuration
cargo run -- --config /path/to/config.yaml

# Enable debug logging
cargo run -- --config config/admin.yaml --log-level debug

# Start on specific port
cargo run -- --config config/admin.yaml --port 9090

# Display help
cargo run -- --help
```

### Development Mode
```bash
# Run in development mode with auto-reload
cargo watch -x "run -- --config config/admin.yaml"

# Run with environment variables
LOG_LEVEL=debug cargo run -- --config config/admin.yaml

# Run tests
cargo test

# Build release version
cargo build --release
```

## 📊 Monitoring and Metrics

### Built-in Metrics
- **System Metrics**: CPU, memory, disk usage
- **Proxy Metrics**: Request rate, response time, error rate
- **Network Metrics**: Connection count, bandwidth usage
- **Application Metrics**: Business-specific metrics
- **Health Metrics**: Component health status

### Prometheus Integration
```yaml
# Prometheus scraping configuration
scrape_configs:
  - job_name: 'neo6-admin'
    static_configs:
      - targets: ['localhost:8090']
    metrics_path: '/api/metrics'
    scrape_interval: 30s
```

### Grafana Dashboard
The admin server provides Grafana dashboard templates for:
- System overview and health status
- Proxy performance monitoring
- Error rate and alerting
- Resource utilization tracking
- Business metrics visualization

## 🔐 Security and Access Control

### Authentication Methods
- **Local Authentication**: Username/password authentication
- **JWT Tokens**: JSON Web Token-based authentication
- **API Keys**: API key-based authentication for automation
- **LDAP Integration**: Integration with LDAP/Active Directory
- **OAuth2**: OAuth2 provider integration

### Authorization Model
```yaml
# Role-based access control configuration
rbac:
  enabled: true
  roles:
    admin:
      permissions: ["*"]
      description: "Full administrative access"
      
    operator:
      permissions: 
        - "proxy:read"
        - "proxy:start"
        - "proxy:stop"
        - "system:read"
      description: "Operational access"
      
    viewer:
      permissions:
        - "proxy:read"
        - "system:read"
        - "metrics:read"
      description: "Read-only access"
```

### Security Best Practices
- Enable TLS for all communications
- Use strong authentication methods
- Implement proper access controls
- Regular security audits and updates
- Network segmentation and firewalls
- Secure configuration management

## 🚨 Troubleshooting

### Common Issues

#### Admin Server Won't Start
```bash
# Check port availability
netstat -tulpn | grep :8090

# Check configuration syntax
./config/validate-config.sh config/admin.yaml

# Check log files
tail -f logs/neo6-admin.log

# Check file permissions
ls -la config/admin.yaml
```

#### Proxy Management Issues
```bash
# Check proxy binary exists
ls -la bin/neo6-proxy

# Test proxy connectivity
curl http://localhost:8080/health

# Check proxy logs
tail -f logs/neo6-proxy-primary.log

# Verify library path
ls -la lib/
export LD_LIBRARY_PATH=$PWD/lib:$LD_LIBRARY_PATH
```

#### Dashboard Access Issues
```bash
# Check static files
ls -la static/

# Test API endpoints
curl http://localhost:8090/api/system/status

# Check network connectivity
telnet localhost 8090

# Verify firewall rules
sudo iptables -L | grep 8090
```

### Log Analysis
```bash
# Filter error logs
grep "ERROR" logs/neo6-admin.log

# Monitor real-time logs
tail -f logs/neo6-admin.log | grep -i "error\|warn"

# Analyze performance
grep "slow\|timeout" logs/neo6-admin.log

# Check proxy operations
grep "proxy.*start\|stop\|restart" logs/neo6-admin.log
```

## 🔄 Deployment and Operations

### Production Deployment
```bash
# Build release binary
cargo build --release

# Create service user
sudo useradd -r -s /bin/false neo6

# Install systemd service
sudo cp scripts/neo6-admin.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable neo6-admin
sudo systemctl start neo6-admin
```

### Docker Deployment
```dockerfile
FROM rust:1.70-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:12-slim
RUN apt-get update && apt-get install -y ca-certificates
WORKDIR /app
COPY --from=builder /app/target/release/neo6-admin .
COPY config/ config/
COPY static/ static/
EXPOSE 8090
CMD ["./neo6-admin", "--config", "config/admin.yaml"]
```

### Kubernetes Deployment
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: neo6-admin
spec:
  replicas: 1
  selector:
    matchLabels:
      app: neo6-admin
  template:
    metadata:
      labels:
        app: neo6-admin
    spec:
      containers:
      - name: neo6-admin
        image: neo6/admin:latest
        ports:
        - containerPort: 8090
        env:
        - name: NEO6_CONFIG
          value: "/config/admin.yaml"
        volumeMounts:
        - name: config
          mountPath: /config
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
      volumes:
      - name: config
        configMap:
          name: neo6-admin-config
```

## 📈 Performance and Scaling

### Performance Characteristics
- **Concurrent Users**: Supports 100+ concurrent dashboard users
- **API Throughput**: 1,000+ API requests per second
- **Memory Usage**: <256MB base memory footprint
- **Response Time**: <100ms for most API operations
- **Proxy Management**: Can manage 50+ proxy instances efficiently

### Scaling Considerations
- **Horizontal Scaling**: Deploy multiple admin instances behind load balancer
- **Database Scaling**: Use external database for large deployments
- **Caching**: Implement Redis caching for improved performance
- **CDN**: Use CDN for static asset delivery
- **Resource Limits**: Configure appropriate CPU and memory limits

---

**NEO6 Admin** provides comprehensive administrative capabilities for managing NEO6 deployments, offering both user-friendly web interfaces and powerful programmatic APIs to ensure efficient operation and monitoring of complex NEO6 environments.