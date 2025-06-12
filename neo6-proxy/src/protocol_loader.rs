// Dynamic protocol loader for neo6-proxy

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use libloading::{Library, Symbol};
use tracing::{info};

use neo6_protocols_lib::ffi::{
    ProtocolHandle, ProtocolInterface, GetProtocolInterfaceFn,
    free_ffi_result, create_c_string, c_str_to_string
};

/// Wrapper for dynamically loaded protocol
pub struct DynamicProtocolHandler {
    _library: Arc<Library>,
    interface: *const ProtocolInterface,
    handle: ProtocolHandle,
}

// Mark as Send and Sync for multi-threading (we ensure proper usage)
unsafe impl Send for DynamicProtocolHandler {}
unsafe impl Sync for DynamicProtocolHandler {}

impl DynamicProtocolHandler {
    /// Load a protocol from a dynamic library
    pub fn load_from_library<P: AsRef<Path>>(lib_path: P) -> Result<Self, String> {
        let lib_path = lib_path.as_ref();
        
        // Load the dynamic library
        let library = unsafe {
            Library::new(lib_path)
                .map_err(|e| format!("Failed to load library {:?}: {}", lib_path, e))?
        };

        // Get the protocol interface function
        let get_interface: Symbol<GetProtocolInterfaceFn> = unsafe {
            library.get(b"get_protocol_interface\0")
                .map_err(|e| format!("Failed to get protocol interface function: {}", e))?
        };

        // Get the protocol interface
        let interface = unsafe { get_interface() };
        if interface.is_null() {
            return Err("Protocol interface is null".to_string());
        }

        // Create a handler instance
        let handle = unsafe { ((*interface).create_handler)() };
        if handle.is_null() {
            return Err("Failed to create protocol handler".to_string());
        }

        Ok(DynamicProtocolHandler {
            _library: Arc::new(library),
            interface,
            handle,
        })
    }

    /// Invoke a transaction on the protocol
    pub fn invoke_transaction(&self, transaction_id: &str, parameters: serde_json::Value) -> Result<serde_json::Value, String> {
        let tx_id_cstr = create_c_string(transaction_id);
        let params_str = serde_json::to_string(&parameters)
            .map_err(|e| format!("Failed to serialize parameters: {}", e))?;
        let params_cstr = create_c_string(&params_str);

        let result = unsafe {
            ((*self.interface).invoke_transaction)(self.handle, tx_id_cstr, params_cstr)
        };

        // Free the C strings
        unsafe {
            neo6_protocols_lib::ffi::free_c_string(tx_id_cstr);
            neo6_protocols_lib::ffi::free_c_string(params_cstr);
        }

        // Process the result
        if result.success == 0 {
            let data_str = unsafe { c_str_to_string(result.data) }
                .unwrap_or_default();
            
            let json_result = serde_json::from_str(&data_str)
                .map_err(|e| format!("Failed to parse result JSON: {}", e))?;
            
            unsafe { free_ffi_result(result); }
            Ok(json_result)
        } else {
            let error_str = unsafe { c_str_to_string(result.error) }
                .unwrap_or_else(|| "Unknown error".to_string());
            
            unsafe { free_ffi_result(result); }
            Err(error_str)
        }
    }

    /// Start a protocol listener if supported
    pub fn start_listener(&self, port: u16) -> Result<serde_json::Value, String> {
        unsafe {
            if let Some(start_listener_fn) = (*self.interface).start_listener {
                let result = start_listener_fn(self.handle, port);
                
                // Process the result
                if result.success == 0 {
                    let data_str = c_str_to_string(result.data).unwrap_or_default();
                    let json_result = serde_json::from_str(&data_str)
                        .map_err(|e| format!("Failed to parse result JSON: {}", e))?;
                    
                    free_ffi_result(result);
                    Ok(json_result)
                } else {
                    let error_str = c_str_to_string(result.error)
                        .unwrap_or_else(|| "Unknown error".to_string());
                    
                    free_ffi_result(result);
                    Err(error_str)
                }
            } else {
                Err("Protocol does not support listeners".to_string())
            }
        }
    }

