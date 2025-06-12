// Test para verificar el manejo correcto de saltos de línea en templates

#[cfg(test)]
mod tests {
    use tn3270::template_parser::{TemplateParser, TemplateElement};

#[test]
fn test_multiline_template_parsing() {
    let parser = TemplateParser::new();
    
    // Template con saltos de línea como en welcome_screen.txt
    let template = r#"[XY1,1][BLUE]Header Line[/BLUE]
[XY2,1][BLUE]|[/BLUE][XY2,10][GREEN]Content[/GREEN][XY2,20][BLUE]|[/BLUE]

[XY4,1][WHITE]Another line[/WHITE]
[XY5,15][FIELD command,length=10][/FIELD]"#;
    
    println!("Testing multiline template parsing...");
    let elements = parser.parse_template(template).unwrap();
    
    println!("Parsed {} elements", elements.len());
    for (i, element) in elements.iter().enumerate() {
        match element {
            TemplateElement::Text { row, col, content, color, .. } => {
                println!("Element {}: Text at ({:?},{:?}) color={:?} content='{}'", 
                         i, row, col, color, content);
            }
            TemplateElement::Field { attributes, row, col } => {
                println!("Element {}: Field '{}' at ({:?},{:?})", 
                         i, attributes.name, row, col);
            }
        }
    }
    
    // Verificar que tenemos los elementos esperados
    assert!(elements.len() >= 5, "Should have parsed multiple elements");
    
    // Verificar elementos específicos
    let has_header = elements.iter().any(|e| match e {
        TemplateElement::Text { content, .. } => content.contains("Header Line"),
        _ => false,
    });
    assert!(has_header, "Should find header text");
    
    let has_field = elements.iter().any(|e| match e {
        TemplateElement::Field { attributes, .. } => attributes.name == "command",
        _ => false,
    });
    assert!(has_field, "Should find command field");
}

#[test]
fn test_welcome_screen_parsing() {
    let parser = TemplateParser::new();
    
    // Leer el archivo welcome_screen.txt
    let screen_content = std::fs::read_to_string("/Users/carlesabarca/MyProjects/NEO6/neo6-proxy/config/screens/welcome_screen.txt")
        .expect("Could not read welcome_screen.txt");
    
    println!("Testing actual welcome screen parsing...");
    let elements = parser.parse_template(&screen_content).unwrap();
    
    println!("Welcome screen parsed {} elements", elements.len());
    
    // Buscar el campo command específicamente
    let command_field = elements.iter().find(|e| match e {
        TemplateElement::Field { attributes, .. } => attributes.name == "command",
        _ => false,
    });
    
    assert!(command_field.is_some(), "Should find command field");
    
    if let Some(TemplateElement::Field { attributes, row, col }) = command_field {
        println!("Command field found at ({:?},{:?}) with length={:?}", 
                 row, col, attributes.length);
        assert_eq!(*row, Some(21), "Command field should be at row 21");
        assert_eq!(*col, Some(15), "Command field should be at column 15");
        assert_eq!(attributes.length, Some(20), "Command field should have length 20");
    }
}

#[test]
fn test_empty_lines_ignored() {
    let parser = TemplateParser::new();
    
    // Template con líneas vacías que deberían ser ignoradas
    let template = r#"[XY1,1][BLUE]First[/BLUE]

[XY2,1][GREEN]Second[/GREEN]


[XY3,1][RED]Third[/RED]"#;
    
    let elements = parser.parse_template(template).unwrap();
    
    // Debe tener exactamente 3 elementos de texto
    let text_elements: Vec<_> = elements.iter().filter(|e| match e {
        TemplateElement::Text { .. } => true,
        _ => false,
    }).collect();
    
    assert_eq!(text_elements.len(), 3, "Should have exactly 3 text elements");
}
}
