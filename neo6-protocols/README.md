# NEO6 Protocols Proxy

Este subproyecto forma parte del ecosistema **NEO6** y proporciona los traits y helpers base para la implementación de un **Proxy de Protocolos CICS**. Permite recibir e interpretar peticiones de múltiples aplicaciones cliente —modernas y legacy— que consumen transacciones del entorno **CICS** (Customer Information Control System).

## 🎯 Objetivo

El objetivo de este módulo es proporcionar una capa de interoperabilidad que permita reemplazar, emular o interceptar las llamadas tradicionales a CICS mediante un conjunto de interfaces modernas y compatibles con los estándares más usados en la banca. El código fuente define el trait `ProtocolHandler` y utilidades de logging para la trazabilidad de invocaciones de protocolo.

---

## 🌐 Protocolos Soportados (interfaces previstas)

### 1. **LU6.2 / APPC (Advanced Program-to-Program Communication)**
- **Tipo**: Comunicación sincrónica sobre SNA/IP.
- **Uso**: Integraciones legacy con CICS a través de IBM CICS Transaction Gateway o conexiones directas.
- **Justificación**: Muchos sistemas bancarios distribuidos aún dependen de este protocolo para ejecutar transacciones COBOL en z/OS o entornos Micro Focus.
- **Soporte**: El trait `ProtocolHandler` permite implementar un handler para LU6.2.

### 2. **IBM MQ (WebSphere MQ)**
- **Tipo**: Comunicación asíncrona basada en colas de mensajes.
- **Uso**: Intercambio de mensajes entre aplicaciones distribuidas y transacciones CICS mediante colas (Request/Reply o Fire-and-Forget).
- **Justificación**: Estándar industrial ampliamente adoptado por bancos y entidades financieras.
- **Soporte**: Implementable mediante el trait `ProtocolHandler`.

### 3. **HTTP(S) + JSON/XML (REST/SOAP)**
- **Tipo**: Comunicación sincrónica sobre protocolos web.
- **Uso**: Servicios web expuestos directamente desde CICS o wrappers modernos.
- **Justificación**: Aumenta la compatibilidad con arquitecturas modernas (microservicios, APIs REST).
- **Soporte**: Implementable mediante el trait `ProtocolHandler`.

### 4. **TCP/IP Proprietary Protocols**
- **Tipo**: Comunicación sincrónica o asíncrona personalizada.
- **Uso**: Protocolos binarios propietarios usados en aplicaciones legacy de alto rendimiento.
- **Justificación**: Presente en soluciones antiguas que no usan estándares IBM por temas de rendimiento o licencia.
- **Soporte**: Implementable mediante el trait `ProtocolHandler`.

### 5. **JCA (Java Connector Architecture) / CICS Transaction Gateway**
- **Tipo**: Conectividad Java EE mediante CICS TG.
- **Uso**: Aplicaciones empresariales Java que invocan transacciones CICS mediante `com.ibm.connector2.cics.ECIConnectionFactory`.
- **Justificación**: Presente en middleware Java que usa IBM WebSphere o entornos similares.
- **Soporte**: Implementable mediante el trait `ProtocolHandler`.

---

## 🧩 Estructura del código principal

- `protocol.rs`: Define el trait `ProtocolHandler` y el helper `log_protocol_invoke` para trazabilidad de invocaciones.
- Cada protocolo concreto (LU6.2, MQ, REST, TCP, JCA) debe implementar el trait `ProtocolHandler` en su propio crate o módulo.

---

## 📝 Ejemplo de uso

```rust
use neo6_protocols_lib::protocol::{ProtocolHandler, log_protocol_invoke};

struct MyProtocol;

impl ProtocolHandler for MyProtocol {
    fn invoke_transaction(&self, transaction_id: &str, parameters: serde_json::Value) -> Result<serde_json::Value, String> {
        log_protocol_invoke("myprotocol", transaction_id, &parameters);
        // Lógica de invocación...
        Ok(serde_json::json!({"result": "ok"}))
    }
}
```

---

## 📦 Dependencias principales

- `serde_json` para la serialización de parámetros y resultados.
- `tracing` para logging estructurado y trazabilidad.

---

## 🚦 Estado

Este crate define la interfaz y utilidades base. Las implementaciones concretas de cada protocolo se encuentran en subcrates como `lu62`, `mq`, `rest`, `tcp`, `jca`.

---

## 🚀 Licencia

Propiedad de ByteMorph.ai. Uso interno o bajo acuerdo de licencia empresarial.