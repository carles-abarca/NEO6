# NEO6 Agent Runtime

The **agent-runtime** is a comprehensive Python library that provides all the necessary components for implementing intelligent agents within the NEO6 ecosystem. It serves as the foundational runtime for all NEO6 agents, offering standardized interfaces and utilities for agent development and deployment.

## 🎯 Core Components

### MCP (Multi-Channel Protocol) Server
- **Purpose**: Central communication hub for agent coordination
- **Features**: Multi-threaded request handling, protocol abstraction, connection management
- **Integration**: Seamless integration with NEO6 proxy and administrative components

### MCP Client
- **Purpose**: Lightweight client interface for agent communication
- **Features**: Async/await support, automatic reconnection, message queuing
- **Use Cases**: Agent-to-agent communication, proxy integration, external system connectivity

### Auxiliary Libraries
- **Configuration Management**: YAML/JSON config parsing and validation
- **Logging Framework**: Structured logging with multiple output formats
- **Metrics Collection**: Performance monitoring and system health metrics
- **Error Handling**: Standardized exception handling and recovery mechanisms
- **Plugin System**: Dynamic agent loading and lifecycle management

## 🏗️ Architecture

The runtime follows a modular architecture that supports:

- **Agent Lifecycle Management**: Start, stop, pause, resume operations
- **Inter-Agent Communication**: Message passing and event handling
- **Resource Management**: Memory, CPU, and I/O resource allocation
- **Configuration Hot-Reload**: Dynamic configuration updates without restart
- **Health Monitoring**: Agent health checks and automatic recovery

## 🚀 Key Features

- **Python-Native**: Fully implemented in Python for maximum flexibility and ease of development
- **Async Support**: Built on asyncio for high-performance concurrent operations  
- **Extensible**: Plugin-based architecture allows for custom agent implementations
- **Observable**: Comprehensive logging, metrics, and tracing capabilities
- **Resilient**: Automatic error recovery and connection management
- **Scalable**: Supports horizontal scaling and load distribution

## 📦 Agent Types Supported

The runtime is designed to support various agent types within the NEO6 ecosystem:

1. **AIOps Agents**: Operational intelligence and automation
2. **Code Generation Agents**: Automated code creation and modification
3. **COBOL Migration Agents**: Legacy system modernization
4. **Monitoring Agents**: System health and performance tracking
5. **Integration Agents**: External system connectivity

## 🛠️ Usage Example

```python
from agent_runtime import AgentRuntime, MCPServer, MCPClient
import asyncio

class MyAgent:
    def __init__(self, runtime):
        self.runtime = runtime
        self.server = MCPServer()
        self.client = MCPClient()
    
    async def start(self):
        await self.server.start()
        await self.client.connect()
        
    async def handle_request(self, request):
        # Process agent request
        result = await self.process(request)
        return result

# Initialize and start agent
async def main():
    runtime = AgentRuntime()
    agent = MyAgent(runtime)
    await agent.start()

if __name__ == "__main__":
    asyncio.run(main())
```

## 🔧 Configuration

The runtime uses YAML configuration files for agent setup:

```yaml
agent:
  name: "my-agent"
  type: "custom"
  version: "1.0.0"
  
mcp_server:
  host: "0.0.0.0"
  port: 8080
  max_connections: 100
  
mcp_client:
  reconnect_interval: 5
  max_retries: 3
  timeout: 30

logging:
  level: "INFO"
  format: "structured"
  output: ["console", "file"]
  
metrics:
  enabled: true
  collection_interval: 60
  export_endpoint: "http://prometheus:9090"
```

## 📋 Requirements

- Python 3.8+
- asyncio support
- Additional dependencies defined in `requirements.txt`

## 🔗 Integration

The agent-runtime integrates seamlessly with other NEO6 components:

- **NEO6 Proxy**: Direct communication through MCP protocol
- **NEO6 Admin**: Management and monitoring interface
- **Protocol Handlers**: Access to legacy and modern protocol implementations
- **Infrastructure**: Cloud deployment and scaling capabilities

---

The **agent-runtime** provides the foundation for building intelligent, scalable, and maintainable agents within the NEO6 ecosystem, enabling advanced automation and integration capabilities across legacy and modern systems.