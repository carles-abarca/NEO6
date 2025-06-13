use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{info, error, warn, debug};
use std::collections::HashSet;

use neo6_proxy::config::ProxyConfig;
use neo6_proxy::proxy::dynamic_router::create_dynamic_router;
use neo6_proxy::logging::{init_logging_with_level};
use neo6_proxy::cics::mapping::load_transaction_map;
use neo6_proxy::protocol_loader::ProtocolLoader;
use neo6_proxy::admin_control::{AdminControlServer, ProxyInfo, ControlMessage};

#[tokio::main]
async fn main() {
    // Mostrar ayuda si se invoca con --help
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        println!("neo6-proxy [--protocol=PROTO] [--port=PUERTO] [--log-level=LEVEL] [--config-dir=DIR]\n\n\t--protocol=PROTO\tProtocolo a escuchar (rest, lu62, mq, tcp, jca, tn3270)\n\t--port=PUERTO\tPuerto a escuchar (por defecto según protocolo)\n\t--log-level=LEVEL\tNivel de log (info, debug, trace, ...)\n\t--config-dir=DIR\tDirectorio base para configuraciones (por defecto: config)\n\t--help\t\tMuestra esta ayuda\n\nNota: neo6-proxy siempre usa carga dinámica de protocolos");
        return;
    }

    // Configuración base
    let mut config_dir = "config".to_string();
    let mut config = ProxyConfig::default();
    
    // Permitir override por CLI
    for arg in std::env::args().skip(1) {
        if arg.starts_with("--log-level=") {
            config.log_level = arg.replace("--log-level=", "");
        } else if arg.starts_with("--protocol=") {
            config.protocol = Some(arg.replace("--protocol=", ""));
        } else if arg.starts_with("--port=") {
            if let Ok(p) = arg.replace("--port=", "").parse::<u16>() {
                config.port = Some(p);
            }
        } else if arg.starts_with("--config-dir=") {
            config_dir = arg.replace("--config-dir=", "");
        }
    }
    
    // Cargar configuración desde el directorio especificado
    config.load_from_dir(&config_dir);
    
    // Inicializar logging con el nivel configurado
    init_logging_with_level(&config.log_level);
    info!(level = %config.log_level, "Inicializando neo6-proxy con carga dinámica de protocolos");

    // Determinar protocolo y puerto
    let protocol = config.protocol.clone().unwrap_or_else(|| "rest".to_string());
    let port = config.port.unwrap_or_else(|| match protocol.as_str() {
        "rest" => 8080,
        "lu62" => 4000,
        "mq" => 5000,
        "tcp" => 6000,
        "jca" => 7000,
        "tn3270" => 2323,
        _ => 8080,
    });

    // Puerto de control administrativo (puerto principal + 1000)
    let admin_port = port + 1000;

    // Mostrar protocolo y puerto
    info!("neo6-proxy escuchando protocolo: {} en puerto: {} (modo dinámico)", protocol, port);
    info!("Control administrativo disponible en puerto: {}", admin_port);

    // Cargar mapa de transacciones desde el directorio de configuración
    let transactions_path = format!("{}/transactions.yaml", config_dir);
    let tx_map = load_transaction_map(&transactions_path).expect("No se pudo cargar transactions.yaml");
    info!("Transacciones disponibles:");
    for (txid, tx) in tx_map.iter() {
        info!("  {}  [{}] -> {}", txid, tx.protocol, tx.server);
    }

    // Extraer todos los protocolos únicos de las transacciones
    let mut protocols_needed: HashSet<String> = tx_map.values()
        .map(|tx| tx.protocol.clone())
        .collect();
    
    // Agregar el protocolo de escucha si no está ya incluido
    protocols_needed.insert(protocol.clone());
    
    info!("Protocolos requeridos: {:?}", protocols_needed);

    // Inicializar el cargador de protocolos
    let protocol_loader = std::sync::Arc::new(ProtocolLoader::new());
    
    // Pre-cargar todos los protocolos necesarios
    for proto in &protocols_needed {
        match protocol_loader.load_protocol(proto) {
            Ok(_) => info!("Protocolo {} cargado exitosamente", proto),
            Err(e) => {
                error!("Error cargando protocolo {}: {}", proto, e);
                warn!("No se pudo cargar el protocolo {}: {}", proto, e);
            }
        }
    }

    // Configurar el nivel de log para todas las librerías dinámicas
    info!("Configurando nivel de log '{}' para todas las librerías dinámicas", config.log_level);
    protocol_loader.set_log_level_for_all(&config.log_level);

    // Crear información del proxy para el control administrativo
    let proxy_info = ProxyInfo {
        protocol: protocol.clone(),
        port,
        status: "running".to_string(),
        uptime: std::time::SystemTime::now(),
        protocols_loaded: protocols_needed.iter().cloned().collect(),
    };

    // Crear el servidor de control administrativo
    let (admin_server, mut control_rx) = AdminControlServer::new(admin_port, proxy_info);

    // Lanzar el servidor de control administrativo en segundo plano
    let admin_handle = tokio::spawn(async move {
        if let Err(e) = admin_server.start().await {
            error!("Error en servidor de control administrativo: {}", e);
        }
    });

    // Lanzar el listener del protocolo específico en segundo plano
    let protocol_loader_clone = protocol_loader.clone();
    let mut listener_handle = tokio::spawn(async move {
        match start_dynamic_listener(&protocol, port, &protocol_loader_clone).await {
            Ok(_) => {
                info!("Listener dinámico {} terminó exitosamente", protocol);
            }
            Err(e) => {
                error!("Error en listener dinámico {}: {}", protocol, e);
                error!("No se pudo iniciar el listener {}: {}", protocol, e);
            }
        }
    });

    // Bucle principal de control
    info!("neo6-proxy iniciado completamente, escuchando comandos de control...");
    loop {
        tokio::select! {
            // Manejar mensajes de control
            msg = control_rx.recv() => {
                match msg {
                    Some(ControlMessage::Shutdown) => {
                        info!("Recibido comando de shutdown, cerrando proxy...");
                        break;
                    }
                    Some(ControlMessage::ReloadConfig) => {
                        info!("Recibido comando de reload config (no implementado aún)");
                        // TODO: Implementar recarga de configuración
                    }
                    Some(ControlMessage::SetLogLevel(level)) => {
                        info!("Cambiando nivel de log a: {}", level);
                        // TODO: Implementar cambio dinámico de nivel de log
                        protocol_loader.set_log_level_for_all(&level);
                    }
                    None => {
                        warn!("Canal de control cerrado");
                        break;
                    }
                }
            }
            // Verificar si el listener terminó inesperadamente
            result = &mut listener_handle => {
                match result {
                    Ok(_) => info!("Listener terminó normalmente"),
                    Err(e) => error!("Error en listener: {}", e),
                }
                break;
            }
        }
    }

    // Limpiar recursos
    info!("Cerrando neo6-proxy...");
    admin_handle.abort();
    listener_handle.abort();
}

