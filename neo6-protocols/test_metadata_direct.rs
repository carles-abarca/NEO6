use std::path::PathBuf;
use tn3270::{ScreenManager, TemplateParser};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test metadata extraction
    let screens_dir = PathBuf::from("/Users/carlesabarca/MyProjects/NEO6/neo6-proxy/config/screens");
    let screen_manager = ScreenManager::new(screens_dir);
    
    println!("=== Testing Screen List Generation ===");
    let screen_list = screen_manager.generate_screen_list()?;
    println!("Generated screen list:\n{}", screen_list);
    
    println!("\n=== Testing COMMANDS Screen Generation ===");
    let commands_screen = screen_manager.generate_tn3270_screen("COMMANDS", &std::collections::HashMap::new())?;
    
    // Check if {screen_list} was replaced
    let screen_str = String::from_utf8_lossy(&commands_screen);
    if screen_str.contains("{screen_list}") {
        println!("ERROR: {screen_list} variable was not replaced!");
    } else {
        println!("SUCCESS: {screen_list} variable was replaced with actual content!");
    }
    
    Ok(())
}
