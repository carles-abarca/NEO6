// FFI implementation for LU62 protocol

use std::os::raw::c_char;
use std::ptr;
use std::sync::Arc;
use tokio::runtime::Runtime;

use neo6_protocols_lib::ffi::{
    FfiResult, ProtocolHandle, ProtocolInterface,
    create_success_result, create_error_result, c_str_to_string
};
use crate::Lu62Handler;
use neo6_protocols_lib::protocol::ProtocolHandler;

/// Internal structure to hold the handler and runtime
struct Lu62FFIHandler {
    handler: Arc<Lu62Handler>,
    runtime: Arc<Runtime>,
}

/// Create a new LU62 protocol handler instance
extern "C" fn lu62_create_handler() -> ProtocolHandle {
    match Runtime::new() {
        Ok(runtime) => {
            let handler = Arc::new(Lu62Handler);
            let ffi_handler = Box::new(Lu62FFIHandler {
                handler,
                runtime: Arc::new(runtime),
            });
            Box::into_raw(ffi_handler) as ProtocolHandle
        }
        Err(_) => ptr::null_mut(),
    }
}

/// Destroy a LU62 protocol handler instance
extern "C" fn lu62_destroy_handler(handle: ProtocolHandle) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle as *mut Lu62FFIHandler);
        }
    }
}

/// Invoke a transaction on the LU62 protocol handler
unsafe extern "C" fn lu62_invoke_transaction(
    handle: ProtocolHandle,
    transaction_id: *const c_char,
    parameters: *const c_char,
) -> FfiResult {
    if handle.is_null() {
        return create_error_result("Invalid handle");
    }

    let ffi_handler = unsafe { &*(handle as *const Lu62FFIHandler) };
    
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
        Err(e) => create_error_result(e.as_str()),
    }
}

/// Set logging level for LU62 protocol
unsafe extern "C" fn lu62_set_log_level(
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

/// Get the protocol interface for LU62 (only exported function)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_protocol_interface() -> *const ProtocolInterface {
    static INTERFACE: ProtocolInterface = ProtocolInterface {
        create_handler: lu62_create_handler,
        destroy_handler: lu62_destroy_handler,
        invoke_transaction: lu62_invoke_transaction,
        start_listener: None,
        set_log_level: Some(lu62_set_log_level),
    };
    &INTERFACE
}
