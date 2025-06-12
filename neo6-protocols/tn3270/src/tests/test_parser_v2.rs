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
}
