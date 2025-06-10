use crate::{Codec, TemplateParser, TemplateElement, FieldManager, ScreenField};
use crate::template_parser::Color3270;
use std::error::Error;
use std::fs;
use std::path::Path;
use tracing::{debug, error, trace};

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
        debug!("Entering FieldAttribute::new");
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
        debug!("Entering FieldAttribute::protected_with_color");
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
        debug!("Entering FieldAttribute::unprotected");
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
        debug!("Entering FieldAttribute::to_byte");
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
        debug!("Entering ScreenManager::new");
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
        debug!("Entering ScreenManager::encode_buffer_addr");
        let addr = (row * 80) + col;
        let high = ((addr >> 6) & 0x3F) | 0x40;
        let low = (addr & 0x3F) | 0x40;
        (high as u8, low as u8)
    }
    
    /// Agrega texto con color al stream de datos usando Set Attribute
    fn add_colored_text(&self, screen_data: &mut Vec<u8>, text: &str, color: Color3270, bright: bool, blink: bool, underline: bool) {
        debug!("Entering ScreenManager::add_colored_text");
        debug!("🔍 add_colored_text ENTRY: text={:?} (len={}) color={:?} bright={} blink={} underline={}", 
               text, text.len(), color, bright, blink, underline);
        
        // Aplicar atributos de color/formato si es necesario
        if !matches!(color, Color3270::Default) || bright || blink || underline {
            screen_data.push(0x29); // SFE (Start Field Extended)
            let mut count = 0u8;
            let mut attrs = Vec::new();

            if !matches!(color, Color3270::Default) {
                count += 1;
                attrs.push(0x42); // Foreground color
                attrs.push(color as u8);
            }

            if bright || blink || underline {
                count += 1;
                attrs.push(0x41); // Extended highlighting
                let mut highlight = 0xF0; // Normal
                if bright {
                    highlight = 0xF8; // Intensify
                } else if blink {
                    highlight = 0xF1; // Blink
                } else if underline {
                    highlight = 0xF4; // Underscore
                }
                attrs.push(highlight);
            }

            screen_data.push(count);
            screen_data.extend_from_slice(&attrs);
        }
        
        // Convertir texto a EBCDIC y agregar
        debug!("🔍 BEFORE EBCDIC CONVERSION: text={:?} (len={})", text, text.len());
        let text_bytes = text.as_bytes();
        debug!("🔍 TEXT AS BYTES: {:?} (len={})", text_bytes, text_bytes.len());
        
        // Log específico para caracteres problemáticos
        if text.contains('+') || text.contains('|') {
            println!("🚨 CONVERTING SPECIAL CHARS: text={:?}", text);
            for (i, &byte) in text_bytes.iter().enumerate() {
                println!("  [{}] ASCII byte: 0x{:02X} ({})", i, byte, byte as char);
            }
        }
        
        let text_ebcdic = self.codec.to_host(text_bytes);
        debug!("🔍 AFTER EBCDIC CONVERSION: {} bytes", text_ebcdic.len());
        
        // Log específico para la conversión EBCDIC de caracteres problemáticos
        if text.contains('+') || text.contains('|') {
            println!("🚨 EBCDIC CONVERSION RESULT:");
            for (i, &byte) in text_ebcdic.iter().enumerate() {
                println!("  [{}] EBCDIC byte: 0x{:02X}", i, byte);
            }
        }
        
        screen_data.extend_from_slice(&text_ebcdic);
        debug!("🔍 SCREEN DATA AFTER ADDING TEXT: {} total bytes", screen_data.len());
        
        // NO resetear atributos inmediatamente después del texto
        // Los atributos se mantendrán hasta el próximo SFE/SF
        // Esto evita que se corten caracteres en terminales TN3270 reales
    }
    
    /// Posiciona el cursor y agrega texto con color usando solo SBA y atributos simples
    fn add_positioned_colored_text(&self, screen_data: &mut Vec<u8>, row: u16, col: u16, text: &str, color: Color3270, bright: bool, blink: bool, underline: bool) {
        debug!("Entering ScreenManager::add_positioned_colored_text");
        debug!("🔍 add_positioned_colored_text ENTRY: row={} col={} text={:?} (len={})", row, col, text, text.len());
        
        // SBA para posicionar
        let (high, low) = Self::encode_buffer_addr(row, col);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        debug!("🔍 SBA added: [0x11, 0x{:02X}, 0x{:02X}]", high, low);
        
        // Aplicar atributos usando Set Attribute (SA) si es necesario
        if !matches!(color, Color3270::Default) {
            screen_data.push(0x28); // SA order
            screen_data.push(0x42); // XA_FOREGROUND
            screen_data.push(color as u8);
        }
        
        if bright || blink || underline {
            screen_data.push(0x28); // SA order  
            screen_data.push(0x41); // XA_HIGHLIGHTING
            let mut highlight = 0xF0; // Normal
            if bright {
                highlight = 0xF8; // Intensify
            } else if blink {
                highlight = 0xF1; // Blink
            } else if underline {
                highlight = 0xF4; // Underscore
            }
            screen_data.push(highlight);
        }
        
        // Convertir texto a EBCDIC y agregar directamente sin crear campos
        debug!("🔍 BEFORE EBCDIC CONVERSION: text={:?} (len={})", text, text.len());
        let text_bytes = text.as_bytes();
        
        // Log específico para caracteres problemáticos
        if text.contains('+') || text.contains('|') || text.contains('=') {
            println!("🚨 CONVERTING SPECIAL CHARS: text={:?}", text);
            for (i, &byte) in text_bytes.iter().enumerate() {
                println!("  [{}] ASCII byte: 0x{:02X} ({})", i, byte, byte as char);
            }
        }
        
        let text_ebcdic = self.codec.to_host(text_bytes);
        debug!("🔍 AFTER EBCDIC CONVERSION: {} bytes", text_ebcdic.len());
        
        // Log específico para la conversión EBCDIC de caracteres problemáticos
        if text.contains('+') || text.contains('|') || text.contains('=') {
            println!("🚨 EBCDIC CONVERSION RESULT:");
            for (i, &byte) in text_ebcdic.iter().enumerate() {
                println!("  [{}] EBCDIC byte: 0x{:02X}", i, byte);
            }
        }
        
        screen_data.extend_from_slice(&text_ebcdic);
        debug!("🔍 SCREEN DATA AFTER ADDING TEXT: {} total bytes", screen_data.len());
        debug!("🔍 add_positioned_colored_text COMPLETE");
    }
    
    /// Método legacy mantenido para compatibilidad - usa add_colored_text internamente
    fn add_colored_field(&self, screen_data: &mut Vec<u8>, text: &str, attr: &FieldAttribute) {
        debug!("Entering ScreenManager::add_colored_field");
        self.add_colored_text(screen_data, text, attr.color, attr.intensity == 1, false, false);
    }
    
    /// Método legacy mantenido para compatibilidad - usa add_positioned_colored_text internamente
    fn add_positioned_colored_field(&self, screen_data: &mut Vec<u8>, row: u16, col: u16, text: &str, attr: &FieldAttribute) {
        debug!("Entering ScreenManager::add_positioned_colored_field");
        self.add_positioned_colored_text(screen_data, row, col, text, attr.color, attr.intensity == 1, false, false);
    }

    /// Carga una plantilla desde `config/screens` buscando primero la versión con
    /// sufijo `_markup.txt` y luego la plana `.txt`
    fn load_template(&self, name: &str) -> Result<String, Box<dyn Error>> {
        debug!("Entering ScreenManager::load_template");
        
        let screens_dir = Path::new("config/screens");
        
        let markup_path = screens_dir.join(format!("{}_markup.txt", name));
        debug!("Looking for template at: {:?}", markup_path);
        if markup_path.exists() {
            debug!("Found markup template, reading file");
            return Ok(fs::read_to_string(markup_path)?);
        }

        let plain_path = screens_dir.join(format!("{}.txt", name));
        debug!("Looking for plain template at: {:?}", plain_path);
        if plain_path.exists() {
            debug!("Found plain template, reading file");
            return Ok(fs::read_to_string(plain_path)?);
        }

        Err(format!("Template '{}' not found in config/screens", name).into())
    }

    /// Genera una pantalla TN3270 a partir de cualquier plantilla de `config/screens`
    pub fn generate_tn3270_screen(&mut self, template_name: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        debug!("Entering ScreenManager::generate_tn3270_screen");

        let mut screen_data = Vec::new();
        screen_data.push(0xF5); // Erase/Write
        screen_data.push(0xC0); // WCC

        // Agregar un campo protegido inicial que cubra toda la pantalla (posición 0,0)
        // Esto establece el contexto base para todos los atributos subsiguientes
        let (high, low) = Self::encode_buffer_addr(0, 0);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA to (0,0)
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0x20); // FA_PROTECT - campo protegido para texto de solo lectura

        let template_content = self.load_template(template_name)?;
        let parser = TemplateParser::new();
        let elements = parser.parse_template(&template_content)?;

        // Debug: Print all parsed TemplateElement::Text elements
        println!("\n🔍 DEBUG: Parsed TemplateElement::Text elements:");
        for (i, element) in elements.iter().enumerate() {
            if let TemplateElement::Text { content, row, col, color, bright, blink, underline } = element {
                println!("  [{}] Text: {:?} at pos ({:?}, {:?}) color={:?} bright={} blink={} underline={}", 
                         i, content, row, col, color, bright, blink, underline);
            }
        }
        println!("🔍 DEBUG: Total elements parsed: {}", elements.len());

        let mut field_manager = FieldManager::new();

        // Process elements first without adding initial protected field
        for element in &elements {
            match element {
                TemplateElement::Text { content, row, col, color, bright, blink, underline } => {
                    debug!("🔍 PROCESSING TEXT ELEMENT: content={:?} (len={}) row={:?} col={:?}", 
                           content, content.len(), row, col);
                    
                    // Log específico para caracteres problemáticos
                    if content.contains('+') || content.contains('|') {
                        println!("🚨 SPECIAL CHARS DETECTED: content={:?} at pos ({:?},{:?})", content, row, col);
                        for (i, ch) in content.chars().enumerate() {
                            println!("  [{}] char: '{}' (U+{:04X})", i, ch, ch as u32);
                        }
                    }
                    
                    if let (Some(r), Some(c)) = (row, col) {
                        // Templates use 1-indexed coordinates, tn3270 uses 0-indexed
                        debug!("🔍 CALLING add_positioned_colored_text with: row={} col={} text={:?} (len={})", 
                               r - 1, c - 1, content, content.len());
                        self.add_positioned_colored_text(&mut screen_data, *r - 1, *c - 1, content, *color, *bright, *blink, *underline);
                        debug!("🔍 COMPLETED add_positioned_colored_text");
                        println!("🔍 DEBUG: Added colored text: {:?} at template row {} col {} (0-indexed: {},{}) with color={:?} bright={} blink={} underline={}", 
                                content, r, c, r-1, c-1, color, bright, blink, underline);
                    } else {
                        // If no position specified, just add colored text at current position
                        debug!("🔍 CALLING add_colored_text with: text={:?} (len={})", content, content.len());
                        self.add_colored_text(&mut screen_data, content, *color, *bright, *blink, *underline);
                        debug!("🔍 COMPLETED add_colored_text");
                        println!("🔍 DEBUG: Added colored text at current position: {:?} with color={:?} bright={} blink={} underline={}", 
                                content, color, bright, blink, underline);
                    }
                }
                TemplateElement::Field { attributes, row, col } => {
                    if let (Some(r), Some(c)) = (row, col) {
                        let (high, low) = Self::encode_buffer_addr(*r - 1, *c - 1);
                        screen_data.extend_from_slice(&[0x11, high, low]);

                        screen_data.push(0x1D); // SF
                        let field_attr = if attributes.protected { 0x20 } else { 0x00 };
                        screen_data.push(field_attr);

                        if !attributes.default_value.is_empty() {
                            let def = self.codec.to_host(attributes.default_value.as_bytes());
                            screen_data.extend_from_slice(&def);
                        }

                        let screen_field = ScreenField::new(
                            attributes.name.clone(),
                            *r,
                            *c,
                            attributes.clone(),
                        );
                        field_manager.add_field(screen_field)?;
                    }
                }
            }
        }

        // Add initial protected field at end of screen to ensure proper field setup
        // Position it at the last position of the screen (row 24, col 80)
        let (high, low) = Self::encode_buffer_addr(23, 79); // 0-based: 23,79 = 24,80
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        screen_data.push(0x1D); // SF
        screen_data.push(0x20); // Protected

        // Position cursor at first input field if any
        if let Some(first_input_field) = field_manager
            .get_fields_by_position()
            .iter()
            .find(|f| !f.attributes.protected)
        {
            let (high, low) = Self::encode_buffer_addr(first_input_field.row - 1, first_input_field.col - 1);
            screen_data.extend_from_slice(&[0x11, high, low]);
            screen_data.push(0x13); // IC
        }

        self.screen_buffer = screen_data.clone();

        debug!("Generic screen generated from {}", template_name);
        debug!("Total elements: {}", elements.len());
        let stats = field_manager.get_field_stats();
        debug!("Fields: {}", stats.total_fields);
        debug!("Bytes: {}", screen_data.len());

        trace!("First bytes: {:02X?}", &screen_data[..std::cmp::min(50, screen_data.len())]);

        Ok(screen_data)
    }

    /// Agrega un campo protegido con color y atributos extendidos
    fn add_protected_field_with_color(
        &self,
        screen_data: &mut Vec<u8>,
        row: u16,
        col: u16,
        text: &str,
        color: Color3270,
        bright: bool,
        blink: bool,
        underline: bool,
    ) {
        let (high, low) = Self::encode_buffer_addr(row, col);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA

        screen_data.push(0x1D); // SF
        screen_data.push(0x20); // Protected

        // SFE para aplicar color y estilo
        screen_data.push(0x29); // SFE
        let mut count = 1;
        if bright || blink || underline {
            count += 1;
        }
        screen_data.push(count); // Número de atributos

        // Color
        screen_data.push(0x42);
        screen_data.push(color as u8);

        if bright || blink || underline {
            screen_data.push(0xC0); // Extended highlighting
            let mut attr = 0x00;
            if bright { attr |= 0x08; }
            if blink { attr |= 0x10; }
            if underline { attr |= 0x20; }
            screen_data.push(attr);
        }

        let text_ebcdic = self.codec.to_host(text.as_bytes());
        screen_data.extend_from_slice(&text_ebcdic);
    }

    /// Genera la pantalla de bienvenida de NEO6 usando el nuevo parser de templates
    pub fn generate_welcome_screen(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
        debug!("Entering ScreenManager::generate_welcome_screen");
        // Use the new generic template generation method with "welcome" template
        self.generate_tn3270_screen("welcome")
    }    
    /// Calcula el byte de atributo basado en los flags de formato
    fn calculate_attribute_byte(&self, bright: bool, blink: bool, underline: bool) -> u8 {
        debug!("Entering ScreenManager::calculate_attribute_byte");
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
        debug!("Entering ScreenManager::add_color_attribute");
        screen_data.push(0x29); // SFE (Start Field Extended)
        screen_data.push(0x02); // Número de pares atributo-valor
        screen_data.push(0xC0); // Extended highlighting attribute
        screen_data.push(0x00); // Valor por defecto
        screen_data.push(0x42); // Foreground color attribute
        screen_data.push(color as u8); // Color value
    }

    /// Genera una pantalla de demostración de colores 3270
    pub fn generate_color_demo_screen(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
        debug!("Entering ScreenManager::generate_color_demo_screen");
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
    /// Genera una pantalla de menú con opciones
    pub fn generate_menu_screen(&mut self, title: &str, options: &[(&str, &str)]) -> Result<Vec<u8>, Box<dyn Error>> {
        debug!("Entering ScreenManager::generate_menu_screen");
        let mut screen_data = Vec::new();
        
        // Comando Erase/Write y WCC
        screen_data.push(0xF5); // Comando: Erase/Write
        screen_data.push(0xC0); // WCC: Reset Keyboard + Reset MDT + Unlock Keyboard

        // Título en la primera línea
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xF8); // Atributo: Protegido + Alta intensidad + Destacado
        
        let title_ebcdic = self.codec.to_host(title.as_bytes());
        screen_data.extend_from_slice(&title_ebcdic);

        // Línea separadora en la fila 2
        let (high, low) = Self::encode_buffer_addr(2, 0);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xF0); // Atributo: Protegido + Normal intensidad
        
        let separator = "=".repeat(60);
        let separator_ebcdic = self.codec.to_host(separator.as_bytes());
        screen_data.extend_from_slice(&separator_ebcdic);

        // Opciones del menú empezando en la fila 4
        for (i, (key, description)) in options.iter().enumerate() {
            let row = 4 + i as u16;
            let (high, low) = Self::encode_buffer_addr(row, 2);
            screen_data.extend_from_slice(&[0x11, high, low]); // SBA
            
            screen_data.push(0x1D); // SF (Start Field)
            screen_data.push(0xF0); // Atributo: Protegido + Normal intensidad
            
            let menu_item = format!("{}. {}", key, description);
            let menu_item_ebcdic = self.codec.to_host(menu_item.as_bytes());
            screen_data.extend_from_slice(&menu_item_ebcdic);
        }

        // Prompt de selección en la parte inferior
        let prompt_row = 20;
        let (high, low) = Self::encode_buffer_addr(prompt_row, 0);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xF0); // Atributo: Protegido + Alta intensidad
        
        let prompt_msg_ascii = "Seleccione una opción: ";
        let prompt_msg_ebcdic = self.codec.to_host(prompt_msg_ascii.as_bytes());
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
        debug!("Entering ScreenManager::generate_error_screen");
        let mut screen_data = Vec::new();
        
        // Comando Erase/Write y WCC
        screen_data.push(0xF5); // Comando: Erase/Write
        screen_data.push(0xC0); // WCC: Reset Keyboard + Reset MDT + Unlock Keyboard

        // Título de error
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xFC); // Atributo: Protegido + Alta intensidad + Parpadeo (error)
        
        let title_ascii = "*** ERROR ***";
        let title_ebcdic = self.codec.to_host(title_ascii.as_bytes());
        screen_data.extend_from_slice(&title_ebcdic);

        // Mensaje de error en la fila 3
        let (high, low) = Self::encode_buffer_addr(3, 0);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xF0); // Atributo: Protegido + Normal intensidad
        
        let error_ebcdic = self.codec.to_host(error_msg.as_bytes());
        screen_data.extend_from_slice(&error_ebcdic);

        // Instrucciones en la fila 5
        let (high, low) = Self::encode_buffer_addr(5, 0);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xF0); // Atributo: Protegido + Normal intensidad
        
        let instruction_ascii = "Presione ENTER para continuar o CLEAR para salir";
        let instruction_ebcdic = self.codec.to_host(instruction_ascii.as_bytes());
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
        debug!("Entering ScreenManager::mark_screen_sent");
        self.screen_sent = true;
    }

    /// Verifica si la pantalla ya fue enviada
    pub fn is_screen_sent(&self) -> bool {
        debug!("Entering ScreenManager::is_screen_sent");
        self.screen_sent
    }

    /// Resetea el estado de pantalla enviada
    pub fn reset_screen_sent(&mut self) {
        debug!("Entering ScreenManager::reset_screen_sent");
        self.screen_sent = false;
    }

    /// Obtiene una copia del buffer de pantalla actual
    pub fn get_screen_buffer(&self) -> Vec<u8> {
        debug!("Entering ScreenManager::get_screen_buffer");
        self.screen_buffer.clone()
    }

    /// Actualiza el codec para conversiones EBCDIC/ASCII
    pub fn set_codec(&mut self, codec: Codec) {
        debug!("Entering ScreenManager::set_codec");
        self.codec = codec;
    }

    /// Genera una pantalla de estado del sistema
    pub fn generate_status_screen(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
        debug!("Entering ScreenManager::generate_status_screen");
        let mut screen_data = Vec::new();
        
        // Comando Erase/Write y WCC
        screen_data.push(0xF5); // Comando: Erase/Write
        screen_data.push(0xC0); // WCC: Reset Keyboard + Reset MDT + Unlock Keyboard

        // Título del estado
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xF8); // Atributo: Protegido + Alta intensidad + Destacado
        
        let title_ascii = "ESTADO DEL SISTEMA NEO6";
        let title_ebcdic = self.codec.to_host(title_ascii.as_bytes());
        screen_data.extend_from_slice(&title_ebcdic);

        // Información del sistema en fila 3
        let (high, low) = Self::encode_buffer_addr(3, 0);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xF0); // Atributo: Protegido + Normal intensidad
        
        let status_ascii = "Terminal: ACTIVO | Conexiones: 1 | Estado: LISTO";
        let status_ebcdic = self.codec.to_host(status_ascii.as_bytes());
        screen_data.extend_from_slice(&status_ebcdic);

        // Línea de comandos disponibles en fila 5
        let (high, low) = Self::encode_buffer_addr(5, 0);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xF0); // Atributo: Protegido + Normal intensidad
        
        let commands_ascii = "Comandos: MENU, STATUS, EXIT";
        let commands_ebcdic = self.codec.to_host(commands_ascii.as_bytes());
        screen_data.extend_from_slice(&commands_ebcdic);

        // Prompt en la parte inferior
        let (high, low) = Self::encode_buffer_addr(20, 0);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xF0); // Atributo: Protegido + Alta intensidad
        
        let prompt_ascii = "Comando: ";
        let prompt_ebcdic = self.codec.to_host(prompt_ascii.as_bytes());
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
        debug!("Entering ScreenManager::clear_screen");
        self.screen_buffer.clear();
        self.screen_sent = false;
    }

    /// Genera una pantalla de prueba muy básica para verificar conectividad
    pub fn generate_basic_test_screen(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
        debug!("Entering ScreenManager::generate_basic_test_screen");
        let mut screen_data = Vec::new();
        
        // Comando Erase/Write y WCC
        screen_data.push(0xF5); // Comando: Erase/Write
        screen_data.push(0xC0); // WCC: Reset Keyboard + Reset MDT + Unlock Keyboard

        // Campo protegido inicial sin posicionamiento explícito
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0x20); // Protegido
        
        // Texto simple concatenado
        let text = "NEO6 FUNCIONA! Si ve esto, el protocolo TN3270 esta operativo.";
        let text_ebcdic = self.codec.to_host(text.as_bytes());
        screen_data.extend_from_slice(&text_ebcdic);

        // Crear campo de entrada al final
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0x00); // Desprotegido
        
        // Posicionar cursor
        screen_data.push(0x13); // IC (Insert Cursor)

        // Guardar en el buffer de pantalla
        self.screen_buffer = screen_data.clone();
        
        debug!("Pantalla ultra-simple generada");
        debug!("Bytes enviados: {}", screen_data.len());
        trace!("Datos completos (hex): {:02X?}", screen_data);
        
        Ok(screen_data)
    }
}

impl Default for ScreenManager {
    fn default() -> Self {
        debug!("Entering ScreenManager::default");
        Self::new()
    }
}
