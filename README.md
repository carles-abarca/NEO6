# NEO6 Project

NEO6 is a comprehensive, modular platform for legacy modernization and interoperability, focused on enabling seamless integration between modern applications and mainframe/COBOL systems. Built with high-performance Rust components and intelligent Python agents, NEO6 provides a complete ecosystem for exposing legacy transactions through modern APIs and protocols.

## 🚀 Key Features

- **High-Performance Proxy**: Rust-based proxy server with dynamic protocol loading
- **Multi-Protocol Support**: REST, MQ, TCP, LU6.2, JCA, TN3270, and more
- **Intelligent Agents**: Python-based agents for automation, monitoring, and code generation
- **Complete Runtime**: Self-contained runtime environment with web dashboard
- **Advanced Observability**: Comprehensive logging, metrics, and monitoring
- **Cloud-Ready**: Infrastructure-as-code and deployment automation

## 📁 Project Structure

### 🤖 NEO6 Agents
- **agent-runtime/**: Core Python runtime and orchestration logic for NEO6 agents. Provides MCP (Multi-Channel Protocol) server and client implementation with auxiliary libraries for easy integration and agent development.

- **aiops-agent/**: Intelligent AIOps agent that leverages `agent-runtime` to automate operational management of NEO6 cloud environments. Features container monitoring, anomaly detection, automated remediation, and integration with logging and metrics systems.

- **cobol-agent-core/**: Specialized agent for COBOL-to-Python migration. Analyzes COBOL source code, resolves dependencies, and generates equivalent Python code optimized for the NEO6 environment.

- **python-agent-core/**: Advanced Python agent for request processing and code generation. Interprets external requests and automatically generates or modifies Python code within the NEO6 environment, supporting development automation and system integration.

### 🌐 NEO6 Runtime Components
- **neo6-protocols/**: Rust workspace containing protocol adapters and FFI interfaces for all supported legacy and modern protocols. Implements the `ProtocolHandler` trait with dynamic loading capabilities:
  - `neo6-protocols-lib/`: Common protocol traits, helpers, and FFI infrastructure
  - `lu62/`, `mq/`, `rest/`, `tcp/`, `jca/`, `tn3270/`: Protocol-specific implementations with C FFI bindings
  - Advanced TN3270 support with screen template language v2.0

- **neo6-proxy/**: High-performance main proxy server written in Rust. Features dynamic protocol loading, multi-protocol endpoint support, advanced routing, CICS transaction mapping, comprehensive observability, and admin control interface.

- **neo6-admin/**: Administrative server for proxy management. Provides web dashboard, API endpoints for proxy lifecycle management, configuration management, and system monitoring.

### 🏗️ Infrastructure & Deployment
- **infra/**: Complete infrastructure-as-code solution with deployment scripts, cloud resource templates, network configurations, storage resources, and CI/CD automation.

- **runtime/**: Self-contained NEO6 runtime environment with pre-built binaries, protocol libraries, configuration files, web dashboard, and control scripts. Includes debug and production deployment configurations.

### 📚 Documentation
- **docs/**: Comprehensive technical documentation including architecture diagrams, protocol specifications, development guides, deployment instructions, and API references. Available in HTML format with interactive examples.

## 🛠️ Quick Start

### Prerequisites
- Rust 1.70+ (for building from source)
- Python 3.8+ (for agents)
- Docker (optional, for containerized deployment)

### Using Pre-built Runtime
```bash
cd runtime/
./neo6.sh start
```

### Web Dashboard
Once started, access the NEO6 Admin dashboard at:
- **Dashboard**: http://localhost:8090
- **API**: http://localhost:8090/api

### Building from Source
```bash
# Build all Rust components
cd neo6-protocols && cargo build --release
cd ../neo6-proxy && cargo build --release
cd ../neo6-admin && cargo build --release

# Install Python agent dependencies
cd agent-runtime && pip install -r requirements.txt
```

## 🏛️ Architecture

NEO6 follows a modular architecture with clear separation of concerns:

1. **Protocol Layer**: Dynamic protocol adapters with FFI interfaces
2. **Proxy Layer**: High-performance routing and transaction mapping
3. **Management Layer**: Administrative control and monitoring
4. **Agent Layer**: Intelligent automation and code generation
5. **Infrastructure Layer**: Deployment and cloud resources

## 📖 Documentation

For detailed documentation, see:
- **Architecture Guide**: `docs/architecture.html`
- **Protocol Development**: `docs/protocols.html`
- **Deployment Guide**: `docs/deploy.html`
- **Developer Guide**: `docs/devguide.html`
- **Component READMEs**: Each subfolder contains specific documentation

## 🔧 Configuration

- **Proxy Configuration**: `neo6-proxy/config/default.toml`
- **Admin Configuration**: `neo6-admin/config/admin.yaml`
- **Runtime Configuration**: `runtime/config/`

## 📊 Current Status

- **Version**: v0.1.0
- **Build Status**: Active development
- **Protocols Supported**: REST, MQ, TCP, LU6.2, JCA, TN3270
- **Platform Support**: Linux, macOS, Windows (Rust components)

---

**NEO6** - Bridging Legacy and Modern Systems with Intelligence and Performance