// Función para cargar dinámicamente el listener de cualquier protocolo
async fn start_dynamic_listener(protocol: &str, port: u16, protocol_loader: &ProtocolLoader) -> Result<(), String> {
    info!("Iniciando listener dinámico para protocolo: {}", protocol);
    
    // Intentar cargar la librería del protocolo
    let protocol_handler = protocol_loader.load_protocol(protocol)
        .map_err(|e| format!("No se pudo cargar protocolo {}: {}", protocol, e))?;
    
    // Verificar si el protocolo tiene un listener nativo
    match protocol_handler.start_listener(port) {
        Ok(result) => {
            info!("Listener nativo {} iniciado exitosamente: {:?}", protocol, result);
            info!("Listener {} nativo está corriendo en segundo plano...", protocol);
            
            // Mantener el programa corriendo indefinidamente
            // El listener se ejecuta en un hilo separado
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                debug!("Listener {} nativo sigue activo...", protocol);
            }
        }
        Err(e) => {
            warn!("Protocolo {} no tiene listener nativo ({}), usando interfaz REST", protocol, e);
            info!("Protocolo '{}' se ejecuta a través de interfaz REST dinámica", protocol);
            start_rest_fallback_listener(port).await
        }
    }
}

// Función para iniciar el listener REST/Axum como fallback
async fn start_rest_fallback_listener(port: u16) -> Result<(), String> {
    let app = create_dynamic_router();
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await
        .map_err(|e| format!("No se pudo bindear puerto {}: {}", port, e))?;
    
    info!("Starting dynamic REST interface on {}", addr);
    axum::serve(listener, app).await
        .map_err(|e| format!("Error en servidor REST: {}", e))?;
    
    Ok(())
}