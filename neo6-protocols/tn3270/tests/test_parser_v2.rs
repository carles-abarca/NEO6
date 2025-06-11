// Test para verificar el parser v2.0 con sintaxis de brackets

use tn3270::template_parser::{TemplateParser, TemplateElement, Color3270};

#[test]
fn test_position_tags_v2() {
let parser = TemplateParser::new();

// Test [XY5,10]
let template = "[XY5,10]Texto en posición específica";
let elements = parser.parse_template(template).unwrap();

assert_eq!(elements.len(), 1);
if let TemplateElement::Text { row, col, content, .. } = &elements[0] {
    assert_eq!(*row, Some(5));
    assert_eq!(*col, Some(10));
    assert_eq!(content, "Texto en posición específica");
} else {
    panic!("Expected Text element");
}
}

#[test]
fn test_column_only_tag_v2() {
    let parser = TemplateParser::new();
    
    // Test [X20]
    let template = "[X20]Texto alineado a columna 20";
    let elements = parser.parse_template(template).unwrap();
    
    assert_eq!(elements.len(), 1);
    if let TemplateElement::Text { row, col, content, .. } = &elements[0] {
        assert_eq!(*row, None);
        assert_eq!(*col, Some(20));
        assert_eq!(content, "Texto alineado a columna 20");
    } else {
        panic!("Expected Text element");
    }
}

#[test]
fn test_row_only_tag_v2() {
    let parser = TemplateParser::new();
    
    // Test [Y8]
    let template = "[Y8]Texto en fila 8";
    let elements = parser.parse_template(template).unwrap();
    
    assert_eq!(elements.len(), 1);
    if let TemplateElement::Text { row, col, content, .. } = &elements[0] {
        assert_eq!(*row, Some(8));
        assert_eq!(*col, None);
        assert_eq!(content, "Texto en fila 8");
    } else {
        panic!("Expected Text element");
    }
}

#[test]
fn test_color_tags_v2() {
    let parser = TemplateParser::new();
    
    // Test [BLUE]...[/BLUE]
    let template = "[BLUE]Texto azul[/BLUE]";
    let elements = parser.parse_template(template).unwrap();
    
    assert_eq!(elements.len(), 1);
    if let TemplateElement::Text { color, content, .. } = &elements[0] {
        assert_eq!(*color, Color3270::Blue);
        assert_eq!(content, "Texto azul");
    } else {
        panic!("Expected Text element");
    }
}

#[test]
fn test_attribute_tags_v2() {
    let parser = TemplateParser::new();
    
    // Test [BRIGHT]...[/BRIGHT]
    let template = "[BRIGHT]Texto brillante[/BRIGHT]";
    let elements = parser.parse_template(template).unwrap();
    
    assert_eq!(elements.len(), 1);
    if let TemplateElement::Text { bright, content, .. } = &elements[0] {
        assert_eq!(*bright, true);
        assert_eq!(content, "Texto brillante");
    } else {
        panic!("Expected Text element");
    }
}

#[test]
fn test_nested_tags_v2() {
    let parser = TemplateParser::new();
    
    // Test [YELLOW][BRIGHT]...[/BRIGHT][/YELLOW]
    let template = "[YELLOW][BRIGHT]Título destacado[/BRIGHT][/YELLOW]";
    let elements = parser.parse_template(template).unwrap();
    
    assert_eq!(elements.len(), 1);
    if let TemplateElement::Text { color, bright, content, .. } = &elements[0] {
        assert_eq!(*color, Color3270::Yellow);
        assert_eq!(*bright, true);
        assert_eq!(content, "Título destacado");
    } else {
        panic!("Expected Text element");
    }
}

#[test]
fn test_field_tags_v2() {
    let parser = TemplateParser::new();
    
    // Test [FIELD usuario,length=8]admin[/FIELD]
    let template = "[FIELD usuario,length=8]admin[/FIELD]";
    let elements = parser.parse_template(template).unwrap();
    
    assert_eq!(elements.len(), 1);
    if let TemplateElement::Field { attributes, .. } = &elements[0] {
        assert_eq!(attributes.name, "usuario");
        assert_eq!(attributes.length, Some(8));
        assert_eq!(attributes.default_value, "admin");
    } else {
        panic!("Expected Field element");
    }
}

