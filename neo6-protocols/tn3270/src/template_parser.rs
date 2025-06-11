// Template parser v2.0 para el lenguaje de marcado de pantallas TN3270 con sintaxis de brackets
// Soporte completo para la especificación v2.0 definida en README2.md
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
        match color_name.to_uppercase().as_str() {
            "DEFAULT" => Ok(Color3270::Default),
            "BLUE" => Ok(Color3270::Blue),
            "RED" => Ok(Color3270::Red),
            "PINK" | "MAGENTA" => Ok(Color3270::Pink),
            "GREEN" => Ok(Color3270::Green),
            "TURQUOISE" | "CYAN" => Ok(Color3270::Turquoise),
            "YELLOW" => Ok(Color3270::Yellow),
            "WHITE" => Ok(Color3270::White),
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
    
    /// Calcula la longitud real del texto eliminando tags de markup (v1.0 y v2.0)
    fn calculate_text_length(content: &str) -> usize {
        // Regex para eliminar tags v1.0 (<tag>) y v2.0 ([tag])
        let html_regex = Regex::new(r"<[^>]*>").unwrap();
        let bracket_regex = Regex::new(r"\[[^\]]*\]").unwrap();
        
        let without_html = html_regex.replace_all(content, "");
        let without_brackets = bracket_regex.replace_all(&without_html, "");
        without_brackets.len()
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

    /// Procesa las etiquetas de marcado v2.0 con sintaxis de brackets
    fn parse_markup_tags(&self, content: &str) -> Result<Vec<TemplateElement>, Box<dyn Error>> {
        debug!("Entering TemplateParser::parse_markup_tags v2.0");
        self.parse_bracket_markup(
            content,
            None,
            None,
            Color3270::Default,
            false,
            false,
            false,
        )
    }

    /// Procesa las etiquetas de marcado v2.0 con sintaxis de brackets  
    fn parse_bracket_markup(
        &self,
        content: &str,
        mut current_row: Option<u16>,
        mut current_col: Option<u16>,
        current_color: Color3270,
        bright: bool,
        blink: bool,
        underline: bool,
    ) -> Result<Vec<TemplateElement>, Box<dyn Error>> {
        debug!("Entering TemplateParser::parse_bracket_markup v2.0");
        let mut elements = Vec::new();

        // Expresiones regulares para sintaxis v2.0 con brackets
        let pos_xy_regex = Regex::new(r"^\[XY(\d+),(\d+)\]")?;
        let pos_x_regex = Regex::new(r"^\[X(\d+)\]")?;
        let pos_y_regex = Regex::new(r"^\[Y(\d+)\]")?;
        
        // Regex para tags de color y atributos
        let color_start_regex = Regex::new(r"^\[(BLUE|RED|PINK|GREEN|TURQUOISE|YELLOW|WHITE|DEFAULT)\]")?;
        let bright_start_regex = Regex::new(r"^\[BRIGHT\]")?;
        let blink_start_regex = Regex::new(r"^\[BLINK\]")?;
        let underline_start_regex = Regex::new(r"^\[UNDERLINE\]")?;
        
        // Regex para campos
        let field_start_regex = Regex::new(r"^\[FIELD ([^\]]+)\]")?;

        let mut pos = 0;
        
        while pos < content.len() {
            let remaining = &content[pos..];
            
            // Buscar el siguiente tag bracket
            if let Some(bracket_pos) = remaining.find('[') {
                // Si hay texto antes del bracket, procesarlo
                if bracket_pos > 0 {
                    let text_before = &remaining[..bracket_pos];
                    if !text_before.is_empty() {
                        trace!("Texto v2.0: {:?}", text_before);
                        elements.push(TemplateElement::Text {
                            content: text_before.to_string(),
                            color: current_color,
                            row: current_row,
                            col: current_col,
                            bright,
                            blink,
                            underline,
                        });
                        
                        // Avanzar posición del cursor
                        let (new_row, new_col) = Self::advance_position(current_row, current_col, text_before.len() as u16);
                        current_row = new_row;
                        current_col = new_col;
                    }
                }
                
                let from_bracket = &remaining[bracket_pos..];
                
                // Verificar qué tipo de tag es
                if let Some(cap) = pos_xy_regex.captures(from_bracket) {
                    // [XY5,10] - posición absoluta
                    let row = cap[1].parse::<u16>().map_err(|_| 
                        TemplateError::InvalidPosition(format!("Fila inválida: {}", &cap[1])))?;
                    let col = cap[2].parse::<u16>().map_err(|_| 
                        TemplateError::InvalidPosition(format!("Columna inválida: {}", &cap[2])))?;
                    
                    if row < 1 || row > 24 || col < 1 || col > 80 {
                        return Err(Box::new(TemplateError::PositionOutOfBounds(row, col)));
                    }
                    
                    current_row = Some(row);
                    current_col = Some(col);
                    pos += bracket_pos + cap.get(0).unwrap().len();
                    continue;
                    
                } else if let Some(cap) = pos_x_regex.captures(from_bracket) {
                    // [X10] - solo columna
                    let col = cap[1].parse::<u16>().map_err(|_| 
                        TemplateError::InvalidPosition(format!("Columna inválida: {}", &cap[1])))?;
                    
                    if col < 1 || col > 80 {
                        return Err(Box::new(TemplateError::PositionOutOfBounds(current_row.unwrap_or(1), col)));
                    }
                    
                    current_col = Some(col);
                    pos += bracket_pos + cap.get(0).unwrap().len();
                    continue;
                    
                } else if let Some(cap) = pos_y_regex.captures(from_bracket) {
                    // [Y5] - solo fila
                    let row = cap[1].parse::<u16>().map_err(|_| 
                        TemplateError::InvalidPosition(format!("Fila inválida: {}", &cap[1])))?;
                    
                    if row < 1 || row > 24 {
                        return Err(Box::new(TemplateError::PositionOutOfBounds(row, current_col.unwrap_or(1))));
                    }
                    
                    current_row = Some(row);
                    pos += bracket_pos + cap.get(0).unwrap().len();
                    continue;
                    
                } else if let Some(cap) = color_start_regex.captures(from_bracket) {
                    // [COLOR] - inicio de color
                    let color_name = &cap[1];
                    let color = Color3270::from_str(color_name)?;
                    let tag_len = cap.get(0).unwrap().len();
                    
                    // Buscar el tag de cierre correspondiente usando un algoritmo más robusto
                    let close_tag = format!("[/{}]", color_name);
                    let content_start = pos + bracket_pos + tag_len;
                    
                    // Buscar el cierre balanceado, considerando tags anidados
                    if let Some(close_pos) = self.find_balanced_closing_tag(&content[content_start..], color_name) {
                        let content_end = content_start + close_pos;
                        let inner_content = &content[content_start..content_end];
                        
                        // Procesar contenido interno recursivamente
                        let inner_elements = self.parse_bracket_markup(
                            inner_content,
                            current_row,
                            current_col,
                            color,
                            bright,
                            blink,
                            underline,
                        )?;
                        
                        elements.extend(inner_elements);
                        
                        // Avanzar posición
                        let text_length = Self::calculate_text_length(inner_content);
                        let (new_row, new_col) = Self::advance_position(current_row, current_col, text_length as u16);
                        current_row = new_row;
                        current_col = new_col;
                        
                        pos = content_end + close_tag.len();
                        continue;
                    } else {
                        return Err(Box::new(TemplateError::UnmatchedTag(format!("[{}]", color_name))));
                    }
                    
                } else if let Some(cap) = bright_start_regex.captures(from_bracket) {
                    // [BRIGHT] - inicio de brillo
                    let tag_len = cap.get(0).unwrap().len();
                    let content_start = pos + bracket_pos + tag_len;
                    
                    if let Some(close_pos) = content[content_start..].find("[/BRIGHT]") {
                        let content_end = content_start + close_pos;
                        let inner_content = &content[content_start..content_end];
                        
                        // Procesar contenido interno recursivamente
                        let inner_elements = self.parse_bracket_markup(
                            inner_content,
                            current_row,
                            current_col,
                            current_color,
                            true,
                            blink,
                            underline,
                        )?;
                        
                        elements.extend(inner_elements);
                        
                        // Avanzar posición
                        let text_length = Self::calculate_text_length(inner_content);
                        let (new_row, new_col) = Self::advance_position(current_row, current_col, text_length as u16);
                        current_row = new_row;
                        current_col = new_col;
                        
                        pos = content_end + 9; // "[/BRIGHT]".len()
                        continue;
                    } else {
                        return Err(Box::new(TemplateError::UnmatchedTag("[BRIGHT]".to_string())));
                    }
                    
                } else if let Some(cap) = blink_start_regex.captures(from_bracket) {
                    // [BLINK] - inicio de parpadeo
                    let tag_len = cap.get(0).unwrap().len();
                    let content_start = pos + bracket_pos + tag_len;
                    
                    if let Some(close_pos) = content[content_start..].find("[/BLINK]") {
                        let content_end = content_start + close_pos;
                        let inner_content = &content[content_start..content_end];
                        
                        // Procesar contenido interno recursivamente
                        let inner_elements = self.parse_bracket_markup(
                            inner_content,
                            current_row,
                            current_col,
                            current_color,
                            bright,
                            true,
                            underline,
                        )?;
                        
                        elements.extend(inner_elements);
                        
                        // Avanzar posición
                        let text_length = Self::calculate_text_length(inner_content);
                        let (new_row, new_col) = Self::advance_position(current_row, current_col, text_length as u16);
                        current_row = new_row;
                        current_col = new_col;
                        
                        pos = content_end + 8; // "[/BLINK]".len()
                        continue;
                    } else {
                        return Err(Box::new(TemplateError::UnmatchedTag("[BLINK]".to_string())));
                    }
                    
                } else if let Some(cap) = underline_start_regex.captures(from_bracket) {
                    // [UNDERLINE] - inicio de subrayado
                    let tag_len = cap.get(0).unwrap().len();
                    let content_start = pos + bracket_pos + tag_len;
                    
                    if let Some(close_pos) = content[content_start..].find("[/UNDERLINE]") {
                        let content_end = content_start + close_pos;
                        let inner_content = &content[content_start..content_end];
                        
                        // Procesar contenido interno recursivamente
                        let inner_elements = self.parse_bracket_markup(
                            inner_content,
                            current_row,
                            current_col,
                            current_color,
                            bright,
                            blink,
                            true,
                        )?;
                        
                        elements.extend(inner_elements);
                        
                        // Avanzar posición
                        let text_length = Self::calculate_text_length(inner_content);
                        let (new_row, new_col) = Self::advance_position(current_row, current_col, text_length as u16);
                        current_row = new_row;
                        current_col = new_col;
                        
                        pos = content_end + 12; // "[/UNDERLINE]".len()
                        continue;
                    } else {
                        return Err(Box::new(TemplateError::UnmatchedTag("[UNDERLINE]".to_string())));
                    }
                    
                } else if let Some(cap) = field_start_regex.captures(from_bracket) {
                    // [FIELD name,attrs] - campo
                    let field_spec = &cap[1];
                    let tag_len = cap.get(0).unwrap().len();
                    let content_start = pos + bracket_pos + tag_len;
                    
                    if let Some(close_pos) = content[content_start..].find("[/FIELD]") {
                        let content_end = content_start + close_pos;
                        let field_content = &content[content_start..content_end];
                        
                        // Parsear el campo
                        let field_attrs = self.parse_field_v2(field_spec, field_content)?;
                        
                        elements.push(TemplateElement::Field {
                            attributes: field_attrs,
                            row: current_row,
                            col: current_col,
                        });
                        
                        pos = content_end + 8; // "[/FIELD]".len()
                        continue;
                    } else {
                        return Err(Box::new(TemplateError::UnmatchedTag("[FIELD]".to_string())));
                    }
                    
                } else {
                    // No es un tag reconocido, tratarlo como texto
                    pos += bracket_pos + 1;
                    continue;
                }
            } else {
                // No hay más brackets, el resto es texto
                if !remaining.is_empty() {
                    trace!("Texto final v2.0: {:?}", remaining);
                    elements.push(TemplateElement::Text {
                        content: remaining.to_string(),
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
        }

        Ok(elements)
    }

    /// Parsea una especificación de posición "fila,columna"
    /// Parsea la definición de un campo v2.0 con sintaxis de brackets
    fn parse_field_v2(&self, field_spec: &str, default_value: &str) -> Result<FieldAttributes, TemplateError> {
        debug!("Entering TemplateParser::parse_field_v2");
        let parts: Vec<&str> = field_spec.splitn(2, ',').collect();
        let field_name = parts[0].trim().to_string();
        
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
    
    /// Busca el tag de cierre balanceado, manejando tags anidados del mismo tipo
    fn find_balanced_closing_tag(&self, content: &str, tag_name: &str) -> Option<usize> {
        let open_tag = format!("[{}]", tag_name);
        let close_tag = format!("[/{}]", tag_name);
        
        let mut depth = 1;
        let mut pos = 0;
        
        while pos < content.len() {
            if let Some(open_pos) = content[pos..].find(&open_tag) {
                let open_abs_pos = pos + open_pos;
                
                if let Some(close_pos) = content[pos..].find(&close_tag) {
                    let close_abs_pos = pos + close_pos;
                    
                    if open_abs_pos < close_abs_pos {
                        // Encontramos otro tag de apertura antes del cierre
                        depth += 1;
                        pos = open_abs_pos + open_tag.len();
                    } else {
                        // Encontramos un tag de cierre
                        depth -= 1;
                        if depth == 0 {
                            return Some(close_abs_pos);
                        }
                        pos = close_abs_pos + close_tag.len();
                    }
                } else {
                    // No hay más tags de cierre
                    return None;
                }
            } else if let Some(close_pos) = content[pos..].find(&close_tag) {
                // Solo hay tag de cierre, no más aperturas
                let close_abs_pos = pos + close_pos;
                depth -= 1;
                if depth == 0 {
                    return Some(close_abs_pos);
                }
                pos = close_abs_pos + close_tag.len();
            } else {
                // No hay más tags
                return None;
            }
        }
        
        None
    }
}

impl Default for TemplateParser {
    fn default() -> Self {
        debug!("Entering TemplateParser::default");
        Self::new()
    }
}
