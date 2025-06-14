# NEO6 Admin

Sistema de administración centralizado para el ecosistema NEO6. Proporciona gestión, monitoreo y control de múltiples instancias de proxy NEO6 a través de un dashboard web y API REST.

## Características Principales

- **Dashboard Web Interactivo**: Interfaz moderna para monitoreo visual y gestión de proxies
- **API REST Completa**: Control programático de todas las funciones administrativas
- **Gestión Centralizada**: Administración de múltiples proxies desde un solo punto
- **Monitoreo en Tiempo Real**: Métricas, conexiones activas y estado de servicios
- **Configuración Flexible**: Sistema de proxy_defaults para eliminación de redundancia
- **Despliegue Automatizado**: Scripts para construcción y despliegue completo del runtime

## Arquitectura

```
neo6-admin
├── Dashboard Web (Puerto 8090)
├── API REST (/api/*)
├── Proxy Manager (Gestión del ciclo de vida)
├── Configuration Manager (Gestión de configuraciones)
└── Admin Control Interface (Comunicación con proxies)
```

## Instalación y Construcción

### Requisitos
- Rust 1.70+
- Cargo

### Construcción
```bash
# Construcción en modo debug
cargo build

# Construcción optimizada
cargo build --release
```

## Configuración

### Archivo Principal: `config/admin.yaml`

```yaml
admin_settings:
  port: 8090
  log_level: "info"

proxy_defaults:
  binary_path: "./bin/neo6-proxy"
  working_directory: "."
  library_path: "./lib"
  config_path: "./config/proxy"
  log_level: "info"

proxy_instances:
  - name: "rest-api"
    protocol: "rest"
    port: 8080
    admin_port: 9080
    
  - name: "tn3270-primary"
    protocol: "tn3270"
    port: 2323
    admin_port: 3323
    
  - name: "mq-gateway"
    protocol: "mq"
    port: 5001
    admin_port: 6001
    log_level: "debug"  # Override del default
```

### Proxy Defaults

El sistema de `proxy_defaults` elimina la redundancia en la configuración:
- Campos comunes definidos una vez
- Instancias pueden override campos específicos
- Generación automática de argumentos CLI
- Configuración más limpia y mantenible

## Despliegue Automatizado

### Script de Despliegue

```bash
# Ejecutar desde el directorio neo6-admin
./scripts/neo6deploy.sh

# Opciones disponibles
./scripts/neo6deploy.sh --release    # Build optimizado
./scripts/neo6deploy.sh --debug      # Build con debug (default)
```

### Proceso Automatizado
1. Construcción de todos los componentes Rust
2. Compilación de bibliotecas de protocolos
3. Creación de estructura de runtime
4. Copia de binarios y bibliotecas
5. Generación de configuraciones
6. Creación de scripts de control
7. Validación automática del despliegue

### Estructura del Runtime Generado

```
runtime/
├── neo6.sh                 # Script principal de control
├── bin/                    # Binarios ejecutables
│   ├── neo6-admin
│   └── neo6-proxy
├── lib/                    # Bibliotecas de protocolos
│   ├── libtn3270.dylib
│   ├── liblu62.dylib
│   └── ...
├── config/                 # Configuraciones
│   ├── admin/admin.yaml
│   └── proxy/
├── logs/                   # Archivos de log
└── static/                 # Assets del dashboard
```

## Uso

### Inicio del Sistema

```bash
# Desde el directorio runtime
./neo6.sh start

# Verificar estado
./neo6.sh status

# Ver logs
./neo6.sh logs
```

### Dashboard Web

Accede al dashboard en: `http://localhost:8090`

Funcionalidades:
- Vista general de todos los proxies
- Detalle individual de cada proxy
- Monitoreo de conexiones activas
- Métricas en tiempo real
- Control de proxies (start/stop/restart)

### API REST

Base URL: `http://localhost:8090/api`

#### Endpoints Principales

```bash
# Listar todos los proxies
GET /api/proxies

# Detalle de un proxy específico
GET /api/proxies/{name}

# Control de proxies
POST /api/proxies/{name}/start
POST /api/proxies/{name}/stop
POST /api/proxies/{name}/restart

# Estado con métricas en tiempo real
GET /api/proxies/{name}/status

# Recargar configuración
POST /api/proxies/{name}/reload

# Enviar comando administrativo
POST /api/proxies/{name}/command
```

#### Ejemplos de Uso

```bash
# Iniciar un proxy
curl -X POST http://localhost:8090/api/proxies/tn3270-primary/start

# Obtener estado con métricas
curl http://localhost:8090/api/proxies/tn3270-primary/status

# Cambiar nivel de log
curl -X POST http://localhost:8090/api/proxies/tn3270-primary/command \
  -H "Content-Type: application/json" \
  -d '{"command": "SetLogLevel", "level": "debug"}'

# Obtener métricas detalladas
curl -X POST http://localhost:8090/api/proxies/tn3270-primary/command \
  -H "Content-Type: application/json" \
  -d '{"command": "GetMetrics"}'
```

