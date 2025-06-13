# NEO6 Runtime Environment

The **NEO6 Runtime Environment** is a complete, self-contained deployment package that includes all necessary components to run a full NEO6 system. This runtime provides pre-built binaries, configuration files, protocol libraries, and management tools for immediate deployment and operation.

## 🎯 Runtime Purpose

Provide a ready-to-deploy NEO6 environment that can be immediately started and used for:
- **Development and Testing**: Complete NEO6 stack for development work
- **Proof of Concept**: Demonstration environments for stakeholders
- **Production Deployment**: Production-ready configurations with proper tooling
- **Educational Purposes**: Learning and training environments

## 📦 Runtime Components

### Executive Binaries
- **neo6-admin**: Administrative server for proxy management and monitoring
- **neo6-proxy**: High-performance transaction proxy server
- **Protocol Libraries**: Dynamic protocol handlers (.so/.dylib files)

### Configuration Management
- **Admin Configuration**: Central administration and monitoring settings
- **Proxy Configuration**: Transaction routing and protocol configurations
- **Protocol Configuration**: Individual protocol handler settings
- **Environment Configuration**: Runtime-specific environment variables

### Web Interface
- **Administrative Dashboard**: Real-time monitoring and management interface
- **API Endpoints**: RESTful API for programmatic access
- **Static Assets**: Web dashboard resources and documentation

### Control and Management
- **Control Script**: Unified start/stop/status management (`neo6.sh`)
- **Log Management**: Centralized logging with rotation and archival
- **Health Monitoring**: Built-in health checks and status reporting
- **Process Management**: PID tracking and process lifecycle management

## 🗂️ Directory Structure

```
runtime/
├── neo6.sh                       # Main control script
├── README.md                     # This documentation
├── bin/                          # Executable binaries
│   ├── neo6-admin                # Administration server binary
│   └── neo6-proxy                # Proxy server binary
├── lib/                          # Protocol shared libraries
│   ├── libneo6_protocols_lib.so  # Core protocol library
│   ├── liblu62.so                # LU6.2 protocol handler
│   ├── libmq.so                  # IBM MQ protocol handler
│   ├── librest.so                # REST protocol handler
│   ├── libtcp.so                 # TCP protocol handler
│   ├── libjca.so                 # JCA protocol handler
│   └── libtn3270.so              # TN3270 protocol handler
├── config/                       # Configuration files
│   ├── admin/                    # Admin server configuration
│   │   └── admin.yaml            # Admin server settings
│   └── proxy/                    # Proxy configuration
│       ├── default.toml          # Default proxy configuration
│       ├── transactions.yaml     # Transaction definitions
│       ├── protocols.yaml        # Protocol configurations
│       └── screens/              # TN3270 screen templates
│           └── README.md         # Screen template documentation
├── static/                       # Web dashboard assets
│   ├── dashboard.html            # Main dashboard interface
│   ├── css/                      # Stylesheets
│   ├── js/                       # JavaScript files
│   └── assets/                   # Images and other assets
├── logs/                         # Runtime log files
│   ├── neo6-admin.log            # Admin server logs
│   ├── neo6-proxy.log            # Proxy server logs
│   ├── neo6-admin.pid            # Admin server PID file
│   └── neo6-proxy.pid            # Proxy server PID file
└── docs/                         # Runtime documentation
    ├── quick-start.md            # Quick start guide
    ├── configuration.md          # Configuration reference
    └── troubleshooting.md        # Common issues and solutions
```

## 🚀 Quick Start Guide

### Prerequisites
- **Operating System**: Linux, macOS, or Windows (WSL)
- **Architecture**: x86_64 or ARM64
- **Memory**: Minimum 512MB RAM, recommended 2GB+
- **Disk Space**: Minimum 100MB free space
- **Network**: Open ports 8090 (admin) and configurable proxy ports

### Starting NEO6
```bash
# Navigate to runtime directory
cd /path/to/neo6/runtime/

# Start the complete NEO6 environment
./neo6.sh start

# Verify startup
./neo6.sh status
```

### Accessing the Dashboard
Once started, access the NEO6 administrative interface:
- **Dashboard**: http://localhost:8090
- **API Endpoints**: http://localhost:8090/api
- **Health Check**: http://localhost:8090/health
- **Metrics**: http://localhost:8090/metrics

### Basic Operations
```bash
# Check system status
./neo6.sh status

# Stop NEO6 environment
./neo6.sh stop

# Restart all services
./neo6.sh restart

# View logs
./neo6.sh logs

# Check configuration
./neo6.sh config
```

## ⚙️ Configuration Overview

