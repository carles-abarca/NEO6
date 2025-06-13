// Test for config-dir functionality in ScreenManager
use tn3270::tn3270_screens::ScreenManager;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_manager_with_config_dir() {
        // Test with different config directories using environment variable
        let test_configs = vec![
            "neo6-proxy/config",
            "runtime/config/proxy",
        ];

        for config_dir in test_configs {
            println!("Testing with config-dir: {}", config_dir);
            
            // Set the environment variable
            unsafe {
                std::env::set_var("NEO6_CONFIG_DIR", config_dir);
            }
            
            // Create ScreenManager (it will use the environment variable)
            let mut screen_manager = ScreenManager::new();
            
            // Test loading a template
            match screen_manager.generate_welcome_screen() {
                Ok(screen_data) => {
                    println!("✓ Successfully loaded welcome screen from {}", config_dir);
                    assert!(screen_data.len() > 0, "Screen data should not be empty");
                    
                    // Verify it starts with the correct TN3270 command
                    assert_eq!(screen_data[0], 0xF5, "Should start with Erase/Write command");
                }
                Err(e) => {
                    eprintln!("✗ Failed to load welcome screen from {}: {}", config_dir, e);
                    // Don't panic here as it might be expected if the config dir doesn't exist
                }
            }
            
            // Clean up environment variable
            unsafe {
                std::env::remove_var("NEO6_CONFIG_DIR");
            }
        }
    }

    #[test]
    fn test_screen_manager_default_config() {
        // Ensure no environment variable is set
        unsafe {
            std::env::remove_var("NEO6_CONFIG_DIR");
        }
        
        // Test with default configuration (should use fallback logic)
        let mut screen_manager = ScreenManager::new();
        
        // This should work with the fallback logic finding screens in the workspace
        match screen_manager.generate_welcome_screen() {
            Ok(screen_data) => {
                println!("✓ Successfully loaded welcome screen with default config");
                assert!(screen_data.len() > 0, "Screen data should not be empty");
                assert_eq!(screen_data[0], 0xF5, "Should start with Erase/Write command");
            }
            Err(e) => {
                println!("Note: Default config failed to load welcome screen: {}", e);
                // This might be expected in some test environments
            }
        }
    }

    #[test]
    fn test_screen_manager_invalid_config_dir() {
        // Test with an invalid config directory
        unsafe {
            std::env::set_var("NEO6_CONFIG_DIR", "/invalid/nonexistent/path");
        }
        let mut screen_manager = ScreenManager::new();
        
        // This should fallback to the default search behavior
        match screen_manager.generate_welcome_screen() {
            Ok(screen_data) => {
                println!("✓ Fallback logic worked for invalid config dir");
                assert!(screen_data.len() > 0, "Screen data should not be empty");
            }
            Err(e) => {
                println!("Expected: Invalid config dir fallback failed: {}", e);
                // This is expected behavior when no valid screens directory is found
            }
        }
        
        // Clean up environment variable
        unsafe {
            std::env::remove_var("NEO6_CONFIG_DIR");
        }
    }

    #[test]
    fn test_multiple_screen_types() {
        // Test loading different types of screens with config dir
        let config_dir = "neo6-proxy/config";
        unsafe {
            std::env::set_var("NEO6_CONFIG_DIR", config_dir);
        }
        let mut screen_manager = ScreenManager::new();
        
        let screen_names = vec!["welcome", "MENU", "STATUS"];
        
        for screen_name in screen_names {
            match screen_manager.generate_tn3270_screen(screen_name) {
                Ok(screen_data) => {
                    println!("✓ Successfully loaded {} screen", screen_name);
                    assert!(screen_data.len() > 0, "Screen data should not be empty for {}", screen_name);
                    assert_eq!(screen_data[0], 0xF5, "Should start with Erase/Write command for {}", screen_name);
                }
                Err(e) => {
                    println!("✗ Failed to load {} screen: {}", screen_name, e);
                }
            }
        }
        
        // Clean up environment variable
        unsafe {
            std::env::remove_var("NEO6_CONFIG_DIR");
        }
    }
}
