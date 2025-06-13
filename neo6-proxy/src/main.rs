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
        println!("neo6-proxy [--protocol=PROTO] [--port=PUERTO] [--log-level=LEVEL] [--config-dir=DIR] [--library-path=PATH]\n\n\t--protocol=PROTO\tProtocolo a escuchar (rest, lu62, mq, tcp, jca, tn3270)\n\t--port=PUERTO\tPuerto a escuchar (por defecto según protocolo)\n\t--log-level=LEVEL\tNivel de log (info, debug, trace, ...)\n\t--config-dir=DIR\tDirectorio base para configuraciones (por defecto: config)\n\t--library-path=PATH\tRuta base para las librerías de protocolos (por defecto: ./lib)\n\t--help\t\tMuestra esta ayuda\n\nNota: neo6-proxy siempre usa carga dinámica de protocolos");
        return;
    }

    // Configuración base
    let mut config_dir = "config".to_string();
    let mut config = ProxyConfig::default();
    
    // Almacenar argumentos de CLI para aplicar después
    let mut cmd_protocol: Option<String> = None;
    let mut cmd_port: Option<u16> = None;
    let mut cmd_log_level: Option<String> = None;
    let mut cmd_library_path: Option<String> = None;
    
    // Permitir override por CLI
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        println!("Procesando argumento {}: {}", i, arg);
        
        if arg.starts_with("--log-level=") {
            cmd_log_level = Some(arg.replace("--log-level=", ""));
            println!("  -> cmd_log_level: {:?}", cmd_log_level);
        } else if arg == "--log-level" && i + 1 < args.len() {
            cmd_log_level = Some(args[i + 1].clone());
            println!("  -> cmd_log_level: {:?}", cmd_log_level);
            i += 1; // skip next argument
        } else if arg.starts_with("--protocol=") {
            cmd_protocol = Some(arg.replace("--protocol=", ""));
            println!("  -> cmd_protocol: {:?}", cmd_protocol);
        } else if arg == "--protocol" && i + 1 < args.len() {
            cmd_protocol = Some(args[i + 1].clone());
            println!("  -> cmd_protocol: {:?}", cmd_protocol);
            i += 1; // skip next argument
        } else if arg.starts_with("--port=") {
            if let Ok(p) = arg.replace("--port=", "").parse::<u16>() {
                cmd_port = Some(p);
                println!("  -> cmd_port: {:?}", cmd_port);
            }
        } else if arg == "--port" && i + 1 < args.len() {
            if let Ok(p) = args[i + 1].parse::<u16>() {
                cmd_port = Some(p);
                println!("  -> cmd_port: {:?}", cmd_port);
            }
            i += 1; // skip next argument
        } else if arg.starts_with("--config-dir=") {
            config_dir = arg.replace("--config-dir=", "");
            println!("  -> config_dir: {}", config_dir);
        } else if arg == "--config-dir" && i + 1 < args.len() {
            config_dir = args[i + 1].clone();
            println!("  -> config_dir: {}", config_dir);
            i += 1; // skip next argument
        } else if arg.starts_with("--library-path=") {
            cmd_library_path = Some(arg.replace("--library-path=", ""));
            println!("  -> cmd_library_path: {:?}", cmd_library_path);
        } else if arg == "--library-path" && i + 1 < args.len() {
            cmd_library_path = Some(args[i + 1].clone());
            println!("  -> cmd_library_path: {:?}", cmd_library_path);
            i += 1; // skip next argument
        }
        i += 1;
    }
    
    // Cargar configuración desde el directorio especificado
    config.load_from_dir(&config_dir);
    
    // Aplicar argumentos CLI (tienen precedencia sobre el archivo)
    if let Some(protocol) = cmd_protocol {
        config.protocol = Some(protocol);
    }
    if let Some(port) = cmd_port {
        config.port = Some(port);
    }
    if let Some(log_level) = cmd_log_level {
        config.log_level = log_level;
    }
    if let Some(library_path) = cmd_library_path {
        config.library_path = Some(library_path);
    }
    
    // Inicializar logging con el nivel configurado
    init_logging_with_level(&config.log_level);
    info!(level = %config.log_level, "Inicializando neo6-proxy con carga dinámica de protocolos");
    info!("Configuración después de procesamiento: protocol={:?}, port={:?}, log_level={}", 
          config.protocol, config.port, config.log_level);

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
    info!("Intentando cargar transactions.yaml desde: {}", transactions_path);
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

    // Establecer variable de entorno para el directorio de configuración
    // Esto permite que los protocolos accedan al config directory sin lógica especial en el proxy
    std::env::set_var("NEO6_CONFIG_DIR", &config_dir);
    info!("Variable de entorno NEO6_CONFIG_DIR establecida a: {}", config_dir);

    // Inicializar el cargador de protocolos con el path personalizado si se especificó
    let protocol_loader = if let Some(lib_path) = &config.library_path {
        info!("Usando ruta personalizada para librerías: {}", lib_path);
        std::sync::Arc::new(ProtocolLoader::with_library_path(Some(lib_path)))
    } else {
        info!("Usando rutas por defecto para librerías");
        std::sync::Arc::new(ProtocolLoader::new())
    };
    
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