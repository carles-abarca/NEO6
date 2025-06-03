
# neo6-proxy

`neo6-proxy` es un proxy de transacciones CICS de alto rendimiento, escrito en Rust y diseñado para ejecutarse en contenedores portables entre entornos on-prem y en la nube. Forma parte del ecosistema ByteMorph.ai, permitiendo la interoperabilidad entre aplicaciones modernas y sistemas COBOL legacy (ya sea en IBM z/OS o Micro Focus Enterprise Server).

---

## ✨ Funcionalidad principal

- Invocación de transacciones COBOL desde aplicaciones modernas (REST/gRPC).
- Adaptación de protocolos legacy (LU6.2 sobre TCP).
- Integración directa con colas IBM MQ mediante bindings en C.
- Traducción dinámica de estructuras JSON ↔ COBOL COMMAREA.
- Exposición de métricas y trazabilidad para monitoreo avanzado.

---

## 📡 Interfaces y Endpoints

### 1. REST API (HTTP JSON)

| Método | Endpoint        | Descripción |
|--------|------------------|-------------|
| POST   | `/invoke`        | Invoca una transacción COBOL sincrónicamente. |
| POST   | `/invoke-async`  | Ejecuta invocación asincrónica. |
| GET    | `/status/{{id}}` | Consulta estado/resultados de la invocación. |
| GET    | `/health`        | Health check básico. |
| GET    | `/metrics`       | Exposición de métricas Prometheus. |

#### Ejemplo de payload para `/invoke`

```json
{
  "transaction_id": "TX01",
  "parameters": {
    "account_number": "1234567890",
    "amount": 500.25
  }
}
```

---

### 2. TCP plano estilo LU6.2

- **Puerto:** `4000`
- **Protocolo:** TCP binario o texto estructurado.
- **Formato esperado:** Identificador de transacción + parámetros COMMAREA serializados.
- **Uso:** Integración con middlewares legacy que aún operan sobre LU6.2 encapsulado.

---

### 3. IBM MQ (bindings C con `cmqc.h`)

- **Conexión:** vía IBM MQ Client (requiere instalación del runtime).
- **Colas:**
  - `REQUEST.Q` → cola de entrada
  - `RESPONSE.Q` → cola de salida
- **Modo:** Listener asíncrono con procesamiento de transacciones en background.
- **Dependencias:** bindings generados con `bindgen` a partir de las cabeceras MQ de IBM (`cmqc.h`).

---

## 🧱 Estructura del proyecto

```
neo6-proxy/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── logging.rs
│   ├── metrics.rs
│   ├── proxy/          # REST, TCP
│   ├── mq/             # IBM MQ bindings y workers
│   ├── cics/           # Llamadas COBOL y mapeos
├── config/
│   └── default.toml
├── .env
├── Dockerfile
└── tests/
```

---

## 📚 Elecciones tecnológicas clave

- **Lenguaje principal:** Rust
- **Async runtime:** Tokio (`tokio`)
- **Frameworks web:** Axum o Warp
- **Metrics y observabilidad:** Prometheus + OpenTelemetry
- **Bindings C para IBM MQ:** `bindgen` sobre `cmqc.h`
- **Compilación cruzada:** `cross` (opcional para multiplataforma)
- **Testing:** `tokio::test`, `assert_json_diff`

---

## 📦 Dependencias Rust previstas (`Cargo.toml`)

```toml
[dependencies]
tokio = { version = "1.38", features = ["full"] }
axum = "0.7"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
prometheus = "0.13"
tracing = "0.1"
tracing-subscriber = "0.3"
dotenvy = "0.15"
config = "0.13"
reqwest = { version = "0.12", features = ["json", "tls"] }
bindgen = "0.69"
libc = "0.2"
```

---

## 🔐 Seguridad

- Compatible con autenticación JWT y TLS para HTTP/gRPC.
- Uso recomendado de canales seguros (VPN o MTLS) para conexiones TCP y MQ.
- Validación de transacción y payload antes de envío al backend COBOL.

---

## ⚙️ Configuración (config/default.toml)

```toml
[mq]
queue_manager = "QM1"
channel       = "DEV.APP.SVRCONN"
conn_name     = "mq-host(1414)"
request_queue = "REQUEST.Q"
response_queue = "RESPONSE.Q"
user          = "app"
password      = "password"
```

---

## 🧠 Roadmap futuro

- Soporte gRPC Protobuf.
- Orquestación automática de mapeos vía agentes AI.
- CLI de administración y simulación de carga.
- Failover entre entornos on-prem y cloud.

---

## 🚀 Licencia

Propiedad de ByteMorph.ai. Uso interno o bajo acuerdo de licencia empresarial.