#[test]
fn test_debug_field_parsing() {
    let parser = TemplateParser::new();
    
    // Test simple de campo
    let template = "[FIELD username,length=8][/FIELD]";
    let elements = parser.parse_template(template).unwrap();
    
    println!("Simple field test - Total elements: {}", elements.len());
    for (i, element) in elements.iter().enumerate() {
        match element {
            TemplateElement::Text { content, .. } => {
                println!("Element {}: Text '{}'", i, content);
            }
            TemplateElement::Field { attributes, .. } => {
                println!("Element {}: Field '{}'", i, attributes.name);
            }
        }
    }
    
    assert_eq!(elements.len(), 1);
    if let TemplateElement::Field { attributes, .. } = &elements[0] {
        assert_eq!(attributes.name, "username");
    } else {
        panic!("Expected Field element");
    }
}

#[test]
fn test_variable_replacement() {
    let parser = TemplateParser::new();
    
    let template = "[XY10,10]Timestamp: {timestamp}";
    let elements = parser.parse_template(template).unwrap();
    
    assert_eq!(elements.len(), 1);
    if let TemplateElement::Text { content, .. } = &elements[0] {
        assert!(content.starts_with("Timestamp: 20"));
        assert!(!content.contains("{timestamp}"));
    } else {
        panic!("Expected Text element");
    }
}

#[test]
fn test_position_bounds_validation() {
    let parser = TemplateParser::new();
    
    // Test posición fuera de límites
    let template = "[XY25,85]Texto fuera de límites";
    let result = parser.parse_template(template);
    
    assert!(result.is_err());
}

#[test]
fn test_unmatched_tags() {
    let parser = TemplateParser::new();
    
    // Test tag sin cerrar
    let template = "[BLUE]Texto sin cerrar";
    let result = parser.parse_template(template);
    
    assert!(result.is_err());
}

#[test]
fn test_real_v2_screen_content() {
    let parser = TemplateParser::new();
    
    // Test with actual v2.0 screen content (a subset of welcome screen)
    let template = "[XY1,1][BLUE]+=============================================================================+[/BLUE]
[XY2,1][BLUE]|[/BLUE][XY2,25][YELLOW][BRIGHT]ACCESO TN3270 A ENTORNO NEO6[/BRIGHT][/YELLOW][XY2,79][BLUE]|[/BLUE]
[XY7,1][BLUE]|[/BLUE][XY7,4][TURQUOISE]Estado del sistema:[/TURQUOISE][XY7,25][GREEN]ACTIVO[/GREEN][XY7,79][BLUE]|[/BLUE]
[XY21,1][PINK]COMANDO === [/PINK][YELLOW]>[/YELLOW] [XY21,15][FIELD command,length=20,uppercase=true][/FIELD]";
    
    let elements = parser.parse_template(template).unwrap();
    
    println!("Real v2 screen test - Total elements: {}", elements.len());
    for (i, element) in elements.iter().enumerate() {
        match element {
            TemplateElement::Text { row, col, content, color, bright, .. } => {
                println!("Element {}: Text at ({:?},{:?}) color={:?} bright={} content='{}'", 
                         i, row, col, color, bright, content);
            }
            TemplateElement::Field { attributes, .. } => {
                println!("Element {}: Field '{}' length={:?} uppercase={}", 
                         i, attributes.name, attributes.length, attributes.uppercase);
            }
        }
    }
    
    // Should have multiple elements
    assert!(elements.len() >= 10, "Should parse multiple elements from real screen content");
    
    // Check for specific elements we expect
    let has_title = elements.iter().any(|e| match e {
        TemplateElement::Text { content, color: Color3270::Yellow, bright: true, .. } => {
            content.contains("ACCESO TN3270")
        }
        _ => false,
    });
    assert!(has_title, "Should find the main title");
    
    let has_field = elements.iter().any(|e| match e {
        TemplateElement::Field { attributes, .. } => {
            attributes.name == "command" && attributes.uppercase == true
        }
        _ => false,
    });
    assert!(has_field, "Should find the command field");
}