    /// Set logging level for this protocol
    pub fn set_log_level(&self, log_level: &str) -> Result<serde_json::Value, String> {
        unsafe {
            if let Some(set_log_level_fn) = (*self.interface).set_log_level {
                let level_cstr = create_c_string(log_level);
                let result = set_log_level_fn(level_cstr);
                
                // Free the C string
                neo6_protocols_lib::ffi::free_c_string(level_cstr);
                
                // Process the result
                if result.success == 0 {
                    let data_str = c_str_to_string(result.data).unwrap_or_default();
                    let json_result = serde_json::from_str(&data_str)
                        .map_err(|e| format!("Failed to parse result JSON: {}", e))?;
                    
                    free_ffi_result(result);
                    Ok(json_result)
                } else {
                    let error_str = c_str_to_string(result.error)
                        .unwrap_or_else(|| "Unknown error".to_string());
                    
                    free_ffi_result(result);
                    Err(error_str)
                }
            } else {
                // If protocol doesn't support set_log_level, that's ok
                Ok(serde_json::json!({"status": "not_supported"}))
            }
        }
    }
}

impl Drop for DynamicProtocolHandler {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                ((*self.interface).destroy_handler)(self.handle);
            }
        }
    }
}

/// Protocol loader manager
pub struct ProtocolLoader {
    loaded_protocols: RwLock<HashMap<String, Arc<DynamicProtocolHandler>>>,
    library_paths: HashMap<String, PathBuf>,
}

impl ProtocolLoader {
    /// Create a new protocol loader
    pub fn new() -> Self {
        let mut library_paths = HashMap::new();
        
        // Default library paths for each protocol
        let protocols = ["tn3270", "lu62", "mq", "rest", "tcp", "jca"];
        
        for protocol in &protocols {
            let lib_name = if cfg!(target_os = "macos") {
                format!("lib{}.dylib", protocol)
            } else if cfg!(target_os = "windows") {
                format!("{}.dll", protocol)
            } else {
                format!("lib{}.so", protocol)
            };
            
            // Look in the target directory first, then in the library directory
            let mut path = PathBuf::from("../neo6-protocols/target/debug");
            path.push(&lib_name);
            
            if !path.exists() {
                path = PathBuf::from("./lib");
                path.push(&lib_name);
            }
            
            library_paths.insert(protocol.to_string(), path);
        }

        ProtocolLoader {
            loaded_protocols: RwLock::new(HashMap::new()),
            library_paths,
        }
    }

    /// Load a protocol if not already loaded
    pub fn load_protocol(&self, protocol_name: &str) -> Result<Arc<DynamicProtocolHandler>, String> {
        // Check if already loaded
        {
            let loaded = self.loaded_protocols.read().unwrap();
            if let Some(handler) = loaded.get(protocol_name) {
                return Ok(handler.clone());
            }
        }

        // Load the protocol
        let lib_path = self.library_paths.get(protocol_name)
            .ok_or_else(|| format!("Unknown protocol: {}", protocol_name))?;

        info!("Loading protocol {} from {:?}", protocol_name, lib_path);

        let handler = DynamicProtocolHandler::load_from_library(lib_path)
            .map_err(|e| format!("Failed to load protocol {}: {}", protocol_name, e))?;

        let handler = Arc::new(handler);

        // Store in cache
        {
            let mut loaded = self.loaded_protocols.write().unwrap();
            loaded.insert(protocol_name.to_string(), handler.clone());
        }

        info!("Successfully loaded protocol {}", protocol_name);
        Ok(handler)
    }

    /// Get a loaded protocol handler
    pub fn get_protocol(&self, protocol_name: &str) -> Option<Arc<DynamicProtocolHandler>> {
        let loaded = self.loaded_protocols.read().unwrap();
        loaded.get(protocol_name).cloned()
    }

    /// Unload a protocol
    pub fn unload_protocol(&self, protocol_name: &str) -> bool {
        let mut loaded = self.loaded_protocols.write().unwrap();
        loaded.remove(protocol_name).is_some()
    }

    /// List loaded protocols
    pub fn list_loaded_protocols(&self) -> Vec<String> {
        let loaded = self.loaded_protocols.read().unwrap();
        loaded.keys().cloned().collect()
    }

    /// Set log level for all loaded protocols
    pub fn set_log_level_for_all(&self, log_level: &str) {
        let loaded = self.loaded_protocols.read().unwrap();
        for (protocol_name, handler) in loaded.iter() {
            match handler.set_log_level(log_level) {
                Ok(_) => info!("Configurado nivel de log '{}' para protocolo {}", log_level, protocol_name),
                Err(e) => info!("Protocolo {} no soporta configuración de log: {}", protocol_name, e),
            }
        }
    }
}

impl Default for ProtocolLoader {
    fn default() -> Self {
        Self::new()
    }
}
