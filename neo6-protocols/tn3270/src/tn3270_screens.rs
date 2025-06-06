use crate::{Codec, TemplateParser, TemplateElement, FieldManager, ScreenField};
use crate::template_parser::Color3270;
use std::error::Error;
use std::fs;
use std::path::Path;
use chrono::{Local, DateTime};

/// Atributos de campo 3270 con soporte para colores
#[derive(Debug, Clone)]
pub struct FieldAttribute {
    pub protected: bool,
    pub numeric: bool,
    pub display: bool,
    pub intensity: u8,      // 0=Normal, 1=High, 2=Zero (invisible)
    pub color: Color3270,
}

impl FieldAttribute {
    /// Crea un nuevo atributo de campo básico
    pub fn new() -> Self {
        FieldAttribute {
            protected: true,
            numeric: false,
            display: true,
            intensity: 0,
            color: Color3270::Default,
        }
    }
    
    /// Crea un atributo protegido con color
    pub fn protected_with_color(color: Color3270) -> Self {
        FieldAttribute {
            protected: true,
            numeric: false,
            display: true,
            intensity: 0,
            color,
        }
    }
    
    /// Crea un atributo desprotegido (entrada de usuario)
    pub fn unprotected() -> Self {
        FieldAttribute {
            protected: false,
            numeric: false,
            display: true,
            intensity: 0,
            color: Color3270::Default,
        }
    }
    
    /// Convierte el atributo a byte 3270
    pub fn to_byte(&self) -> u8 {
        let mut attr = 0x00;
        
        if self.protected { attr |= 0x20; }
        if self.numeric { attr |= 0x10; }
        if !self.display { attr |= 0x0C; }
        
        match self.intensity {
            1 => attr |= 0x08,  // Alta intensidad
            2 => attr |= 0x0C,  // Invisible
            _ => {}             // Normal
        }
        
        attr
    }
}

/// Estructura para manejar pantallas 3270
#[derive(Debug)]
pub struct ScreenManager {
    pub screen_buffer: Vec<u8>,
    pub screen_sent: bool,
    codec: Codec,
}

impl ScreenManager {
    pub fn new() -> Self {
        ScreenManager {
            screen_buffer: vec![0x00; 1920], // 80x24 por defecto
            screen_sent: false,
            codec: Codec::new(),
        }
    }

    /// Función auxiliar para calcular dirección de buffer 3270 (14-bit addressing compatible)
    /// Para 80x24: dirección = (fila * 80) + columna  
    /// Codificación estándar 3270: high = (addr >> 6) & 0x3F | 0x40, low = addr & 0x3F | 0x40
    pub fn encode_buffer_addr(row: u16, col: u16) -> (u8, u8) {
        let addr = (row * 80) + col;
        let high = ((addr >> 6) & 0x3F) | 0x40;
        let low = (addr & 0x3F) | 0x40;
        (high as u8, low as u8)
    }
    
    /// Agrega texto con color al stream de datos usando Set Attribute
    fn add_colored_text(&self, screen_data: &mut Vec<u8>, text: &str, color: Color3270, bright: bool, blink: bool, underline: bool) {
        // Aplicar atributos de color/formato si es necesario
        if !matches!(color, Color3270::Default) || bright || blink || underline {
            screen_data.push(0x28); // SA (Set Attribute)
            screen_data.push(0x00); // All character attributes
            
            let mut attr_byte = 0x00;
            
            // Mapear color a atributo 3270
            match color {
                Color3270::Blue => attr_byte |= 0x04,
                Color3270::Red => attr_byte |= 0x02,
                Color3270::Pink => attr_byte |= 0x06,
                Color3270::Green => attr_byte |= 0x01,
                Color3270::Turquoise => attr_byte |= 0x05,
                Color3270::Yellow => attr_byte |= 0x03,
                Color3270::White => attr_byte |= 0x07,
                Color3270::Default => {}
            }
            
            // Agregar intensidad/efectos
            if bright { attr_byte |= 0x08; }
            if blink { attr_byte |= 0x10; }
            if underline { attr_byte |= 0x20; }
            
            screen_data.push(attr_byte);
        }
        
        // Convertir texto a EBCDIC y agregar
        let text_ebcdic = self.codec.from_host(text.as_bytes());
        screen_data.extend_from_slice(&text_ebcdic);
        
        // Resetear atributos después del texto si se aplicaron atributos especiales
        if !matches!(color, Color3270::Default) || bright || blink || underline {
            screen_data.push(0x28); // SA (Set Attribute)
            screen_data.push(0x00); // All character attributes
            screen_data.push(0x00); // Reset to default
        }
    }
    