### Admin Configuration (config/admin/admin.yaml)
```yaml
# NEO6 Administrative Server Configuration
admin:
  port: 8090                      # Admin dashboard port
  bind_address: "0.0.0.0"         # Bind to all interfaces
  log_level: "info"               # Logging level
  max_connections: 100            # Maximum concurrent connections
  
# Protocol library path
library_path: "./lib"

# Managed proxy instances
proxy_instances:
  - name: "primary-proxy"
    protocol: "rest"              # Primary protocol
    port: 8080                    # Service port
    admin_port: 4001              # Administrative control port
    auto_start: true              # Start automatically
    config_path: "./config/proxy"
    binary_path: "./bin/neo6-proxy"
    
  - name: "tn3270-proxy"
    protocol: "tn3270"
    port: 2323                    # TN3270 port
    admin_port: 3323
    auto_start: true
    config_path: "./config/proxy"
    binary_path: "./bin/neo6-proxy"

# Monitoring and health checks
monitoring:
  enabled: true
  health_check_interval: 30       # Seconds
  metrics_collection: true
  log_rotation: true

# Security settings
security:
  enable_tls: false               # Set to true for production
  admin_auth: false               # Set to true for production
  allowed_origins: ["*"]          # Configure for production
```

### Proxy Configuration (config/proxy/default.toml)
```toml
[server]
host = "0.0.0.0"
port = 8080
admin_port = 4001
max_connections = 1000
timeout = 30000

[protocols]
library_path = "../lib/"
auto_load = true
protocols = ["rest", "tcp", "tn3270"]

[logging]
level = "info"
format = "structured"
output = ["stdout", "file"]
file_path = "../logs/neo6-proxy.log"

[metrics]
enabled = true
endpoint = "/metrics"
collect_interval = 60

# TN3270 specific configuration
[tn3270]
screen_templates = "./screens/"
charset = "ebcdic-cp037"
timeout = 60
max_sessions = 100
```

### Transaction Configuration (config/proxy/transactions.yaml)
```yaml
transactions:
  CUSTOMER_INQUIRY:
    description: "Customer account inquiry transaction"
    protocol: "tn3270"
    server: "mainframe.company.com:3270"
    timeout: 30000
    screen_template: "customer_inquiry.txt"
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
    description: "Account balance update transaction"
    protocol: "rest"
    endpoint: "http://backend.company.com/api/balance"
    method: "POST"
    timeout: 15000
    parameters:
      - name: "account_number"
        type: "string"
        required: true
        pattern: "^[0-9]{10}$"
      - name: "amount"
        type: "decimal"
        required: true
        min: 0.01
        max: 999999.99
```

## 🛠️ Control Script Operations

### Main Control Commands
```bash
# Start NEO6 environment
./neo6.sh start

# Stop NEO6 environment
./neo6.sh stop

# Restart NEO6 environment
./neo6.sh restart

# Check status
./neo6.sh status

# View real-time logs
./neo6.sh logs [--follow] [--component=admin|proxy]

# Reload configuration
./neo6.sh reload

# Health check
./neo6.sh health

# Display configuration
./neo6.sh config [--component=admin|proxy]
```

### Advanced Operations
```bash
# Start specific component
./neo6.sh start --component=admin
./neo6.sh start --component=proxy

# Debug mode with verbose logging
./neo6.sh start --debug

# Start with custom configuration
./neo6.sh start --config=/path/to/custom/config

# Performance monitoring
./neo6.sh monitor

# Backup configuration
./neo6.sh backup

# Update runtime
./neo6.sh update
```

## 📊 Runtime Monitoring

### System Status Information
The runtime provides comprehensive status information:
```bash
$ ./neo6.sh status

NEO6 Runtime Status Report
==========================

System Information:
  Runtime Version: v0.1.0
  Platform: Linux x86_64
  Uptime: 2 hours, 34 minutes
  
Administrative Server:
  Status: Running (PID: 12345)
  Port: 8090
  Dashboard: http://localhost:8090
  Memory Usage: 45.2 MB
  
Proxy Instances:
  primary-proxy:
    Status: Running (PID: 12346)
    Protocol: REST
    Port: 8080
    Active Connections: 23
    Requests/sec: 150
    
  tn3270-proxy:
    Status: Running (PID: 12347)
    Protocol: TN3270
    Port: 2323
    Active Sessions: 8
    
Protocol Libraries:
  librest.so: Loaded
  libtn3270.so: Loaded
  libtcp.so: Available
  libmq.so: Available
  
Health Status: All systems operational
```

### Log File Management
```bash
# View recent admin logs
tail -f logs/neo6-admin.log

# View proxy logs with filtering
grep "ERROR" logs/neo6-proxy.log

# Log rotation (automatic)
# Logs are automatically rotated when they exceed 100MB
# Up to 10 historical log files are maintained
```

