// FFI (Foreign Function Interface) definitions for dynamic protocol loading

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use serde_json::Value;

/// Result type for FFI operations
#[repr(C)]
pub struct FfiResult {
    /// Success flag (0 = success, non-zero = error)
    pub success: i32,
    /// Result data as JSON string (null-terminated C string)
    pub data: *mut c_char,
    /// Error message if success is non-zero (null-terminated C string)
    pub error: *mut c_char,
}

/// Opaque pointer to protocol handler instance
pub type ProtocolHandle = *mut c_void;

/// Function pointer type for creating protocol handlers
pub type CreateProtocolHandlerFn = unsafe extern "C" fn() -> ProtocolHandle;

/// Function pointer type for destroying protocol handlers
pub type DestroyProtocolHandlerFn = unsafe extern "C" fn(handle: ProtocolHandle);

/// Function pointer type for invoking transactions
pub type InvokeTransactionFn = unsafe extern "C" fn(
    handle: ProtocolHandle,
    transaction_id: *const c_char,
    parameters: *const c_char,
) -> FfiResult;

/// Function pointer type for starting a protocol listener
pub type StartListenerFn = unsafe extern "C" fn(
    handle: ProtocolHandle,
    port: u16,
) -> FfiResult;

/// Function pointer type for configuring logging level
pub type SetLogLevelFn = unsafe extern "C" fn(
    log_level: *const c_char,
) -> FfiResult;

/// Protocol interface structure containing function pointers
#[repr(C)]
pub struct ProtocolInterface {
    pub create_handler: CreateProtocolHandlerFn,
    pub destroy_handler: DestroyProtocolHandlerFn,
    pub invoke_transaction: InvokeTransactionFn,
    pub start_listener: Option<StartListenerFn>,
    pub set_log_level: Option<SetLogLevelFn>,
}

/// Function pointer type for getting protocol interface from dynamic library
pub type GetProtocolInterfaceFn = unsafe extern "C" fn() -> *const ProtocolInterface;

/// Helper function to create a C string from Rust string
pub fn create_c_string(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Helper function to convert C string to Rust string
pub unsafe fn c_str_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    
    match CStr::from_ptr(ptr).to_str() {
        Ok(s) => Some(s.to_string()),
        Err(_) => None,
    }
}

/// Helper function to free C string created by create_c_string
pub unsafe fn free_c_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = CString::from_raw(ptr);
    }
}

/// Helper function to create success result
pub fn create_success_result(data: Value) -> FfiResult {
    let data_str = serde_json::to_string(&data).unwrap_or_default();
    FfiResult {
        success: 0,
        data: create_c_string(&data_str),
        error: std::ptr::null_mut(),
    }
}

/// Helper function to create error result
pub fn create_error_result(error: &str) -> FfiResult {
    FfiResult {
        success: 1,
        data: std::ptr::null_mut(),
        error: create_c_string(error),
    }
}

/// Helper function to free FFI result
pub unsafe fn free_ffi_result(result: FfiResult) {
    free_c_string(result.data);
    free_c_string(result.error);
}
