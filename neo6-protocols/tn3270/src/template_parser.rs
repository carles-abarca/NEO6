// Template parser para el lenguaje de marcado de pantallas TN3270
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use chrono::{Local, DateTime};
use regex::Regex;
use tracing::{trace, debug};

/// Errores específicos del parser de templates
#[derive(Debug)]
pub enum TemplateError {
    InvalidPosition(String),
    InvalidColor(String),
    InvalidField(String),
    UnmatchedTag(String),
    PositionOutOfBounds(u16, u16),
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        debug!("Entering TemplateError::fmt");
        match self {
            TemplateError::InvalidPosition(msg) => write!(f, "Posición inválida: {}", msg),
            TemplateError::InvalidColor(msg) => write!(f, "Color inválido: {}", msg),
            TemplateError::InvalidField(msg) => write!(f, "Campo inválido: {}", msg),
            TemplateError::UnmatchedTag(tag) => write!(f, "Etiqueta sin cerrar: {}", tag),
            TemplateError::PositionOutOfBounds(row, col) => {
                write!(f, "Posición fuera de límites: fila {}, columna {}", row, col)
            }
        }
    }
}

impl Error for TemplateError {}

/// Representa un color 3270
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color3270 {
    Default = 0x00,
    Blue = 0xF1,
    Red = 0xF2,
    Pink = 0xF3,
    Green = 0xF4,
    Turquoise = 0xF5,
    Yellow = 0xF6,
    White = 0xF7,
}

impl Color3270 {
    pub fn from_str(color_name: &str) -> Result<Self, TemplateError> {
        debug!("Entering Color3270::from_str");
        match color_name.to_lowercase().as_str() {
            "default" => Ok(Color3270::Default),
            "blue" => Ok(Color3270::Blue),
            "red" => Ok(Color3270::Red),
            "pink" | "magenta" => Ok(Color3270::Pink),
            "green" => Ok(Color3270::Green),
            "turquoise" | "cyan" => Ok(Color3270::Turquoise),
            "yellow" => Ok(Color3270::Yellow),
            "white" => Ok(Color3270::White),
            _ => Err(TemplateError::InvalidColor(format!("Color desconocido: {}", color_name))),
        }
    }
}

/// Atributos de un campo de entrada
#[derive(Debug, Clone)]
pub struct FieldAttributes {
    pub name: String,
    pub length: Option<usize>,
    pub hidden: bool,
    pub numeric: bool,
    pub uppercase: bool,
    pub protected: bool,
    pub default_value: String,
}

impl FieldAttributes {
    pub fn new(name: String) -> Self {
        debug!("Entering FieldAttributes::new");
        Self {
            name,
            length: None,
            hidden: false,
            numeric: false,
            uppercase: false,
            protected: false,
            default_value: String::new(),
        }
    }

    pub fn parse_attributes(attr_str: &str) -> Result<HashMap<String, String>, TemplateError> {
        debug!("Entering FieldAttributes::parse_attributes");
        let mut attributes = HashMap::new();
        
        for attr in attr_str.split(',') {
            let attr = attr.trim();
            if attr.contains('=') {
                let parts: Vec<&str> = attr.splitn(2, '=').collect();
                if parts.len() == 2 {
                    attributes.insert(parts[0].to_string(), parts[1].to_string());
                }
            } else {
                // Atributo booleano (sin valor)
                attributes.insert(attr.to_string(), "true".to_string());
            }
        }
        
        Ok(attributes)
    }

    pub fn apply_attributes(&mut self, attributes: HashMap<String, String>) -> Result<(), TemplateError> {
        debug!("Entering FieldAttributes::apply_attributes");
        for (key, value) in attributes {
            match key.as_str() {
                "length" => {
                    self.length = Some(value.parse().map_err(|_| {
                        TemplateError::InvalidField(format!("Longitud inválida: {}", value))
                    })?);
                }
                "hidden" => self.hidden = value == "true",
                "numeric" => self.numeric = value == "true",
                "uppercase" => self.uppercase = value == "true",
                "protected" => self.protected = value == "true",
                _ => return Err(TemplateError::InvalidField(format!("Atributo desconocido: {}", key))),
            }
        }
        Ok(())
    }
}