### Performance Metrics
Access built-in metrics through multiple interfaces:
- **Web Dashboard**: Real-time metrics visualization
- **Prometheus Endpoint**: http://localhost:8090/metrics
- **CLI Status**: `./neo6.sh monitor`
- **API Endpoint**: http://localhost:8090/api/metrics

## 🔧 Customization and Extension

### Adding Custom Protocols
```bash
# Add new protocol library
cp /path/to/custom_protocol.so lib/

# Update configuration
vim config/proxy/protocols.yaml

# Restart to load new protocol
./neo6.sh restart
```

### Custom Screen Templates (TN3270)
```bash
# Add new screen template
vim config/proxy/screens/custom_screen.txt

# Update transaction configuration
vim config/proxy/transactions.yaml

# Reload configuration
./neo6.sh reload
```

### Environment Customization
```bash
# Set custom environment variables
export NEO6_LOG_LEVEL=debug
export NEO6_ADMIN_PORT=9090
export NEO6_LIBRARY_PATH=/custom/lib/path

# Start with environment settings
./neo6.sh start
```

## 🔐 Security Considerations

### Production Security Settings
```yaml
# config/admin/admin.yaml - Production settings
admin:
  bind_address: "127.0.0.1"       # Restrict to localhost
  
security:
  enable_tls: true                # Enable HTTPS
  admin_auth: true                # Require authentication
  allowed_origins: ["https://dashboard.company.com"]
  session_timeout: 3600           # 1 hour session timeout
  
# Enable audit logging
audit:
  enabled: true
  log_path: "./logs/audit.log"
  include_requests: true
  include_responses: false        # Don't log sensitive data
```

### Network Security
- **Firewall Rules**: Configure firewall to allow only necessary ports
- **VPN Access**: Use VPN for remote administrative access
- **Certificate Management**: Use proper SSL certificates for production
- **Access Control**: Implement proper authentication and authorization

## 🚨 Troubleshooting

### Common Issues and Solutions

#### Service Won't Start
```bash
# Check for port conflicts
netstat -tulpn | grep :8090

# Check permissions
ls -la neo6.sh
chmod +x neo6.sh

# Check library dependencies
ldd bin/neo6-admin
export LD_LIBRARY_PATH=$PWD/lib:$LD_LIBRARY_PATH
```

#### Configuration Issues
```bash
# Validate configuration syntax
./neo6.sh config --validate

# Reset to default configuration
./neo6.sh reset --config

# Check configuration templates
./neo6.sh config --template
```

#### Performance Issues
```bash
# Monitor resource usage
./neo6.sh monitor --detailed

# Check system resources
free -h
df -h
top -p $(cat logs/neo6-admin.pid)

# Analyze logs for bottlenecks
grep "SLOW" logs/neo6-proxy.log
```

#### Protocol Issues
```bash
# Test protocol connectivity
./neo6.sh test --protocol=tn3270 --target=mainframe.company.com:3270

# Reload protocol libraries
./neo6.sh reload --protocols

# Check protocol library versions
./neo6.sh info --protocols
```

## 📋 Build Information

The runtime includes build metadata for tracking and support:
```
Build Information:
==================
Version: v0.1.0
Build Type: Release
Build Date: June 12, 2025
Build Platform: Linux x86_64
Compiler: rustc 1.70.0
Source Commit: dd87bd9eebec45dd41f85a901315391e9bd6e51e
Workspace: /home/carlesabarca/MyProjects/NEO6

Component Versions:
- neo6-admin: v0.1.0
- neo6-proxy: v0.1.0  
- neo6-protocols-lib: v0.1.0
- Protocol Libraries: v0.1.0

Dependencies:
- Tokio: v1.38
- Axum: v0.7
- Serde: v1.0
```

## 🔄 Updates and Maintenance

### Updating the Runtime
```bash
# Check for updates
./neo6.sh update --check

# Download and install updates
./neo6.sh update --install

# Backup before update
./neo6.sh backup --full

# Rollback if needed
./neo6.sh rollback --version=previous
```

### Maintenance Tasks
```bash
# Clean up old logs
./neo6.sh cleanup --logs --older-than=30d

# Optimize configuration
./neo6.sh optimize --config

# System health check
./neo6.sh doctor

# Generate support bundle
./neo6.sh support-bundle
```

---

The **NEO6 Runtime Environment** provides a complete, production-ready deployment of the NEO6 platform with comprehensive management tools, monitoring capabilities, and operational procedures to ensure reliable and efficient operation in any environment.
