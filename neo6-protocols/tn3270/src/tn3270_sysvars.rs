// TN3270 System Variables Module
// This module defines system variables used in TN3270 templates.

use chrono::Local;

/// Retrieves the current timestamp in the format "YYYY-MM-DD HH:MM:SS"
pub fn get_timestamp() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Retrieves the terminal type (default: "IBM-3278-2-E")
pub fn get_terminal_type() -> String {
    "IBM-3278-2-E".to_string()
}

/// Retrieves the system status (default: "ACTIVO")
pub fn get_system_status() -> String {
    "ACTIVO".to_string()
}
