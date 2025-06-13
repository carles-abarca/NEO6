// TN3270 System Variables Module
// This module defines system variables used in TN3270 templates.

use chrono::Local;

/// System context that can be passed to system variable functions
pub struct SystemContext {
    pub terminal_type: Option<String>,
    pub logical_unit: Option<String>,
}

impl SystemContext {
    pub fn new() -> Self {
        Self {
            terminal_type: None,
            logical_unit: None,
        }
    }

    pub fn with_terminal_type(terminal_type: String) -> Self {
        Self {
            terminal_type: Some(terminal_type),
            logical_unit: None,
        }
    }

    pub fn with_session(terminal_type: String, logical_unit: String) -> Self {
        Self {
            terminal_type: Some(terminal_type),
            logical_unit: Some(logical_unit),
        }
    }
}

/// Retrieves the current timestamp in the format "YYYY-MM-DD HH:MM:SS"
pub fn get_timestamp() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Retrieves the terminal type with optional session context
pub fn get_terminal_type_with_context(context: Option<&SystemContext>) -> String {
    context
        .and_then(|ctx| ctx.terminal_type.as_deref())
        .filter(|t| !t.is_empty())
        .unwrap_or("IBM-3278-2-e")
        .to_string()
}

/// Retrieves the terminal type (default: "IBM-3278-2-E")
pub fn get_terminal_type() -> String {
    get_terminal_type_with_context(None)
}

/// Retrieves the system status (default: "ACTIVO")
pub fn get_system_status() -> String {
    "ACTIVO".to_string()
}
