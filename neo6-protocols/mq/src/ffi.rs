// FFI implementation for MQ protocol

use std::os::raw::c_char;
use std::ptr;
use std::sync::Arc;
use tokio::runtime::Runtime;

use neo6_protocols_lib::ffi::{
    FfiResult, ProtocolHandle, ProtocolInterface,
    create_success_result, create_error_result, c_str_to_string
};
use crate::MqHandler;
use neo6_protocols_lib::protocol::ProtocolHandler;

/// Internal structure to hold the handler and runtime
struct MqFFIHandler {
    handler: Arc<MqHandler>,
    runtime: Arc<Runtime>,
}

/// Create a new MQ protocol handler instance
extern "C" fn mq_create_handler() -> ProtocolHandle {
    match Runtime::new() {
        Ok(runtime) => {
            let handler = Arc::new(MqHandler);
            let ffi_handler = Box::new(MqFFIHandler {
                handler,
                runtime: Arc::new(runtime),
            });
            Box::into_raw(ffi_handler) as ProtocolHandle
        }
        Err(_) => ptr::null_mut(),
    }
}

/// Destroy a MQ protocol handler instance
extern "C" fn mq_destroy_handler(handle: ProtocolHandle) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle as *mut MqFFIHandler);
        }
    }
}

/// Invoke a transaction on the MQ protocol handler
unsafe extern "C" fn mq_invoke_transaction(
    handle: ProtocolHandle,
    transaction_id: *const c_char,
    parameters: *const c_char,
) -> FfiResult {
    if handle.is_null() {
        return create_error_result("Invalid handle");
    }

    let ffi_handler = unsafe { &*(handle as *const MqFFIHandler) };
    
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

/// Get the protocol interface for MQ (only exported function)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_protocol_interface() -> *const ProtocolInterface {
    static INTERFACE: ProtocolInterface = ProtocolInterface {
        create_handler: mq_create_handler,
        destroy_handler: mq_destroy_handler,
        invoke_transaction: mq_invoke_transaction,
        start_listener: None,
    };
    &INTERFACE
}
