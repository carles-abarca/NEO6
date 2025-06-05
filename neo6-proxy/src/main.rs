use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{info};

use neo6_proxy::config::ProxyConfig;
use neo6_proxy::proxy::router::create_router;
use neo6_proxy::logging::{init_logging_with_level};
use neo6_proxy::cics::mapping::load_transaction_map;

#[tokio::main]
async fn main() {
    // Mostrar ayuda si se invoca con --help
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        println!("neo6-proxy [--protocol=PROTO] [--port=PUERTO] [--log-level=LEVEL]\n\n\t--protocol=PROTO\tProtocolo a escuchar (rest, lu62, mq, tcp, jca)\n\t--port=PUERTO\tPuerto a escuchar (por defecto según protocolo)\n\t--log-level=LEVEL\tNivel de log (info, debug, trace, ...)\n\t--help\t\tMuestra esta ayuda");
        return;
    }
    // Cargar configuración
    let mut config = ProxyConfig::from_default();
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
        }
    }
    // Inicializar logging con el nivel configurado
    init_logging_with_level(&config.log_level);
    info!(level = %config.log_level, "Inicializando neo6-proxy con nivel de log");

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

    // Mostrar protocolo y puerto
    println!("neo6-proxy escuchando protocolo: {} en puerto: {}", protocol, port);

    // Mostrar transacciones disponibles
    let tx_map = load_transaction_map("config/transactions.yaml").expect("No se pudo cargar transactions.yaml");
    println!("Transacciones disponibles:");
    for (txid, tx) in tx_map.iter() {
        println!("  {}  [{}] -> {}", txid, tx.protocol, tx.server);
    }

    // Lanzar el listener adecuado según protocolo
    match protocol.as_str() {
        "rest" => {
            // Build the Axum app with all routes
            let app = create_router();
            let addr = SocketAddr::from(([0, 0, 0, 0], port));
            let listener = TcpListener::bind(addr).await.unwrap();
            tracing::info!("Starting neo6-proxy REST on {}", addr);
            axum::serve(listener, app).await.unwrap();
        },
        // Aquí irían los listeners de otros protocolos (lu62, mq, tcp, jca)
        "tn3270" => {
            // Llama al listener TN3270 con API genérica
            println!("Iniciando listener TN3270 en puerto {}", port);
            let tx_map_arc = std::sync::Arc::new(tx_map.clone());
            let tx_map_for_exec = tx_map_arc.clone();
            let exec_fn = move |txid: String, params: serde_json::Value| {
                let tx_map = tx_map_for_exec.clone();
                async move {
                    if let Some(tx) = tx_map.get(&txid) {
                        if let Some(handler) = neo6_proxy::proxy::handler::get_protocol_handler(tx) {
                            handler.invoke_transaction(&txid, params).await
                        } else {
                            Err("No handler for protocol".to_string())
                        }
                    } else {
                        Err("Transaction not found".to_string())
                    }
                }
            };
            tn3270::start_tn3270_listener(port, tx_map_arc, exec_fn).await.expect("Fallo en listener TN3270");
        },
        _ => {
            eprintln!("Protocolo '{}' no implementado aún como listener.", protocol);
        }
    }
}