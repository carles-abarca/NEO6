// FFI implementation for TN3270 protocol

use std::os::raw::c_char;
use std::ptr;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing::{debug, info, error};

use neo6_protocols_lib::ffi::{
    FfiResult, ProtocolHandle, ProtocolInterface,
    create_success_result, create_error_result, c_str_to_string
};
use crate::Tn3270Handler;
use neo6_protocols_lib::protocol::ProtocolHandler;

/// Internal structure to hold the handler and runtime
struct Tn3270FFIHandler {
    handler: Arc<Tn3270Handler>,
    runtime: Arc<Runtime>,
}

/// Create a new TN3270 protocol handler instance
extern "C" fn tn3270_create_handler() -> ProtocolHandle {
    match Runtime::new() {
        Ok(runtime) => {
            let handler = Arc::new(Tn3270Handler);
            let ffi_handler = Box::new(Tn3270FFIHandler {
                handler,
                runtime: Arc::new(runtime),
            });
            Box::into_raw(ffi_handler) as ProtocolHandle
        }
        Err(_) => ptr::null_mut(),
    }
}

/// Destroy a TN3270 protocol handler instance
extern "C" fn tn3270_destroy_handler(handle: ProtocolHandle) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle as *mut Tn3270FFIHandler);
        }
    }
}

/// Invoke a transaction on the TN3270 protocol handler
unsafe extern "C" fn tn3270_invoke_transaction(
    handle: ProtocolHandle,
    transaction_id: *const c_char,
    parameters: *const c_char,
) -> FfiResult {
    if handle.is_null() {
        return create_error_result("Invalid handle");
    }

    let ffi_handler = unsafe { &*(handle as *const Tn3270FFIHandler) };
    
    let tx_id = match unsafe { c_str_to_string(transaction_id) } {
        Some(s) => s,
        None => return create_error_result("Invalid transaction_id"),
    };

    let params_str = match unsafe { c_str_to_string(parameters) } {
        Some(s) => s,
        None => return create_error_result("Invalid parameters"),
    };

    let params = match serde_json::from_str(&params_str) {
        Ok(p) => p,
        Err(e) => return create_error_result(&format!("JSON parse error: {}", e)),
    };

    // Execute the async call in the runtime
    match ffi_handler.runtime.block_on(
        ffi_handler.handler.invoke_transaction(&tx_id, params)
    ) {
        Ok(result) => create_success_result(result),
        Err(e) => create_error_result(&e),
    }
}

/// Start a TN3270 listener on the specified port
unsafe extern "C" fn tn3270_start_listener(
    handle: ProtocolHandle,
    port: u16,
) -> FfiResult {
    if handle.is_null() {
        return create_error_result("Invalid handle");
    }

    let ffi_handler = unsafe { &*(handle as *const Tn3270FFIHandler) };
    
    // Start the actual TN3270 listener in a separate task
    let runtime = ffi_handler.runtime.clone();
    let handler = ffi_handler.handler.clone();
    
    std::thread::spawn(move || {
        runtime.block_on(async {
            use std::collections::HashMap;
            use neo6_protocols_lib::protocol::TransactionConfig;
            
            info!("Iniciando listener nativo real en puerto {}", port);
            
            // Create transaction map from configuration
            let mut tx_map = HashMap::<String, TransactionConfig>::new();
            tx_map.insert("TX_TN01".to_string(), TransactionConfig {
                protocol: "tn3270".to_string(),
                server: "mainframe-tn3270".to_string(),
                parameters: vec![],
                expected_response: None,
            });
            let tx_map = std::sync::Arc::new(tx_map);
            
            // Start the actual TN3270 listener
            let exec_fn = move |tx_id: String, params: serde_json::Value| {
                let handler_clone = handler.clone();
                async move {
                    debug!("Ejecutando transacción: {}", tx_id);
                    // Use the actual handler to process the transaction
                    handler_clone.invoke_transaction(&tx_id, params).await
                }
            };
            
            // This will run the listener indefinitely
            if let Err(e) = crate::start_tn3270_listener(port, tx_map, exec_fn).await {
                error!("Error en listener: {}", e);
            }
        });
    });
    
    // Give the listener a moment to start
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    create_success_result(serde_json::json!({
        "status": "listener_started", 
        "port": port,
        "address": format!("0.0.0.0:{}", port),
        "type": "native_tn3270_background"
    }))
}

/// Set logging level for TN3270 protocol
unsafe extern "C" fn tn3270_set_log_level(
    log_level: *const c_char,
) -> FfiResult {
    let level_str = match unsafe { c_str_to_string(log_level) } {
        Some(s) => s,
        None => return create_error_result("Invalid log_level"),
    };

    // Configure tracing subscriber for this protocol
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};
    
    // Create filter based on log level
    let filter = match level_str.to_lowercase().as_str() {
        "trace" => EnvFilter::new("trace"),
        "debug" => EnvFilter::new("debug"),
        "info" => EnvFilter::new("info"),
        "warn" => EnvFilter::new("warn"),
        "error" => EnvFilter::new("error"),
        _ => EnvFilter::new("info"), // default
    };
    
    // Try to set global subscriber (will fail if already set, but that's ok)
    let _ = tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_level(true))
        .with(filter)
        .try_init();
    
    create_success_result(serde_json::json!({
        "status": "log_level_set",
        "level": level_str
    }))
}

/// Get the protocol interface for TN3270 (only exported function)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_protocol_interface() -> *const ProtocolInterface {
    static INTERFACE: ProtocolInterface = ProtocolInterface {
        create_handler: tn3270_create_handler,
        destroy_handler: tn3270_destroy_handler,
        invoke_transaction: tn3270_invoke_transaction,
        start_listener: Some(tn3270_start_listener),
        set_log_level: Some(tn3270_set_log_level),
    };
    &INTERFACE
}