    /// Posiciona el cursor y agrega texto con color
    fn add_positioned_colored_text(&self, screen_data: &mut Vec<u8>, row: u16, col: u16, text: &str, color: Color3270, bright: bool, blink: bool, underline: bool) {
        // SBA para posicionar
        let (high, low) = Self::encode_buffer_addr(row, col);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        
        // Agregar texto con color
        self.add_colored_text(screen_data, text, color, bright, blink, underline);
    }
    
    /// Método legacy mantenido para compatibilidad - usa add_colored_text internamente
    fn add_colored_field(&self, screen_data: &mut Vec<u8>, text: &str, attr: &FieldAttribute) {
        self.add_colored_text(screen_data, text, attr.color, attr.intensity == 1, false, false);
    }
    
    /// Método legacy mantenido para compatibilidad - usa add_positioned_colored_text internamente
    fn add_positioned_colored_field(&self, screen_data: &mut Vec<u8>, row: u16, col: u16, text: &str, attr: &FieldAttribute) {
        self.add_positioned_colored_text(screen_data, row, col, text, attr.color, attr.intensity == 1, false, false);
    }

    /// Genera la pantalla de bienvenida de NEO6 usando el nuevo parser de templates
    pub fn generate_welcome_screen(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut screen_data = Vec::new();
        
        // Comando Erase/Write y WCC
        screen_data.push(0xF5); // Comando: Erase/Write
        screen_data.push(0xC0); // WCC: Reset Keyboard + Reset MDT + Unlock Keyboard

        // Leer la plantilla y procesarla con el nuevo parser
        let template_content = self.load_welcome_template()?;
        let parser = TemplateParser::new();
        let elements = parser.parse_template(&template_content)?;

        // Crear gestor de campos para manejar campos de entrada
        let mut field_manager = FieldManager::new();
        
        // Campo protegido inicial que cubre toda la pantalla
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0x20); // Protegido
        
        // Procesar cada elemento de la plantilla
        for element in &elements {
            match element {
                TemplateElement::Text { content, color, row, col, bright, blink, underline } => {
                    // Posicionar si es necesario
                    if let (Some(r), Some(c)) = (row, col) {
                        let (high, low) = Self::encode_buffer_addr(*r - 1, *c - 1); // Convertir a 0-based
                        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
                    }
                    
                    // Simplemente enviar el texto sin atributos de color por ahora
                    // para asegurar que el cliente puede mostrar el contenido básico
                    let text_ebcdic = self.codec.from_host(content.as_bytes());
                    screen_data.extend_from_slice(&text_ebcdic);
                }
                TemplateElement::Field { attributes, row, col } => {
                    // Crear el campo y agregarlo al gestor
                    if let (Some(r), Some(c)) = (row, col) {
                        // Posicionar el campo
                        let (high, low) = Self::encode_buffer_addr(*r - 1, *c - 1); // Convertir a 0-based
                        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
                        
                        // Crear Start Field apropiado para el campo de entrada
                        screen_data.push(0x1D); // SF (Start Field)
                        
                        let field_attr = if attributes.protected {
                            0x20 // Protegido
                        } else {
                            0x00 // Desprotegido (entrada de usuario)
                        };
                        screen_data.push(field_attr);
                        
                        // Agregar valor por defecto si existe
                        if !attributes.default_value.is_empty() {
                            let default_ebcdic = self.codec.from_host(attributes.default_value.as_bytes());
                            screen_data.extend_from_slice(&default_ebcdic);
                        }
                        
                        // Crear ScreenField para el gestor
                        let screen_field = ScreenField::new(
                            attributes.name.clone(),
                            *r,
                            *c,
                            attributes.clone(),
                        );
                        
                        // Guardar el campo en el gestor
                        field_manager.add_field(screen_field)?;
                    }
                }
            }
        }
        
