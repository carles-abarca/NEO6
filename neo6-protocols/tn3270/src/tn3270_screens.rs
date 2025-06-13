use crate::{Codec, TemplateParser, FieldManager, ScreenField};
use crate::template_parser::TemplateElement;
use crate::template_parser::Color3270;
use crate::tn3270_constants::*;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, trace, warn};

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
        // CRITICAL: All field attributes MUST include FA_PRINTABLE bits (0xC0)
        // This is required by TN3270 protocol to make the field attribute valid
        let mut attr = FA_PRINTABLE;  // FA_PRINTABLE bits are MANDATORY
        
        if self.protected { attr |= FA_PROTECT; }  // FA_PROTECT
        if self.numeric { attr |= FA_NUMERIC; }    // FA_NUMERIC
        if !self.display { attr |= FA_INT_ZERO_NSEL; }   // FA_INT_ZERO_NSEL
        
        // CRITICAL: Add detectability bits for unprotected fields
        // This makes the field selectable and allows cursor positioning
        if !self.protected {
            attr |= FA_INT_NORM_SEL;  // Add detectability bits
        }
        attr
    }
}

/// Estructura para manejar pantallas 3270
#[derive(Debug)]
pub struct ScreenManager {
    pub screen_buffer: Vec<u8>,
    pub screen_sent: bool,
    pub codec: Codec,
    pub field_navigator: crate::field_navigation::FieldNavigator,
}

