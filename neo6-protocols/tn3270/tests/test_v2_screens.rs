// Test to verify that the new v2.0 screen files work with our parser
use tn3270::template_parser::{TemplateParser, TemplateElement, Color3270};
use std::fs;

#[test]
fn test_v2_welcome_screen_parsing() {
    let parser = TemplateParser::new();
    
    // Read the new v2.0 welcome screen file
    let screen_content = fs::read_to_string("/Users/carlesabarca/MyProjects/NEO6/neo6-proxy/config/screens/welcome_screen.txt")
        .expect("Could not read welcome_screen.txt");
    
    println!("Testing v2.0 welcome screen content:");
    println!("{}", screen_content);
    
    // Parse the v2.0 template
    let elements = parser.parse_template(&screen_content)
        .expect("Failed to parse v2.0 welcome screen");
    
    println!("Parsed {} elements from welcome_screen.txt", elements.len());
    
    // Verify we have elements
    assert!(!elements.is_empty(), "Should have parsed elements from the screen");
    
    // Check that we have positioned text elements
    let positioned_elements: Vec<_> = elements.iter()
        .filter(|e| match e {
            TemplateElement::Text { row: Some(_), col: Some(_), .. } => true,
            _ => false,
        })
        .collect();
    
    assert!(!positioned_elements.is_empty(), "Should have positioned text elements");
    
    // Check that we have colored elements
    let colored_elements: Vec<_> = elements.iter()
        .filter(|e| match e {
            TemplateElement::Text { color: Color3270::Blue, .. } |
            TemplateElement::Text { color: Color3270::Yellow, .. } |
            TemplateElement::Text { color: Color3270::White, .. } |
            TemplateElement::Text { color: Color3270::Turquoise, .. } |
            TemplateElement::Text { color: Color3270::Green, .. } |
            TemplateElement::Text { color: Color3270::Pink, .. } => true,
            _ => false,
        })
        .collect();
    
    assert!(!colored_elements.is_empty(), "Should have colored elements");
    
    // Check that we have field elements
    let field_elements: Vec<_> = elements.iter()
        .filter(|e| match e {
            TemplateElement::Field { .. } => true,
            _ => false,
        })
        .collect();
    
    assert!(!field_elements.is_empty(), "Should have field elements");
    
    // Print some sample elements for verification
    for (i, element) in elements.iter().take(5).enumerate() {
        match element {
            TemplateElement::Text { row, col, content, color, bright, .. } => {
                println!("Element {}: Text at ({:?},{:?}) color={:?} bright={} content='{}'", 
                         i, row, col, color, bright, content);
            }
            TemplateElement::Field { attributes, .. } => {
                println!("Element {}: Field '{}' length={:?}", 
                         i, attributes.name, attributes.length);
            }
        }
    }
}

#[test]
fn test_v2_colors_screen_parsing() {
    let parser = TemplateParser::new();
    
    // Read the new v2.0 colors screen file
    let screen_content = fs::read_to_string("/Users/carlesabarca/MyProjects/NEO6/neo6-proxy/config/screens/COLORS_screen.txt")
        .expect("Could not read COLORS_screen.txt");
    
    // Parse the v2.0 template
    let elements = parser.parse_template(&screen_content)
        .expect("Failed to parse v2.0 colors screen");
    
    println!("Parsed {} elements from COLORS_screen.txt", elements.len());
    
    // Verify we have elements
    assert!(!elements.is_empty(), "Should have parsed elements from the colors screen");
    
    // Check for specific color elements that should be present
    let color_tests = vec![
        (Color3270::Blue, "BLUE"),
        (Color3270::Red, "RED"),
        (Color3270::Pink, "PINK"),
        (Color3270::Green, "GREEN"),
        (Color3270::Turquoise, "TURQUOISE"),
        (Color3270::Yellow, "YELLOW"),
        (Color3270::White, "WHITE"),
    ];
    
    for (expected_color, color_name) in color_tests {
        let found = elements.iter().any(|e| match e {
            TemplateElement::Text { color, content, .. } if *color == expected_color => {
                content.contains(color_name)
            }
            _ => false,
        });
        assert!(found, "Should find {:?} colored text containing '{}'", expected_color, color_name);
    }
    
    // Check for bright text
    let bright_elements: Vec<_> = elements.iter()
        .filter(|e| match e {
            TemplateElement::Text { bright: true, .. } => true,
            _ => false,
        })
        .collect();
    
    assert!(!bright_elements.is_empty(), "Should have bright text elements");
}

#[test]
fn test_v2_menu_screen_parsing() {
    let parser = TemplateParser::new();
    
    // Read the new v2.0 menu screen file
    let screen_content = fs::read_to_string("/Users/carlesabarca/MyProjects/NEO6/neo6-proxy/config/screens/MENU_screen.txt")
        .expect("Could not read MENU_screen.txt");
    
    // Parse the v2.0 template
    let elements = parser.parse_template(&screen_content)
        .expect("Failed to parse v2.0 menu screen");
    
    println!("Parsed {} elements from MENU_screen.txt", elements.len());
    
    // Verify we have elements
    assert!(!elements.is_empty(), "Should have parsed elements from the menu screen");
    
    // Check for the title
    let title_found = elements.iter().any(|e| match e {
        TemplateElement::Text { content, color: Color3270::Yellow, bright: true, .. } => {
            content.contains("MENU PRINCIPAL NEO6")
        }
        _ => false,
    });
    assert!(title_found, "Should find the menu title in yellow bright text");
    
    // Check for menu options
    let menu_options = vec!["WELCOME", "STATUS", "TRANS", "COLORS", "FIELDS", "HELP", "EXIT"];
    for option in menu_options {
        let found = elements.iter().any(|e| match e {
            TemplateElement::Text { content, .. } => content.contains(option),
            _ => false,
        });
        assert!(found, "Should find menu option '{}'", option);
    }
}
