// Simple test to verify v2.0 screen parsing works
use tn3270::template_parser::{TemplateParser, TemplateElement, Color3270};

#[test]
fn test_simple_v2_syntax() {
    let parser = TemplateParser::new();
    
    // Test a simple v2.0 bracket syntax
    let template = "[XY1,1][BLUE]Test line[/BLUE][XY2,5][YELLOW][BRIGHT]Bright text[/BRIGHT][/YELLOW]";
    let elements = parser.parse_template(template).unwrap();
    
    println!("Simple v2 test parsed {} elements", elements.len());
    assert_eq!(elements.len(), 2);
    
    // Check first element
    if let TemplateElement::Text { row, col, content, color, .. } = &elements[0] {
        assert_eq!(*row, Some(1));
        assert_eq!(*col, Some(1));
        assert_eq!(content, "Test line");
        assert_eq!(*color, Color3270::Blue);
    } else {
        panic!("Expected Text element");
    }
    
    // Check second element
    if let TemplateElement::Text { row, col, content, color, bright, .. } = &elements[1] {
        assert_eq!(*row, Some(2));
        assert_eq!(*col, Some(5));
        assert_eq!(content, "Bright text");
        assert_eq!(*color, Color3270::Yellow);
        assert_eq!(*bright, true);
    } else {
        panic!("Expected Text element");
    }
}

#[test]
fn test_v2_field_syntax() {
    let parser = TemplateParser::new();
    
    // Test v2.0 field syntax
    let template = "[XY10,15][FIELD username,length=20,uppercase=true]admin[/FIELD]";
    let elements = parser.parse_template(template).unwrap();
    
    println!("Field test parsed {} elements", elements.len());
    assert_eq!(elements.len(), 1);
    
    if let TemplateElement::Field { attributes, .. } = &elements[0] {
        assert_eq!(attributes.name, "username");
        assert_eq!(attributes.length, Some(20));
        assert_eq!(attributes.default_value, "admin");
        assert_eq!(attributes.uppercase, true);
    } else {
        panic!("Expected Field element");
    }
}
