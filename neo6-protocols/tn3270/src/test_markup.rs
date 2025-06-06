// Test module for the markup language system
use crate::{ScreenManager, TemplateParser, Codec};

pub fn test_markup_integration() {
    println!("Testing TN3270 markup language integration...");
    
    // Create a screen manager
    let mut screen_manager = ScreenManager::new();
    
    // Test generating welcome screen with markup
    match screen_manager.generate_welcome_screen() {
        Ok(screen_data) => {
            println!("✅ Welcome screen generated successfully!");
            println!("   Screen data size: {} bytes", screen_data.len());
            
            // Show first few bytes for debugging
            if screen_data.len() > 10 {
                print!("   First 10 bytes: ");
                for i in 0..10 {
                    print!("{:02X} ", screen_data[i]);
                }
                println!();
            }
        }
        Err(e) => {
            println!("❌ Error generating welcome screen: {}", e);
        }
    }
    
    // Test the template parser directly
    println!("\nTesting template parser...");
    let parser = TemplateParser::new();
    
    let test_markup = r#"<pos:1,1><color:red>Hello</color> <color:green>World</color></pos>
<pos:2,5><field:test,length=10>default</field></pos>"#;
    
    match parser.parse_template(test_markup) {
        Ok(elements) => {
            println!("✅ Template parsed successfully!");
            println!("   Found {} elements", elements.len());
            for (i, element) in elements.iter().enumerate() {
                println!("   Element {}: {:?}", i + 1, element);
            }
        }
        Err(e) => {
            println!("❌ Error parsing template: {}", e);
        }
    }
    
    println!("\nMarkup integration test completed!");
}
