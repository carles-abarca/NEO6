// FFI implementation for JCA protocol

use std::os::raw::c_char;
use std::ptr;
use std::sync::Arc;
use tokio::runtime::Runtime;

use neo6_protocols_lib::ffi::{
    FfiResult, ProtocolHandle, ProtocolInterface,
    create_success_result, create_error_result, c_str_to_string
};
use crate::JcaHandler;
use neo6_protocols_lib::protocol::ProtocolHandler;

/// Internal structure to hold the handler and runtime
struct JcaFFIHandler {
    handler: Arc<JcaHandler>,
    runtime: Arc<Runtime>,
}

/// Create a new JCA protocol handler instance
extern "C" fn jca_create_handler() -> ProtocolHandle {
    match Runtime::new() {
        Ok(runtime) => {
            let handler = Arc::new(JcaHandler);
            let ffi_handler = Box::new(JcaFFIHandler {
                handler,
                runtime: Arc::new(runtime),
            });
            Box::into_raw(ffi_handler) as ProtocolHandle
        }
        Err(_) => ptr::null_mut(),
    }
}

/// Destroy a JCA protocol handler instance
extern "C" fn jca_destroy_handler(handle: ProtocolHandle) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle as *mut JcaFFIHandler);
        }
    }
}

/// Invoke a transaction on the JCA protocol handler
unsafe extern "C" fn jca_invoke_transaction(
    handle: ProtocolHandle,
    transaction_id: *const c_char,
    parameters: *const c_char,
) -> FfiResult {
    if handle.is_null() {
        return create_error_result("Invalid handle");
    }

    let ffi_handler = unsafe { &*(handle as *const JcaFFIHandler) };
    
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

/// Set logging level for JCA protocol
unsafe extern "C" fn jca_set_log_level(log_level: *const c_char) -> FfiResult {
    let level_str = match unsafe { c_str_to_string(log_level) } {
        Some(s) => s,
        None => return create_error_result("Invalid log_level"),
    };
    
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};
    let filter = match level_str.to_lowercase().as_str() {
        "trace" => EnvFilter::new("trace"),
        "debug" => EnvFilter::new("debug"), 
        "info" => EnvFilter::new("info"),
        "warn" => EnvFilter::new("warn"),
        "error" => EnvFilter::new("error"),
        _ => EnvFilter::new("info"),
    };
    let _ = tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_level(true))
        .with(filter).try_init();
    create_success_result(serde_json::json!({"status": "log_level_set", "level": level_str}))
}

/// Get the protocol interface for JCA (only exported function)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_protocol_interface() -> *const ProtocolInterface {
    static INTERFACE: ProtocolInterface = ProtocolInterface {
        create_handler: jca_create_handler,
        destroy_handler: jca_destroy_handler,
        invoke_transaction: jca_invoke_transaction,
        start_listener: None,
        set_log_level: Some(jca_set_log_level),
    };
    &INTERFACE
}