#[test]
fn test_debug_v2_parsing_issue() {
    let parser = TemplateParser::new();
    
    // Test regex patterns first
    use regex::Regex;
    let pos_xy_regex = Regex::new(r"\[XY(\d+),(\d+)\]").unwrap();
    let pos_x_regex = Regex::new(r"\[X(\d+)\]").unwrap();
    let pos_y_regex = Regex::new(r"\[Y(\d+)\]").unwrap();
    let color_start_regex = Regex::new(r"\[(BLUE|RED|PINK|GREEN|TURQUOISE|YELLOW|WHITE|DEFAULT)\]").unwrap();
    
    println!("=== Testing regex patterns ===");
    let test_string = "[BLUE]|[/BLUE][XY2,25]";
    println!("Testing all regexes on: {}", test_string);
    
    if let Some(cap) = pos_xy_regex.captures(test_string) {
        println!("  XY regex matched: {:?}", cap);
    } else {
        println!("  XY regex: No match");
    }
    
    if let Some(cap) = pos_x_regex.captures(test_string) {
        println!("  X regex matched: {:?}", cap);
    } else {
        println!("  X regex: No match");
    }
    
    if let Some(cap) = pos_y_regex.captures(test_string) {
        println!("  Y regex matched: {:?}", cap);
    } else {
        println!("  Y regex: No match");
    }
    
    if let Some(cap) = color_start_regex.captures(test_string) {
        println!("  Color regex matched: {:?}", cap);
        println!("  Color: {}", &cap[1]);
    } else {
        println!("  Color regex: No match");
    }
    
    // Test simple cases first
    println!("\n=== Testing simple position tag ===");
    let simple_pos = "[XY1,1]Simple text";
    let elements = parser.parse_template(simple_pos).unwrap();
    println!("Simple position: {} elements", elements.len());
    for (i, e) in elements.iter().enumerate() {
        println!("  {}: {:?}", i, e);
    }
    
    println!("\n=== Testing simple color tag ===");
    let simple_color = "[BLUE]Blue text[/BLUE]";
    let elements = parser.parse_template(simple_color).unwrap();
    println!("Simple color: {} elements", elements.len());
    for (i, e) in elements.iter().enumerate() {
        println!("  {}: {:?}", i, e);
    }
    
    println!("\n=== Testing combined position and color ===");
    let combined = "[XY2,5][BLUE]Blue text[/BLUE]";
    let elements = parser.parse_template(combined).unwrap();
    println!("Combined: {} elements", elements.len());
    for (i, e) in elements.iter().enumerate() {
        println!("  {}: {:?}", i, e);
    }
    
    println!("\n=== Testing problematic line ===");
    let problematic = "[XY2,1][BLUE]|[/BLUE][XY2,25][YELLOW][BRIGHT]ACCESO TN3270[/BRIGHT][/YELLOW]";
    println!("Parsing: {}", problematic);
    let result = parser.parse_template(problematic);
    match result {
        Ok(elements) => {
            println!("Problematic line: {} elements", elements.len());
            for (i, e) in elements.iter().enumerate() {
                println!("  {}: {:?}", i, e);
            }
        }
        Err(e) => {
            println!("Error parsing problematic line: {:?}", e);
        }
    }
    
    println!("\n=== Testing simpler problematic case ===");
    let simple_problematic = "[BLUE]text[/BLUE]more";
    println!("Parsing: {}", simple_problematic);
    let result = parser.parse_template(simple_problematic);
    match result {
        Ok(elements) => {
            println!("Simple problematic: {} elements", elements.len());
            for (i, e) in elements.iter().enumerate() {
                println!("  {}: {:?}", i, e);
            }
        }
        Err(e) => {
            println!("Error parsing simple problematic: {:?}", e);
        }
    }
}