/// Elemento procesado de la plantilla
#[derive(Debug, Clone)]
pub enum TemplateElement {
    Text {
        content: String,
        color: Color3270,
        row: Option<u16>,
        col: Option<u16>,
        bright: bool,
        blink: bool,
        underline: bool,
    },
    Field {
        attributes: FieldAttributes,
        row: Option<u16>,
        col: Option<u16>,
    },
}

/// Parser principal para templates de pantallas TN3270
pub struct TemplateParser {
    variables: HashMap<String, String>,
}

impl TemplateParser {
    // ...existing code...
    
    /// Calculate the actual text length by stripping markup tags
    fn calculate_text_length(content: &str) -> usize {
        // Simple regex to remove all markup tags
        let re = Regex::new(r"<[^>]*>").unwrap();
        re.replace_all(content, "").len()
    }
    
    /// Advance cursor position with bounds checking and line wrapping
    fn advance_position(current_row: Option<u16>, current_col: Option<u16>, advance_by: u16) -> (Option<u16>, Option<u16>) {
        if let (Some(row), Some(col)) = (current_row, current_col) {
            let new_col = col + advance_by;
            if new_col > 80 {
                // Handle line wrapping - advance to next line
                let lines_to_advance = (new_col - 1) / 80;
                let final_col = ((new_col - 1) % 80) + 1;
                let final_row = row + lines_to_advance;
                
                // Check if we're going beyond screen bounds (24 rows)
                if final_row > 24 {
                    // Don't advance beyond screen bounds
                    (Some(row), Some(col))
                } else {
                    (Some(final_row), Some(final_col))
                }
            } else {
                (Some(row), Some(new_col))
            }
        } else {
            (current_row, current_col)
        }
    }
    
    pub fn new() -> Self {
        debug!("Entering TemplateParser::new");
        let mut parser = Self {
            variables: HashMap::new(),
        };
        parser.init_system_variables();
        parser
    }

    /// Inicializa variables del sistema
    fn init_system_variables(&mut self) {
        debug!("Entering TemplateParser::init_system_variables");
        let now: DateTime<Local> = Local::now();
        
        self.variables.insert("timestamp".to_string(), now.format("%Y-%m-%d %H:%M:%S").to_string());
        self.variables.insert("terminal_type".to_string(), "IBM-3278-2-E".to_string());
        self.variables.insert("system_date".to_string(), now.format("%Y-%m-%d").to_string());
        self.variables.insert("system_time".to_string(), now.format("%H:%M:%S").to_string());
        self.variables.insert("user_id".to_string(), "ADMIN01".to_string());
        self.variables.insert("session_id".to_string(), "SES001".to_string());
    }

    /// Añade o actualiza una variable personalizada
    pub fn set_variable(&mut self, name: String, value: String) {
        debug!("Entering TemplateParser::set_variable");
        self.variables.insert(name, value);
    }

    /// Procesa una plantilla completa
    pub fn parse_template(&self, template_content: &str) -> Result<Vec<TemplateElement>, Box<dyn Error>> {
        debug!("Entering TemplateParser::parse_template");
        // Paso 1: Reemplazar variables del sistema
        let content_with_vars = self.replace_variables(template_content)?;
        
        // Paso 2: Procesar etiquetas de marcado
        let elements = self.parse_markup_tags(&content_with_vars)?;
        
        // Paso 3: Validar posiciones
        self.validate_elements(&elements)?;
        
        Ok(elements)
    }

    /// Reemplaza las variables del sistema en el contenido
    fn replace_variables(&self, content: &str) -> Result<String, Box<dyn Error>> {
        debug!("Entering TemplateParser::replace_variables");
        let mut result = content.to_string();
        
        for (var_name, var_value) in &self.variables {
            let pattern = format!("{{{}}}", var_name);
            result = result.replace(&pattern, var_value);
        }
        
        Ok(result)
    }

    /// Procesa las etiquetas de marcado
    fn parse_markup_tags(&self, content: &str) -> Result<Vec<TemplateElement>, Box<dyn Error>> {
        debug!("Entering TemplateParser::parse_markup_tags");
        self.parse_markup_tags_ctx(
            content,
            None,
            None,
            Color3270::Default,
            false,
            false,
            false,
        )
    }