## Comandos Administrativos

Los proxies soportan los siguientes comandos via interface admin:

### Comandos Básicos
```json
{"command": "Status"}                          // Estado del proxy
{"command": "GetMetrics"}                      // Métricas detalladas
{"command": "GetConnections"}                  // Conexiones activas
{"command": "GetProtocols"}                    // Protocolos cargados
```

### Comandos de Control
```json
{"command": "Shutdown"}                        // Apagar proxy
{"command": "ReloadConfig"}                    // Recargar configuración
{"command": "SetLogLevel", "level": "debug"}   // Cambiar nivel de log
```

### Comandos Avanzados
```json
{"command": "TestProtocol", "protocol": "tn3270"}           // Test de conectividad
{"command": "KillConnection", "connection_id": "conn_123"}  // Terminar conexión
{"command": "GetLogs", "lines": 100}                       // Obtener logs
{"command": "GetProtocolStatus", "protocol": "tn3270"}     // Estado de protocolo
```

## Monitoreo y Métricas

### Tipos de Métricas

#### Métricas de Conexión
```json
{
  "connection_id": "conn_12345",
  "protocol": "tn3270",
  "remote_address": "192.168.1.100:45678",
  "connected_at_timestamp": 1718366400,
  "bytes_sent": 2048,
  "bytes_received": 1024,
  "requests_processed": 15,
  "last_activity_timestamp": 1718366460
}
```

#### Métricas de Protocolo
```json
{
  "protocol_name": "tn3270",
  "total_connections": 1250,
  "active_connections": 23,
  "failed_connections": 15,
  "total_bytes_sent": 1048576,
  "total_bytes_received": 524288,
  "total_requests": 5000,
  "avg_response_time_ms": 125.5,
  "uptime_seconds": 86400
}
```

### Logging Estructurado

Niveles de log disponibles:
- **ERROR**: Errores críticos
- **WARN**: Situaciones problemáticas
- **INFO**: Información general
- **DEBUG**: Información detallada
- **TRACE**: Información muy detallada

## Desarrollo

### Estructura del Código

```
src/
├── main.rs              # Punto de entrada
├── config.rs            # Gestión de configuración
├── proxy_manager.rs     # Gestión del ciclo de vida de proxies
└── web_server.rs        # Servidor web y API REST
```

### Dependencias Principales

- **axum**: Framework web para API REST
- **tokio**: Runtime asíncrono
- **serde**: Serialización/deserialización
- **tracing**: Logging estructurado
- **clap**: Parsing de argumentos CLI

### Construcción para Desarrollo

```bash
# Construcción en modo debug con logs detallados
cargo build

# Ejecutar directamente
cargo run

# Ejecutar con configuración específica
cargo run -- --config-file config/admin.yaml
```

## Troubleshooting

### Problemas Comunes

#### Puerto Ocupado
```bash
# Verificar puertos en uso
netstat -tulpn | grep 8090

# Cambiar puerto en configuración
admin_settings:
  port: 8091
```

#### Proxy No Responde
```bash
# Verificar logs
tail -f runtime/logs/neo6-admin.log

# Verificar estado del proceso
ps aux | grep neo6

# Reiniciar proxy específico
curl -X POST http://localhost:8090/api/proxies/proxy-name/restart
```

#### Configuración Inválida
```bash
# Validar sintaxis YAML
python -c "import yaml; yaml.safe_load(open('config/admin.yaml'))"

# Verificar logs de configuración
grep -i "config" runtime/logs/neo6-admin.log
```

### Logs de Diagnóstico

```bash
# Logs del admin
tail -f runtime/logs/neo6-admin.log

# Logs de proxy específico
tail -f runtime/logs/tn3270-primary.log

# Buscar errores
grep -i error runtime/logs/*.log

# Logs con nivel debug
curl -X POST http://localhost:8090/api/proxies/proxy-name/command \
  -d '{"command": "SetLogLevel", "level": "debug"}'
```

## Contribución

### Desarrollo Local

```bash
# Clonar repositorio
git clone <repo-url>
cd neo6-admin

# Instalar dependencias
cargo build

# Ejecutar tests
cargo test

# Ejecutar con logs detallados
RUST_LOG=debug cargo run
```

### Tests

```bash
# Ejecutar todos los tests
cargo test

# Tests con output detallado
cargo test -- --nocapture

# Tests específicos
cargo test config
```

## Licencia

[Especificar licencia del proyecto]

## Contacto y Soporte

[Información de contacto para soporte]
