// Test para verificar el parser v2.0 con sintaxis de brackets

#[cfg(test)]
mod tests {
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
}