    /// Procesa las etiquetas de marcado con contexto de posición y atributos
    fn parse_markup_tags_ctx(
        &self,
        content: &str,
        mut current_row: Option<u16>,
        mut current_col: Option<u16>,
        current_color: Color3270,
        bright: bool,
        blink: bool,
        underline: bool,
    ) -> Result<Vec<TemplateElement>, Box<dyn Error>> {
        debug!("Entering TemplateParser::parse_markup_tags_ctx");
        let mut elements = Vec::new();

        // Regex para directivas (sin grupos duplicados ni backreferences)
        let re_directive = Regex::new(r#"<(pos|col)(?::([^>]+))?>"#)?;

        // Define enum to unify directive/container match
        enum NextTagMatch<'a> {
            Directive(regex::Captures<'a>),
            Container(&'a str, &'a str, usize, usize),
        }

        let mut pos = 0;
        let len = content.len();
        while pos < len {
            let rest_str = &content[pos..];
            // Buscar el primer match de directiva
            let dir_match = re_directive.captures(rest_str).and_then(|cap| {
                let m = cap.get(0).unwrap();
                Some((m.start(), m.end(), cap))
            });

            // Buscar el primer tag contenedor manualmente (soporta anidamiento)
            let mut cont_start = None;
            let mut cont_tag = "";
            let mut cont_params = "";
            let mut cont_open_len = 0;
            let mut cont_content_start = 0;
            if let Some(open) = rest_str.find('<') {
                let after = &rest_str[open..];
                if let Some(cap) = Regex::new(r#"^<(color|bright|blink|underline|field)(?::([^>]+))?>"#)
                    .unwrap()
                    .captures(after)
                {
                    cont_tag = cap.get(1).unwrap().as_str();
                    cont_params = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                    cont_start = Some(open);
                    cont_open_len = cap.get(0).unwrap().end();
                    cont_content_start = open + cont_open_len;
                }
            }

            let mut cont_match = None;
            if let Some(start) = cont_start {
                let tag = cont_tag;
                let mut depth = 1;
                let mut search_pos = cont_content_start;
                while depth > 0 {
                    if let Some(next_open) = rest_str[search_pos..].find(&format!("<{}", tag)) {
                        let next_open_abs = search_pos + next_open;
                        let next_close = rest_str[search_pos..].find(&format!("</{}>", tag));
                        match next_close {
                            Some(close_rel) if next_open < close_rel => {
                                depth += 1;
                                search_pos = next_open_abs + 1;
                            }
                            Some(close_rel) => {
                                depth -= 1;
                                if depth == 0 {
                                    let close_abs = search_pos + close_rel;
                                    let end = close_abs + tag.len() + 3; // </tag>
                                    cont_match = Some((start, end, tag, cont_params, cont_content_start, close_abs));
                                    break;
                                } else {
                                    search_pos = search_pos + close_rel + 1;
                                }
                            }
                            None => break,
                        }
                    } else if let Some(close_rel) = rest_str[search_pos..].find(&format!("</{}>", tag)) {
                        depth -= 1;
                        if depth == 0 {
                            let close_abs = search_pos + close_rel;
                            let end = close_abs + tag.len() + 3; // </tag>
                            cont_match = Some((start, end, tag, cont_params, cont_content_start, close_abs));
                            break;
                        } else {
                            search_pos = search_pos + close_rel + 1;
                        }
                    } else {
                        break;
                    }
                }
            }

            // Determinar cuál ocurre antes
            let (next_type, start, end, cap) = match (dir_match, cont_match) {
                (Some((ds, de, dcap)), Some((cs, ce, tag, params, content_start, content_end))) => {
                    if ds <= cs {
                        ("dir", ds, de, NextTagMatch::Directive(dcap))
                    } else {
                        ("cont_manual", cs, ce, NextTagMatch::Container(tag, params, content_start, content_end))
                    }
                }
                (Some((ds, de, dcap)), None) => ("dir", ds, de, NextTagMatch::Directive(dcap)),
                (None, Some((cs, ce, tag, params, content_start, content_end))) => {
                    ("cont_manual", cs, ce, NextTagMatch::Container(tag, params, content_start, content_end))
                }
                (None, None) => {
                    let text_rest = &content[pos..];
                    if !text_rest.is_empty() {
                        trace!("Texto plano: {:?}", text_rest);
                        
                        // Log específico para caracteres problemáticos
                        if text_rest.contains('+') || text_rest.contains('|') || text_rest.contains('=') {
                            println!("🚨 PARSER: Creating text element with special chars: {:?} at pos ({:?},{:?})", 
                                    text_rest, current_row, current_col);
                        }
                        
                        elements.push(TemplateElement::Text {
                            content: text_rest.to_string(),
                            color: current_color,
                            row: current_row,
                            col: current_col,
                            bright,
                            blink,
                            underline,
                        });
                    }
                    break;
                }
            };

            // Si hay texto antes del tag, añadirlo como texto plano
            if start > 0 {
                let text_before = &rest_str[..start];
                if !text_before.is_empty() {
                    trace!("Texto antes de tag: {:?}", text_before);
                    
                    // Log específico para caracteres problemáticos
                    if text_before.contains('+') || text_before.contains('|') || text_before.contains('=') {
                        println!("🚨 PARSER: Creating text element (before tag) with special chars: {:?} at pos ({:?},{:?})", 
                                text_before, current_row, current_col);
                    }
                    
                    elements.push(TemplateElement::Text {
                        content: text_before.to_string(),
                        color: current_color,
                        row: current_row,
                        col: current_col,
                        bright,
                        blink,
                        underline,
                    });
                    
                    // Advance column position for next element
                    let (new_row, new_col) = Self::advance_position(current_row, current_col, text_before.len() as u16);
                    current_row = new_row;
                    current_col = new_col;
                }
            }

            match next_type {
                "dir" => {
                    if let NextTagMatch::Directive(cap) = cap {
                        let dir = cap.get(1).unwrap().as_str();
                        let params = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                        trace!("Directiva <{}:{}> en pos {}", dir, params, pos + start);
                        match dir {
                            "pos" => {
                                let (row, col) = self.parse_position(params)?;
                                current_row = Some(row);
                                current_col = Some(col);
                            }
                            "col" => {
                                let col = params.parse::<u16>().map_err(|_| {
                                    TemplateError::InvalidPosition(format!("Columna inválida: {}", params))
                                })?;
                                current_col = Some(col);
                            }
                            _ => {
                                return Err(Box::new(TemplateError::UnmatchedTag(dir.to_string())));
                            }
                        }
                    }
                }
                "cont_manual" => {
                    if let NextTagMatch::Container(tag, params, content_start, content_end) = cap {
                        trace!("Abre <{}:{}> en pos {}", tag, params, pos + start);
                        let content_text = &rest_str[content_start..content_end];
                        match tag {
                            "color" => {
                                let color = Color3270::from_str(params)?;
                                if !content_text.is_empty() {
                                    let inner = self.parse_markup_tags_ctx(
                                        content_text,
                                        current_row,
                                        current_col,
                                        color,
                                        bright,
                                        blink,
                                        underline,
                                    )?;
                                    elements.extend(inner);
                                    
                                    // Advance column position based on actual text length (strip markup)
                                    let text_length = Self::calculate_text_length(content_text);
                                    let (new_row, new_col) = Self::advance_position(current_row, current_col, text_length as u16);
                                    current_row = new_row;
                                    current_col = new_col;
                                }
                            }
                            "bright" => {
                                if !content_text.is_empty() {
                                    let inner = self.parse_markup_tags_ctx(
                                        content_text,
                                        current_row,
                                        current_col,
                                        current_color,
                                        true,
                                        blink,
                                        underline,
                                    )?;
                                    elements.extend(inner);
                                    
                                    // Advance column position based on actual text length (strip markup)
                                    let text_length = Self::calculate_text_length(content_text);
                                    let (new_row, new_col) = Self::advance_position(current_row, current_col, text_length as u16);
                                    current_row = new_row;
                                    current_col = new_col;
                                }
                            }
                            "blink" => {
                                if !content_text.is_empty() {
                                    let inner = self.parse_markup_tags_ctx(
                                        content_text,
                                        current_row,
                                        current_col,
                                        current_color,
                                        bright,
                                        true,
                                        underline,
                                    )?;
                                    elements.extend(inner);
                                    
                                    // Advance column position based on actual text length (strip markup)
                                    let text_length = Self::calculate_text_length(content_text);
                                    let (new_row, new_col) = Self::advance_position(current_row, current_col, text_length as u16);
                                    current_row = new_row;
                                    current_col = new_col;
                                }
                            }
                            "underline" => {
                                if !content_text.is_empty() {
                                    let inner = self.parse_markup_tags_ctx(
                                        content_text,
                                        current_row,
                                        current_col,
                                        current_color,
                                        bright,
                                        blink,
                                        true,
                                    )?;
                                    elements.extend(inner);
                                    
                                    // Advance column position based on actual text length (strip markup)
                                    let text_length = Self::calculate_text_length(content_text);
                                    let (new_row, new_col) = Self::advance_position(current_row, current_col, text_length as u16);
                                    current_row = new_row;
                                    current_col = new_col;
                                }
                            }
                            "field" => {
                                let field_attrs = self.parse_field(params, content_text)?;
                                elements.push(TemplateElement::Field {
                                    attributes: field_attrs,
                                    row: current_row,
                                    col: current_col,
                                });
                            }
                            _ => {
                                return Err(Box::new(TemplateError::UnmatchedTag(tag.to_string())));
                            }
                        }
                        trace!("Cierra </{}> en pos {}", tag, pos + end - 1);
                    }
                }
                _ => unreachable!(),
            }

            pos += end;
        }

        Ok(elements)
    }

    /// Parsea una especificación de posición "fila,columna"
    fn parse_position(&self, pos_str: &str) -> Result<(u16, u16), TemplateError> {
        debug!("Entering TemplateParser::parse_position");
        let parts: Vec<&str> = pos_str.split(',').collect();
        if parts.len() != 2 {
            return Err(TemplateError::InvalidPosition(
                format!("Formato de posición inválido: {}. Use 'fila,columna'", pos_str)
            ));
        }
        
        let row = parts[0].trim().parse::<u16>().map_err(|_| {
            TemplateError::InvalidPosition(format!("Fila inválida: {}", parts[0]))
        })?;
        
        let col = parts[1].trim().parse::<u16>().map_err(|_| {
            TemplateError::InvalidPosition(format!("Columna inválida: {}", parts[1]))
        })?;
        
        Ok((row, col))
    }

    /// Parsea la definición de un campo
    fn parse_field(&self, params: &str, default_value: &str) -> Result<FieldAttributes, TemplateError> {
        debug!("Entering TemplateParser::parse_field");
        let parts: Vec<&str> = params.splitn(2, ',').collect();
        let field_name = parts[0].to_string();
        
        let mut field_attrs = FieldAttributes::new(field_name);
        field_attrs.default_value = default_value.to_string();
        
        if parts.len() > 1 {
            let attributes = FieldAttributes::parse_attributes(parts[1])?;
            field_attrs.apply_attributes(attributes)?;
        }
        
        Ok(field_attrs)
    }

    /// Valida que las posiciones estén dentro de los límites de la pantalla
    fn validate_elements(&self, elements: &[TemplateElement]) -> Result<(), TemplateError> {
        debug!("Entering TemplateParser::validate_elements");
        for element in elements {
            match element {
                TemplateElement::Text { row, col, .. } | TemplateElement::Field { row, col, .. } => {
                    if let Some(r) = row {
                        if *r < 1 || *r > 24 {
                            return Err(TemplateError::PositionOutOfBounds(*r, col.unwrap_or(1)));
                        }
                    }
                    if let Some(c) = col {
                        if *c < 1 || *c > 80 {
                            return Err(TemplateError::PositionOutOfBounds(row.unwrap_or(1), *c));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl Default for TemplateParser {
    fn default() -> Self {
        debug!("Entering TemplateParser::default");
        Self::new()
    }
}