        // Posicionar cursor en el primer campo de entrada (si existe)
        if let Some(first_input_field) = field_manager.get_fields_by_position()
            .iter()
            .find(|f| !f.attributes.protected) {
            let (high, low) = Self::encode_buffer_addr(
                first_input_field.row - 1, 
                first_input_field.col - 1
            );
            screen_data.extend_from_slice(&[0x11, high, low]); // SBA
            screen_data.push(0x13); // IC (Insert Cursor)
        }

        // Guardar en el buffer de pantalla
        self.screen_buffer = screen_data.clone();
        
        // Imprimir información de depuración simplificada
        println!("[DEBUG] Pantalla generada sin colores para compatibilidad:");
        println!("[DEBUG] Total elementos procesados: {}", elements.len());
        let stats = field_manager.get_field_stats();
        println!("[DEBUG] Campos de entrada: {}", stats.total_fields);
        println!("[DEBUG] Bytes enviados: {}", screen_data.len());
        
        // Mostrar una muestra de los primeros bytes para depuración
        println!("[DEBUG] Primeros 50 bytes (hex): {:02X?}", &screen_data[..std::cmp::min(50, screen_data.len())]);
        
        Ok(screen_data)
    }

    /// Calcula el byte de atributo basado en los flags de formato
    fn calculate_attribute_byte(&self, bright: bool, blink: bool, underline: bool) -> u8 {
        let mut attr = 0x20; // Protegido por defecto
        
        if bright {
            attr |= 0x08; // Alta intensidad
        }
        if blink {
            attr |= 0x04; // Parpadeo
        }
        if underline {
            attr |= 0x01; // Subrayado
        }
        
        attr
    }

    /// Agrega atributos de color al stream de datos
    fn add_color_attribute(&self, screen_data: &mut Vec<u8>, color: Color3270) {
        screen_data.push(0x29); // SFE (Start Field Extended)
        screen_data.push(0x02); // Número de pares atributo-valor
        screen_data.push(0xC0); // Extended highlighting attribute
        screen_data.push(0x00); // Valor por defecto
        screen_data.push(0x42); // Foreground color attribute
        screen_data.push(color as u8); // Color value
    }

    /// Genera una pantalla de demostración de colores 3270
    pub fn generate_color_demo_screen(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut screen_data = Vec::new();
        
        // Comando Erase/Write y WCC
        screen_data.push(0xF5); // Comando: Erase/Write
        screen_data.push(0xC0); // WCC: Reset Keyboard + Reset MDT + Unlock Keyboard

        // Título principal
        let title_attr = FieldAttribute::protected_with_color(Color3270::Yellow);
        self.add_colored_field(&mut screen_data, "DEMOSTRACION DE COLORES TN3270 - NEO6", &title_attr);

        // Línea separadora
        let separator_attr = FieldAttribute::protected_with_color(Color3270::Blue);
        self.add_positioned_colored_field(&mut screen_data, 2, 0, "=".repeat(78).as_str(), &separator_attr);

        // Demostración de cada color
        let colors = [
            (Color3270::Default, "Color por defecto (Default)"),
            (Color3270::Blue, "Texto en azul (Blue)"),
            (Color3270::Red, "Texto en rojo (Red)"),
            (Color3270::Pink, "Texto en rosa/magenta (Pink)"),
            (Color3270::Green, "Texto en verde (Green)"),
            (Color3270::Turquoise, "Texto en turquesa/cyan (Turquoise)"),
            (Color3270::Yellow, "Texto en amarillo (Yellow)"),
            (Color3270::White, "Texto en blanco (White)"),
        ];

        for (i, (color, description)) in colors.iter().enumerate() {
            let row = 4 + i as u16;
            let attr = FieldAttribute::protected_with_color(*color);
            self.add_positioned_colored_field(&mut screen_data, row, 2, description, &attr);
        }

        // Texto con alta intensidad
        let mut high_intensity_attr = FieldAttribute::protected_with_color(Color3270::White);
        high_intensity_attr.intensity = 1;
        self.add_positioned_colored_field(&mut screen_data, 13, 2, "Texto con ALTA INTENSIDAD", &high_intensity_attr);

        // Información adicional
        let info_attr = FieldAttribute::protected_with_color(Color3270::Turquoise);
        self.add_positioned_colored_field(&mut screen_data, 15, 0, "Nota: Los colores dependen de las capacidades del terminal.", &info_attr);
        self.add_positioned_colored_field(&mut screen_data, 16, 0, "Los terminales modernos (c3270, x3270) soportan colores completos.", &info_attr);

        // Prompt
        let prompt_attr = FieldAttribute::protected_with_color(Color3270::Pink);
        self.add_positioned_colored_field(&mut screen_data, 20, 0, "Presione ENTER para continuar: ", &prompt_attr);

        // Campo de entrada
        let (high, low) = Self::encode_buffer_addr(20, 31);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        
        let input_attr = FieldAttribute::unprotected();
        screen_data.push(0x1D); // SF (Start Field) 
        screen_data.push(input_attr.to_byte());
        screen_data.push(0x13); // IC (Insert Cursor)

        self.screen_buffer = screen_data.clone();
        Ok(screen_data)
    }
    fn load_welcome_template(&self) -> Result<String, Box<dyn Error>> {
        // Intentar cargar primero la plantilla con markup
        let markup_template_path = Path::new("screens/welcome_markup.txt");
        
        let content = if markup_template_path.exists() {
            fs::read_to_string(markup_template_path)?
        } else {
            // Ruta absoluta desde el directorio del proyecto (markup template)
            let abs_markup_path = Path::new("/home/carlesabarca/MyProjects/NEO6/neo6-protocols/tn3270/screens/welcome_markup.txt");
            if abs_markup_path.exists() {
                fs::read_to_string(abs_markup_path)?
            } else {
                // Fallback a plantilla original sin markup
                let template_path = Path::new("screens/welcome.txt");
                if template_path.exists() {
                    fs::read_to_string(template_path)?
                } else {
                    let abs_path = Path::new("/home/carlesabarca/MyProjects/NEO6/neo6-protocols/tn3270/screens/welcome.txt");
                    if abs_path.exists() {
                        fs::read_to_string(abs_path)?
                    } else {
                        // Último fallback a una pantalla básica
                        return Ok(self.get_fallback_welcome_screen());
                    }
                }
            }
        };

        Ok(content)
    }

    /// Procesa las variables del template (reemplaza placeholders)
    fn process_template_variables(&self, mut content: String) -> Result<String, Box<dyn Error>> {
        // Obtener timestamp actual
        let now: DateTime<Local> = Local::now();
        let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();
        
        // Procesar línea por línea para mantener el formato de 80 caracteres
        let lines: Vec<String> = content
            .split('\n')  // Usar split en lugar de lines() para mantener control
            .map(|line| {
                let mut processed_line = line.to_string();
                
                // Reemplazar variables manteniendo el padding correcto
                if processed_line.contains("{timestamp}") {
                    // Calcular espacios necesarios para mantener 80 caracteres
                    let line_without_macro = processed_line.replace("{timestamp}", "");
                    let available_space = 80 - line_without_macro.len();
                    let padded_timestamp = if timestamp.len() <= available_space {
                        format!("{:<width$}", timestamp, width = available_space)
                    } else {
                        timestamp[..available_space].to_string()
                    };
                    processed_line = processed_line.replace("{timestamp}", &padded_timestamp);
                }
                
                if processed_line.contains("{terminal_type}") {
                    let terminal_type = "IBM-3278-2-E";
                    let line_without_macro = processed_line.replace("{terminal_type}", "");
                    let available_space = 80 - line_without_macro.len();
                    let padded_terminal = if terminal_type.len() <= available_space {
                        format!("{:<width$}", terminal_type, width = available_space)
                    } else {
                        terminal_type[..available_space].to_string()
                    };
                    processed_line = processed_line.replace("{terminal_type}", &padded_terminal);
                }
                
                processed_line
            })
            .collect();
        
        // Unir líneas usando \n pero será procesado posteriormente sin enviarlo al terminal
        content = lines.join("\n");
        
        // Limpiar caracteres que pueden causar problemas en EBCDIC
        content = Self::clean_for_ebcdic(content);
        
        // Normalizar líneas a exactamente 80 caracteres (sin \n en el resultado)
        content = Self::normalize_lines_to_80_chars(content);
        
        Ok(content)
    }

    /// Normaliza todas las líneas para que tengan exactamente 80 caracteres
    fn normalize_lines_to_80_chars(content: String) -> String {
        let mut lines: Vec<String> = content
            .split('\n')  // Usar split para tener control total
            .enumerate()
            .map(|(i, line)| {
                // Limpiar caracteres de control al final de cada línea
                let clean_line = line.trim_end_matches(['\r', '\n']);
                
                let normalized = if clean_line.len() > 80 {
                    // Truncar líneas demasiado largas
                    println!("[DEBUG] Línea {} truncada de {} a 80 caracteres", i + 1, clean_line.len());
                    clean_line[..80].to_string()
                } else if clean_line.len() < 80 {
                    // Rellenar líneas cortas con espacios
                    format!("{:80}", clean_line)
                } else {
                    // Línea ya tiene 80 caracteres
                    clean_line.to_string()
                };
                
                // Verificar que efectivamente tenga 80 caracteres
                if normalized.len() != 80 {
                    println!("[ERROR] Línea {} tiene {} caracteres en lugar de 80", i + 1, normalized.len());
                }
                
                normalized
            })
            .collect();

        // Asegurar que tengamos exactamente 24 líneas (tamaño estándar 3270)
        while lines.len() < 24 {
            lines.push(" ".repeat(80));
        }
        
        // Si hay más de 24 líneas, truncar
        if lines.len() > 24 {
            lines.truncate(24);
        }

        // Unir líneas CON \n para que generate_welcome_screen pueda procesarlas
        // El \n será eliminado en el procesamiento posterior
        let result = lines.join("\n");
        println!("[DEBUG] Pantalla generada con {} líneas, {} caracteres totales", lines.len(), result.len());
        result
    }

    /// Limpia el contenido para asegurar compatibilidad con EBCDIC
    fn clean_for_ebcdic(mut content: String) -> String {
        // Reemplazar caracteres problemáticos con equivalentes ASCII seguros
        content = content.replace("á", "a");
        content = content.replace("é", "e");
        content = content.replace("í", "i");
        content = content.replace("ó", "o");
        content = content.replace("ú", "u");
        content = content.replace("ñ", "n");
        content = content.replace("ü", "u");
        content = content.replace("Á", "A");
        content = content.replace("É", "E");
        content = content.replace("Í", "I");
        content = content.replace("Ó", "O");
        content = content.replace("Ú", "U");
        content = content.replace("Ñ", "N");
        content = content.replace("Ü", "U");
        
        // Mantener solo caracteres ASCII imprimibles y espacios en blanco
        content.chars()
            .filter(|&c| c.is_ascii() && (c.is_ascii_graphic() || c.is_ascii_whitespace()))
            .collect()
    }

    /// Pantalla de fallback si no se encuentra el archivo
    fn get_fallback_welcome_screen(&self) -> String {
        let content = format!(
            "+============================================================================+\n\
             |                        SISTEMA NEO6 - FALLBACK MODE                       |\n\
             |                                                                            |\n\
             |  Bienvenido al Sistema NEO6                                               |\n\
             |  Archivo de plantilla no encontrado, usando modo basico.                 |\n\
             |                                                                            |\n\
             |  Estado: ACTIVO                                                           |\n\
             |  Hora: {}                                            |\n\
             +============================================================================+\n\
             \n\
             \n\
             COMANDO ===> ",
            Local::now().format("%Y-%m-%d %H:%M:%S")
        );
        
        // Normalizar líneas a 80 caracteres
        Self::normalize_lines_to_80_chars(content)
    }

    /// Genera una pantalla de menú con opciones
    pub fn generate_menu_screen(&mut self, title: &str, options: &[(&str, &str)]) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut screen_data = Vec::new();
        
        // Comando Erase/Write y WCC
        screen_data.push(0xF5); // Comando: Erase/Write
        screen_data.push(0xC0); // WCC: Reset Keyboard + Reset MDT + Unlock Keyboard

        // Título en la primera línea
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xF8); // Atributo: Protegido + Alta intensidad + Destacado
        
        let title_ebcdic = self.codec.from_host(title.as_bytes());
        screen_data.extend_from_slice(&title_ebcdic);

        // Línea separadora en la fila 2
        let (high, low) = Self::encode_buffer_addr(2, 0);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xF0); // Atributo: Protegido + Normal intensidad
        
        let separator = "=".repeat(60);
        let separator_ebcdic = self.codec.from_host(separator.as_bytes());
        screen_data.extend_from_slice(&separator_ebcdic);

        // Opciones del menú empezando en la fila 4
        for (i, (key, description)) in options.iter().enumerate() {
            let row = 4 + i as u16;
            let (high, low) = Self::encode_buffer_addr(row, 2);
            screen_data.extend_from_slice(&[0x11, high, low]); // SBA
            
            screen_data.push(0x1D); // SF (Start Field)
            screen_data.push(0xF0); // Atributo: Protegido + Normal intensidad
            
            let menu_item = format!("{}. {}", key, description);
            let menu_item_ebcdic = self.codec.from_host(menu_item.as_bytes());
            screen_data.extend_from_slice(&menu_item_ebcdic);
        }

        // Prompt de selección en la parte inferior
        let prompt_row = 20;
        let (high, low) = Self::encode_buffer_addr(prompt_row, 0);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xF0); // Atributo: Protegido + Alta intensidad
        
        let prompt_msg_ascii = "Seleccione una opción: ";
        let prompt_msg_ebcdic = self.codec.from_host(prompt_msg_ascii.as_bytes());
        screen_data.extend_from_slice(&prompt_msg_ebcdic);

        // Campo de entrada
        screen_data.push(0x1D); // SF (Start Field) 
        screen_data.push(0x60); // Atributo: Desprotegido + Normal intensidad
        screen_data.push(0x13); // IC (Insert Cursor)

        self.screen_buffer = screen_data.clone();
        Ok(screen_data)
    }

    /// Genera una pantalla de error con mensaje
    pub fn generate_error_screen(&mut self, error_msg: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut screen_data = Vec::new();
        
        // Comando Erase/Write y WCC
        screen_data.push(0xF5); // Comando: Erase/Write
        screen_data.push(0xC0); // WCC: Reset Keyboard + Reset MDT + Unlock Keyboard

        // Título de error
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xFC); // Atributo: Protegido + Alta intensidad + Parpadeo (error)
        
        let title_ascii = "*** ERROR ***";
        let title_ebcdic = self.codec.from_host(title_ascii.as_bytes());
        screen_data.extend_from_slice(&title_ebcdic);

        // Mensaje de error en la fila 3
        let (high, low) = Self::encode_buffer_addr(3, 0);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xF0); // Atributo: Protegido + Normal intensidad
        
        let error_ebcdic = self.codec.from_host(error_msg.as_bytes());
        screen_data.extend_from_slice(&error_ebcdic);

        // Instrucciones en la fila 5
        let (high, low) = Self::encode_buffer_addr(5, 0);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xF0); // Atributo: Protegido + Normal intensidad
        
        let instruction_ascii = "Presione ENTER para continuar o CLEAR para salir";
        let instruction_ebcdic = self.codec.from_host(instruction_ascii.as_bytes());
        screen_data.extend_from_slice(&instruction_ebcdic);

        // Campo invisible para capturar entrada
        let (high, low) = Self::encode_buffer_addr(20, 0);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        screen_data.push(0x1D); // SF (Start Field) 
        screen_data.push(0x6C); // Atributo: Desprotegido + No display
        screen_data.push(0x13); // IC (Insert Cursor)

        self.screen_buffer = screen_data.clone();
        Ok(screen_data)
    }

    /// Marca la pantalla como enviada
    pub fn mark_screen_sent(&mut self) {
        self.screen_sent = true;
    }

    /// Verifica si la pantalla ya fue enviada
    pub fn is_screen_sent(&self) -> bool {
        self.screen_sent
    }

    /// Resetea el estado de pantalla enviada
    pub fn reset_screen_sent(&mut self) {
        self.screen_sent = false;
    }

    /// Obtiene una copia del buffer de pantalla actual
    pub fn get_screen_buffer(&self) -> Vec<u8> {
        self.screen_buffer.clone()
    }

    /// Actualiza el codec para conversiones EBCDIC/ASCII
    pub fn set_codec(&mut self, codec: Codec) {
        self.codec = codec;
    }

    /// Genera una pantalla de estado del sistema
    pub fn generate_status_screen(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut screen_data = Vec::new();
        
        // Comando Erase/Write y WCC
        screen_data.push(0xF5); // Comando: Erase/Write
        screen_data.push(0xC0); // WCC: Reset Keyboard + Reset MDT + Unlock Keyboard

        // Título del estado
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xF8); // Atributo: Protegido + Alta intensidad + Destacado
        
        let title_ascii = "ESTADO DEL SISTEMA NEO6";
        let title_ebcdic = self.codec.from_host(title_ascii.as_bytes());
        screen_data.extend_from_slice(&title_ebcdic);

        // Información del sistema en fila 3
        let (high, low) = Self::encode_buffer_addr(3, 0);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xF0); // Atributo: Protegido + Normal intensidad
        
        let status_ascii = "Terminal: ACTIVO | Conexiones: 1 | Estado: LISTO";
        let status_ebcdic = self.codec.from_host(status_ascii.as_bytes());
        screen_data.extend_from_slice(&status_ebcdic);

        // Línea de comandos disponibles en fila 5
        let (high, low) = Self::encode_buffer_addr(5, 0);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xF0); // Atributo: Protegido + Normal intensidad
        
        let commands_ascii = "Comandos: MENU, STATUS, EXIT";
        let commands_ebcdic = self.codec.from_host(commands_ascii.as_bytes());
        screen_data.extend_from_slice(&commands_ebcdic);

        // Prompt en la parte inferior
        let (high, low) = Self::encode_buffer_addr(20, 0);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xF0); // Atributo: Protegido + Alta intensidad
        
        let prompt_ascii = "Comando: ";
        let prompt_ebcdic = self.codec.from_host(prompt_ascii.as_bytes());
        screen_data.extend_from_slice(&prompt_ebcdic);

        // Campo de entrada
        screen_data.push(0x1D); // SF (Start Field) 
        screen_data.push(0x60); // Atributo: Desprotegido + Normal intensidad
        screen_data.push(0x13); // IC (Insert Cursor)

        self.screen_buffer = screen_data.clone();
        Ok(screen_data)
    }

    /// Limpia el buffer de pantalla
    pub fn clear_screen(&mut self) {
        self.screen_buffer.clear();
        self.screen_sent = false;
    }

    /// Genera una pantalla de prueba muy básica para verificar conectividad
    pub fn generate_basic_test_screen(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut screen_data = Vec::new();
        
        // Comando Erase/Write y WCC
        screen_data.push(0xF5); // Comando: Erase/Write
        screen_data.push(0xC0); // WCC: Reset Keyboard + Reset MDT + Unlock Keyboard

        // Campo protegido inicial sin posicionamiento explícito
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0x20); // Protegido
        
        // Texto simple concatenado
        let text = "NEO6 FUNCIONA! Si ve esto, el protocolo TN3270 esta operativo.";
        let text_ebcdic = self.codec.from_host(text.as_bytes());
        screen_data.extend_from_slice(&text_ebcdic);

        // Crear campo de entrada al final
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0x00); // Desprotegido
        
        // Posicionar cursor
        screen_data.push(0x13); // IC (Insert Cursor)

        // Guardar en el buffer de pantalla
        self.screen_buffer = screen_data.clone();
        
        println!("[DEBUG] Pantalla ultra-simple generada:");
        println!("[DEBUG] Bytes enviados: {}", screen_data.len());
        println!("[DEBUG] Datos completos (hex): {:02X?}", screen_data);
        
        Ok(screen_data)
    }
}

impl Default for ScreenManager {
    fn default() -> Self {
        Self::new()
    }
}