impl ScreenManager {
    pub fn new() -> Self {
        debug!("Entering ScreenManager::new");
        ScreenManager {
            screen_buffer: vec![0x00; 1920], // 80x24 por defecto
            screen_sent: false,
            codec: Codec::new(),
            field_navigator: crate::field_navigation::FieldNavigator::new(),
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
            trace!("🚨 CONVERTING SPECIAL CHARS: text={:?}", text);
            for (i, &byte) in text_bytes.iter().enumerate() {
                trace!("  [{}] ASCII byte: 0x{:02X} ({})", i, byte, byte as char);
            }
        }
        
        let text_ebcdic = self.codec.to_host(text_bytes);
        debug!("🔍 AFTER EBCDIC CONVERSION: {} bytes", text_ebcdic.len());
        
        // Log específico para la conversión EBCDIC de caracteres problemáticos
        if text.contains('+') || text.contains('|') {
            trace!("🚨 EBCDIC CONVERSION RESULT:");
            for (i, &byte) in text_ebcdic.iter().enumerate() {
                trace!("  [{}] EBCDIC byte: 0x{:02X}", i, byte);
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
            trace!("🚨 CONVERTING SPECIAL CHARS: text={:?}", text);
            for (i, &byte) in text_bytes.iter().enumerate() {
                trace!("  [{}] ASCII byte: 0x{:02X} ({})", i, byte, byte as char);
            }
        }
        
        let text_ebcdic = self.codec.to_host(text_bytes);
        debug!("🔍 AFTER EBCDIC CONVERSION: {} bytes", text_ebcdic.len());
        
        // Log específico para la conversión EBCDIC de caracteres problemáticos
        if text.contains('+') || text.contains('|') || text.contains('=') {
            trace!("🚨 EBCDIC CONVERSION RESULT:");
            for (i, &byte) in text_ebcdic.iter().enumerate() {
                trace!("  [{}] EBCDIC byte: 0x{:02X}", i, byte);
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

    /// Carga una plantilla desde la ubicación correcta de pantallas buscando en el siguiente orden:
    /// 1. `{name}_screen.txt` (v2.0 formato con sintaxis de corchetes)
    /// 2. `{name}_markup.txt` (v1.0 formato legacy)
    /// 3. `{name}.txt` (archivo plano sin sufijo)
    fn load_template(&self, name: &str) -> Result<String, Box<dyn Error>> {
        debug!("Entering ScreenManager::load_template");
        
        // Try to get config directory from environment variable first
        let mut screens_dir = if let Ok(config_dir) = std::env::var("NEO6_CONFIG_DIR") {
            debug!("Using config directory from environment variable: {}", config_dir);
            PathBuf::from(config_dir).join("screens")
        } else {
            debug!("No NEO6_CONFIG_DIR environment variable found, using default config/screens");
            PathBuf::from("config/screens")
        };
        
        // If the configured path doesn't exist, try relative fallbacks for compatibility
        if !screens_dir.exists() {
            debug!("Configured screens directory {:?} doesn't exist, trying fallbacks", screens_dir);
            
            // Try from current directory as relative path
            screens_dir = PathBuf::from("config/screens");            
        }
        
        debug!("Using screens directory: {:?}", screens_dir);
        
        // Look for v2.0 screen files first (new format with _screen.txt suffix)
        let screen_path = screens_dir.join(format!("{}_screen.txt", name));
        debug!("Looking for v2.0 template at: {:?}", screen_path);
        if screen_path.exists() {
            debug!("Found v2.0 screen template, reading file");
            return Ok(fs::read_to_string(screen_path)?);
        }
        
        Err(format!("Template '{}' not found in {:?}", name, screens_dir).into())
    }

    /// Genera una pantalla TN3270 a partir de cualquier plantilla de `config/screens`
    pub fn generate_tn3270_screen(&mut self, template_name: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        debug!("Entering ScreenManager::generate_tn3270_screen");
        
        // IMPORTANTE: Limpiar el estado del buffer antes de generar nueva pantalla
        self.clear_screen();

        let mut screen_data = Vec::new();
        screen_data.push(0xF5); // Erase/Write - borra completamente la pantalla y buffer
        // WCC (Write Control Character) con flags de reseteo completo:
        // Bit 7: 1 - Reset (resetea estado del terminal)
        // Bit 6: 1 - Print (permite impresión si aplicable)  
        // Bit 5: 1 - Start printer (si aplicable)
        // Bit 4: 1 - Sound alarm (opcional)
        // Bit 3: 1 - (unused)
        // Bit 2: 1 - Reset modified data tags (resetea MDT de todos los campos)
        // Bit 1: 1 - Keyboard restore (restaura teclado) ← FIX CRÍTICO
        // Bit 0: 0 - Reserved
        screen_data.push(0xFE); // WCC con keyboard restore: 11111110

        // Agregar un campo protegido inicial que cubra toda la pantalla (posición 0,0)
        // Esto establece el contexto base para todos los atributos subsiguientes
        let (high, low) = Self::encode_buffer_addr(0, 0);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA to (0,0)
        screen_data.push(0x1D); // SF (Start Field)
        // TN3270 field attribute: 11100000 = 0xE0 (protected, normal display)
        // Bits 7-6: 11 (reserved bits must be set)
        // Bit 5: 1 (protected)
        // Bit 4: 0 (alphanumeric)
        // Bits 3-2: 00 (normal display)
        // Bit 1: 0 (reserved)
        // Bit 0: 0 (not modified)
        screen_data.push(0xE0); // Proper protected field attribute

        let mut template_content = self.load_template(template_name)?;
        
        // CRITICAL: Handle screen list variable replacement for COMMANDS template
        if template_name.to_uppercase() == "COMMANDS" {
            debug!("Processing COMMANDS template - checking for {{screen_list}} variable");
            if template_content.contains("{screen_list}") {
                match self.generate_screen_list() {
                    Ok(screen_list) => {
                        template_content = template_content.replace("{screen_list}", &screen_list);
                        debug!("Successfully replaced {{screen_list}} variable with {} lines", 
                               screen_list.lines().count());
                    }
                    Err(e) => {
                        warn!("Failed to generate screen list: {}", e);
                        // Fallback: replace with error message
                        let fallback = "[POS(5,5)][TEXT(Error: Failed to load screen list)]";
                        template_content = template_content.replace("{screen_list}", fallback);
                    }
                }
            }
        }
        
        let parser = TemplateParser::new();
        let elements = parser.parse_template(&template_content)?;

        // Debug: Print all parsed TemplateElement::Text elements
        debug!("\n🔍 DEBUG: Parsed TemplateElement::Text elements:");
        for (i, element) in elements.iter().enumerate() {
            if let TemplateElement::Text { content, row, col, color, bright, blink, underline } = element {
                debug!("  [{}] Text: {:?} at pos ({:?}, {:?}) color={:?} bright={} blink={} underline={}", 
                         i, content, row, col, color, bright, blink, underline);
            }
        }
        debug!("🔍 DEBUG: Total elements parsed: {}", elements.len());

        let mut field_manager = FieldManager::new();

        // Process elements first without adding initial protected field
        for element in &elements {
            match element {
                TemplateElement::Text { content, row, col, color, bright, blink, underline } => {
                    debug!("🔍 PROCESSING TEXT ELEMENT: content={:?} (len={}) row={:?} col={:?}", 
                           content, content.len(), row, col);
                    
                    // Log específico para caracteres problemáticos
                    if content.contains('+') || content.contains('|') {
                        trace!("🚨 SPECIAL CHARS DETECTED: content={:?} at pos ({:?},{:?})", content, row, col);
                        for (i, ch) in content.chars().enumerate() {
                            trace!("  [{}] char: '{}' (U+{:04X})", i, ch, ch as u32);
                        }
                    }
                    
                    if let (Some(r), Some(c)) = (row, col) {
                        // Templates use 1-indexed coordinates, tn3270 uses 0-indexed
                        debug!("🔍 CALLING add_positioned_colored_text with: row={} col={} text={:?} (len={})", 
                               r - 1, c - 1, content, content.len());
                        self.add_positioned_colored_text(&mut screen_data, *r - 1, *c - 1, content, *color, *bright, *blink, *underline);
                        debug!("🔍 COMPLETED add_positioned_colored_text");
                        debug!("🔍 DEBUG: Added colored text: {:?} at template row {} col {} (0-indexed: {},{}) with color={:?} bright={} blink={} underline={}", 
                                content, r, c, r-1, c-1, color, bright, blink, underline);
                    } else {
                        // If no position specified, just add colored text at current position
                        debug!("🔍 CALLING add_colored_text with: text={:?} (len={})", content, content.len());
                        self.add_colored_text(&mut screen_data, content, *color, *bright, *blink, *underline);
                        debug!("🔍 COMPLETED add_colored_text");
                        debug!("🔍 DEBUG: Added colored text at current position: {:?} with color={:?} bright={} blink={} underline={}", 
                                content, color, bright, blink, underline);
                    }
                }
                TemplateElement::Field { attributes, row, col } => {
                    if let (Some(r), Some(c)) = (row, col) {
                        let (high, low) = Self::encode_buffer_addr(*r - 1, *c - 1);
                        screen_data.extend_from_slice(&[0x11, high, low]);

                        screen_data.push(0x1D); // SF
                        
                        // CRITICAL FIX: Use the corrected FieldAttribute::to_byte() method
                        // which properly includes FA_PRINTABLE bits (0xC0) as required by TN3270 protocol
                        let field_attr_byte = attributes.to_byte();
                        
                        // Debug logging for field attributes
                        debug!("🔍 FIELD DEBUG: name={} protected={} numeric={} attr=0x{:02X} (includes FA_PRINTABLE)", 
                                attributes.name, attributes.protected, attributes.numeric, field_attr_byte);
                        
                        screen_data.push(field_attr_byte);

                        // CRITICAL: For unprotected fields, add exactly the specified length in spaces
                        // MOCHA may be sensitive to the exact field length and spacing
                        if !attributes.protected {
                            let field_length = attributes.length.unwrap_or(10);
                            let spaces = vec![0x40; field_length]; // 0x40 = espacio en EBCDIC
                            screen_data.extend_from_slice(&spaces);
                            
                            // CRITICAL FIX: Properly terminate the field with a new protected field
                            // In TN3270, fields are terminated by starting a new field immediately after
                            // the data area. This is the correct way to delimit field boundaries.
                            screen_data.push(0x1D); // SF (Start Field) to terminate the input field
                            screen_data.push(0xE0); // Protected field attribute to end the input area
                            
                            debug!("🔍 FIELD TERMINATION: Added {} EBCDIC spaces + terminating protected field for unprotected field '{}'", 
                                     field_length, attributes.name);
                        } else if !attributes.default_value.is_empty() {
                            // Para campos protegidos, agregar el default_value si existe
                            let def = self.codec.to_host(attributes.default_value.as_bytes());
                            screen_data.extend_from_slice(&def);
                            
                            // CRITICAL FIX: Properly terminate protected fields too
                            screen_data.push(0x1D); // SF (Start Field) to terminate 
                            screen_data.push(0xE0); // Protected field attribute 
                            
                            debug!("🔍 FIELD VALUE: Added default value '{}' + terminating protected field to protected field '{}'", 
                                     attributes.default_value, attributes.name);
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

        // Add final protected field at end of screen to ensure proper field setup
        // Position it at the last position of the screen (row 24, col 80)
        let (high, low) = Self::encode_buffer_addr(23, 79); // 0-based: 23,79 = 24,80
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        screen_data.push(0x1D); // SF
        screen_data.push(0xE0); // Proper protected field attribute (same as initial field)

        // Initialize field navigator with current fields
        if let Err(e) = self.field_navigator.initialize_from_field_manager(&field_manager) {
            warn!("Failed to initialize field navigator: {}", e);
        }

        // Position cursor at first input field (if any exists)
        let fields = field_manager.get_fields_by_position();
        if let Some(first_input_field) = fields.iter().find(|f| !f.attributes.protected) {
            // CRITICAL: Position cursor in the FIRST DATA POSITION of the input field
            // The field attribute is at (row, col), so first data position is at (row, col+1)
            let cursor_row = first_input_field.row - 1; // Convert to 0-based
            let cursor_col = first_input_field.col;     // Position after field attribute (first data position)
            let (high, low) = Self::encode_buffer_addr(cursor_row, cursor_col);
            
            screen_data.extend_from_slice(&[0x11, high, low]); // SBA to position cursor
            screen_data.push(0x13); // IC (Insert Cursor)
            
            // EXPERIMENTAL: Some TN3270 terminals require an additional step to fully enable input
            // Try adding a NUL character or space to ensure the field is ready for input
            // This is sometimes needed to "activate" the input field for certain emulators
            
            debug!("🔍 CURSOR POSITIONED: Field '{}' data area at TN3270 position ({},{}) -> buffer ({},{})", 
                     first_input_field.attributes.name, 
                     first_input_field.row, first_input_field.col + 1,
                     cursor_row + 1, cursor_col + 1);
        } else {
            warn!("🔍 WARNING: No input fields found - cursor not positioned");
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

    /// Genera la pantalla de bienvenida de NEO6 usando el nuevo parser de templates
    pub fn generate_welcome_screen(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
        debug!("Entering ScreenManager::generate_welcome_screen");
        // Use the new generic template generation method with "welcome" template
        self.generate_tn3270_screen("welcome")
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
        // Crear atributo correcto para campo desprotegido con bits detectability
        let field_attr = FieldAttribute::unprotected();
        screen_data.push(field_attr.to_byte()); // Atributo: 0xC4 (FA_PRINTABLE + detectability)
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

        // Campo de entrada visible para capturar ENTER con prompt más claro
        let (high, low) = Self::encode_buffer_addr(20, 0);
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
        
        screen_data.push(0x1D); // SF (Start Field)
        screen_data.push(0xF0); // Atributo: Protegido + Normal intensidad
        
        let prompt_ascii = "Presione ENTER: ";
        let prompt_ebcdic = self.codec.to_host(prompt_ascii.as_bytes());
        screen_data.extend_from_slice(&prompt_ebcdic);
        
        // Campo de entrada después del prompt
        screen_data.push(0x1D); // SF (Start Field) 
        let field_attr = FieldAttribute::unprotected();
        screen_data.push(field_attr.to_byte()); // Atributo: 0xC4 (FA_PRINTABLE + detectability)
        
        // Agregar algunos espacios para el campo
        let spaces = vec![0x40; 5]; // 5 espacios EBCDIC para visibilidad
        screen_data.extend_from_slice(&spaces);
        
        // Terminar el campo correctamente
        screen_data.push(0x1D); // SF (Start Field) para terminar
        screen_data.push(0xE0); // Protected field attribute para terminación
        
        // Posicionar cursor al principio del campo de entrada
        let (high, low) = Self::encode_buffer_addr(20, 16); // Justo después del prompt
        screen_data.extend_from_slice(&[0x11, high, low]); // SBA
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
        // Crear atributo correcto para campo desprotegido con bits detectability
        let field_attr = FieldAttribute::unprotected();
        screen_data.push(field_attr.to_byte()); // Atributo: 0xC4 (FA_PRINTABLE + detectability)
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

    /// Genera una pantalla por nombre, permitiendo navegación dinámica
    pub fn generate_screen_by_name(&mut self, screen_name: &str) -> Result<Vec<u8>, String> {
        debug!("Entering ScreenManager::generate_screen_by_name with name: {}", screen_name);
        
        match screen_name.to_uppercase().as_str() {
            "WELCOME" | "HOME" | "INICIO" => {
                // Usar plantilla welcome_markup.txt
                self.generate_tn3270_screen("welcome").map_err(|e| e.to_string())
            },
            "MENU" | "MAIN" => {
                // Usar plantilla MENU_markup.txt
                self.generate_tn3270_screen("MENU").map_err(|e| e.to_string())
            },
            "STATUS" | "ESTADO" => {
                // Usar plantilla STATUS_markup.txt
                self.generate_tn3270_screen("STATUS").map_err(|e| e.to_string())
            },
            "COMMANDS" | "COMANDOS" | "LIST" => {
                // Usar plantilla COMMANDS_screen.txt
                self.generate_tn3270_screen("COMMANDS").map_err(|e| e.to_string())
            },
            "COLORS" | "COLORES" | "COLOR" => {
                // Usar plantilla COLORS_markup.txt
                self.generate_tn3270_screen("COLORS").map_err(|e| e.to_string())
            },
            "TEST" | "PRUEBA" | "FIELDS" => {
                // Usar plantilla TEST_markup.txt
                self.generate_tn3270_screen("TEST").map_err(|e| e.to_string())
            },
            "HELP" | "AYUDA" => {
                // Usar plantilla HELP_markup.txt
                self.generate_tn3270_screen("HELP").map_err(|e| e.to_string())
            },
            "EXIT" | "SALIR" | "QUIT" => {
                // Usar plantilla EXIT_markup.txt
                self.generate_tn3270_screen("EXIT").map_err(|e| e.to_string())
            },
            // Soporte para navegación numérica desde el menú
            "1" => {
                self.generate_tn3270_screen("welcome").map_err(|e| e.to_string())
            },
            "2" => {
                self.generate_tn3270_screen("STATUS").map_err(|e| e.to_string())
            },
            "3" => {
                self.generate_tn3270_screen("COLORS").map_err(|e| e.to_string())
            },
            "4" => {
                self.generate_tn3270_screen("TEST").map_err(|e| e.to_string())
            },
            "5" => {
                self.generate_tn3270_screen("TEST").map_err(|e| e.to_string())
            },
            "6" => {
                self.generate_tn3270_screen("HELP").map_err(|e| e.to_string())
            },
            "7" => {
                self.generate_tn3270_screen("EXIT").map_err(|e| e.to_string())
            },
            _ => {
                // Intentar cargar como plantilla personalizada
                debug!("Attempting to load custom template: {}", screen_name);
                match self.generate_tn3270_screen(screen_name) {
                    Ok(screen) => Ok(screen),
                    Err(e) => {
                        debug!("Failed to load template '{}': {}", screen_name, e);
                        Err(format!("Pantalla '{}' no encontrada", screen_name))
                    }
                }
            }
        }
    }

    /// Genera una lista formateada de todas las pantallas disponibles con sus descripciones
    /// extrayendo metadatos de los archivos *_screen.txt
    pub fn generate_screen_list(&self) -> Result<String, Box<dyn Error>> {
        debug!("Entering ScreenManager::generate_screen_list");
        
        // Try to get config directory from environment variable first
        let mut screens_dir = if let Ok(config_dir) = std::env::var("NEO6_CONFIG_DIR") {
            debug!("Using config directory from environment variable for screen list: {}", config_dir);
            PathBuf::from(config_dir).join("screens")
        } else {
            debug!("No NEO6_CONFIG_DIR environment variable found, using default config/screens");
            PathBuf::from("config/screens")
        };
        
        // Si no existe, buscar en la estructura de directorios del proyecto
        if !screens_dir.exists() {
            screens_dir = PathBuf::from("neo6-proxy/config/screens");
        }
        
        // Si aún no existe, intentar rutas absolutas basadas en el workspace actual
        if !screens_dir.exists() {
            let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let mut search_path = current_dir.clone();
            for _ in 0..5 {
                let candidate = search_path.join("neo6-proxy/config/screens");
                if candidate.exists() {
                    screens_dir = candidate;
                    break;
                }
                search_path = search_path.parent().unwrap_or(Path::new(".")).to_path_buf();
            }
        }
        
        debug!("Using screens directory for list generation: {:?}", screens_dir);
        
        if !screens_dir.exists() {
            return Err("Screens directory not found".into());
        }
        
        // Leer archivos *_screen.txt
        let mut screen_entries = Vec::new();
        
        match fs::read_dir(&screens_dir) {
            Ok(entries) => {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if let Some(file_name) = path.file_name() {
                            if let Some(name_str) = file_name.to_str() {
                                if name_str.ends_with("_screen.txt") {
                                    let screen_name = name_str.replace("_screen.txt", "");
                                    debug!("Found screen file: {}", name_str);
                                    
                                    // Leer el archivo y extraer metadata
                                    match fs::read_to_string(&path) {
                                        Ok(content) => {
                                            let parser = crate::TemplateParser::new();
                                            if let Some(metadata) = parser.extract_metadata(&content) {
                                                debug!("Extracted metadata for {}: {}", screen_name, &metadata);
                                                screen_entries.push((screen_name.to_uppercase(), metadata));
                                            } else {
                                                // Si no hay metadata, usar el nombre como descripción
                                                screen_entries.push((screen_name.to_uppercase(), format!("Pantalla {}", screen_name)));
                                                debug!("No metadata found for {}, using default description", screen_name);
                                            }
                                        }
                                        Err(e) => {
                                            warn!("Error reading screen file {}: {}", name_str, e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                return Err(format!("Error reading screens directory: {}", e).into());
            }
        }
        
        // Ordenar entradas alfabéticamente
        screen_entries.sort_by(|a, b| a.0.cmp(&b.0));
        
        // Formatear la lista para TN3270 con límite de 77 caracteres por línea
        let mut formatted_lines = Vec::new();
        let max_width = 77; // TN3270 screen width minus borders
        
        for (screen_name, description) in screen_entries {
            // Formatear cada entrada: "NOMBRE - Descripción"
            let entry = format!("{} - {}", screen_name, description);
            
            if entry.len() <= max_width - 7 { // -7 for border and padding: "|    " + "   |"
                // La línea cabe completa
                formatted_lines.push(format!("[XY{},1][BLUE]|[/BLUE][XY{},4][TURQUOISE]{}[/TURQUOISE][XY{},79][BLUE]|[/BLUE]",
                    7 + formatted_lines.len(), 7 + formatted_lines.len(), entry, 7 + formatted_lines.len()));
            } else {
                // La línea es demasiado larga, dividirla
                let available_width = max_width - 7; // Espacio disponible para texto
                
                // Línea con nombre de pantalla
                let name_line = format!("{} -", screen_name);
                formatted_lines.push(format!("[XY{},1][BLUE]|[/BLUE][XY{},4][TURQUOISE]{}[/TURQUOISE][XY{},79][BLUE]|[/BLUE]",
                    7 + formatted_lines.len(), 7 + formatted_lines.len(), name_line, 7 + formatted_lines.len()));
                
                // Dividir descripción en palabras
                let words: Vec<&str> = description.split_whitespace().collect();
                let mut current_line = String::new();
                
                for word in words {
                    let test_line = if current_line.is_empty() {
                        format!("  {}", word) // Indentación para descripción
                    } else {
                        format!("{} {}", current_line, word)
                    };
                    
                    if test_line.len() <= available_width {
                        current_line = test_line;
                    } else {
                        // Línea completa, añadir y empezar nueva
                        if !current_line.is_empty() {
                            formatted_lines.push(format!("[XY{},1][BLUE]|[/BLUE][XY{},4][WHITE]{}[/WHITE][XY{},79][BLUE]|[/BLUE]",
                                7 + formatted_lines.len(), 7 + formatted_lines.len(), current_line, 7 + formatted_lines.len()));
                        }
                        current_line = format!("  {}", word);
                    }
                }
                
                // Añadir última línea si tiene contenido
                if !current_line.is_empty() {
                    formatted_lines.push(format!("[XY{},1][BLUE]|[/BLUE][XY{},4][WHITE]{}[/WHITE][XY{},79][BLUE]|[/BLUE]",
                        7 + formatted_lines.len(), 7 + formatted_lines.len(), current_line, 7 + formatted_lines.len()));
                }
            }
        }
        
        // Rellenar con líneas vacías hasta la línea 17 (para mantener consistencia con el template)
        while formatted_lines.len() < 11 { // 11 líneas de contenido (7-17)
            formatted_lines.push(format!("[XY{},1][BLUE]|[/BLUE][XY{},79][BLUE]|[/BLUE]",
                7 + formatted_lines.len(), 7 + formatted_lines.len()));
        }
        
        let result = formatted_lines.join("\n");
        debug!("Generated screen list with {} lines", formatted_lines.len());
        Ok(result)
    }
}

impl Default for ScreenManager {
    fn default() -> Self {
        debug!("Entering ScreenManager::default");
        Self::new()
    }
}
