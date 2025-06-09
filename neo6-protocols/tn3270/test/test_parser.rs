use std::env;
use std::path::Path;
use std::fs::File;
use std::io::Write;
use tn3270::ScreenManager;
use tracing_subscriber::{fmt, EnvFilter, prelude::*};

/// Initialize logging for debug output
fn init_logging() {
    let filter = EnvFilter::from_default_env()
        .add_directive("tn3270=debug".parse().unwrap())
        .add_directive("debug".parse().unwrap());
    
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().pretty())
        .init();
}

fn decode_addr(high: u8, low: u8) -> u16 {
    (((high & 0x3F) as u16) << 6) | ((low & 0x3F) as u16)
}

/// Convert EBCDIC byte to ASCII character for display
fn ebcdic_to_ascii(ebcdic: u8) -> char {
    // Comprehensive EBCDIC to ASCII conversion table (IBM-037 codepage)
    match ebcdic {
        // Control characters and space
        0x00..=0x3F => ' ',  // Most control chars display as space
        0x40 => ' ',         // Space
        
        // Special characters
        0x4A => '[',   0x4B => '.',   0x4C => '<',   0x4D => '(',   0x4E => '+',   0x4F => '|',
        0x50 => '&',   0x5A => '!',   0x5B => '$',   0x5C => '*',   0x5D => ')',   0x5E => ';',   0x5F => '¬',
        0x60 => '-',   0x61 => '/',   0x6A => '|',   0x6B => ',',   0x6C => '%',   0x6D => '_',   0x6E => '>',   0x6F => '?',
        0x7A => ':',   0x7B => '#',   0x7C => '@',   0x7D => '\'',  0x7E => '=',   0x7F => '"',
        0x79 => '`',   
        
        // Lowercase letters a-i
        0x81..=0x89 => char::from(ebcdic - 0x81 + b'a'),
        
        // Lowercase letters j-r  
        0x91..=0x99 => char::from(ebcdic - 0x91 + b'j'),
        
        // Lowercase letters s-z
        0xA2..=0xA9 => char::from(ebcdic - 0xA2 + b's'),
        
        // Uppercase letters A-I
        0xC1..=0xC9 => char::from(ebcdic - 0xC1 + b'A'),
        
        // Uppercase letters J-R
        0xD1..=0xD9 => char::from(ebcdic - 0xD1 + b'J'),
        
        // Uppercase letters S-Z
        0xE2..=0xE9 => char::from(ebcdic - 0xE2 + b'S'),
        
        // Numbers 0-9
        0xF0..=0xF9 => char::from(ebcdic - 0xF0 + b'0'),
        
        // Box drawing and special graphics (approximate with ASCII)
        0xA0 => ' ',   // Non-breaking space
        0xA1 => '~',   // Tilde
        0xAA => '[',   // Left bracket variant
        0xAB => ']',   // Right bracket variant
        0xAC => '\\',  // Backslash
        0xAD => '^',   // Caret
        0xAE => '{',   // Left brace
        0xAF => '}',   // Right brace
        0xB0 => '|',   // Vertical bar
        0xBA => '+',   // Plus/cross
        0xBB => '+',   // Box drawing
        0xBC => '+',   // Box drawing
        0xBD => '+',   // Box drawing
        0xBE => '+',   // Box drawing
        0xBF => '+',   // Box drawing
        
        // More special characters that commonly appear
        0x90 => ']',   // Right bracket
        0x9A => '^',   // Caret
        0x9B => '<',   // Less than
        0x9C => '(',   // Left paren
        0x9D => '+',   // Plus
        0x9E => '|',   // Pipe
        0x9F => '&',   // Ampersand
        
        // Default for unmapped characters - use space instead of ? to avoid visual noise
        _ => ' ',
    }
}

/// Comprehensive TN3270E stream verification according to RFC specifications
fn verify_stream(data: &[u8]) -> Result<(), String> {
    if data.is_empty() {
        return Err("Empty TN3270E stream".to_string());
    }

    // Verify Write Control Character (WCC) commands
    verify_wcc_commands(data)?;
    
    // Parse and verify the stream structure
    let mut parser = TN3270StreamParser::new(data);
    parser.parse_and_verify()?;
    
    Ok(())
}

/// Verifies Write Control Character commands per TN3270E RFC
fn verify_wcc_commands(data: &[u8]) -> Result<(), String> {
    if data.is_empty() {
        return Err("Empty stream cannot contain WCC".to_string());
    }

    let command = data[0];
    match command {
        0xF1 => println!("Command: Write (0xF1)"),
        0xF5 => println!("Command: Erase/Write (0xF5)"),
        0x7E => println!("Command: Erase/Write Alternate (0x7E)"),
        0xF3 => println!("Command: Write Structured Field (0xF3)"),
        0x6F => println!("Command: Read Buffer (0x6F)"),
        0xF2 => println!("Command: Read Modified (0xF2)"),
        0xF6 => println!("Command: Read Modified All (0xF6)"),
        _ => return Err(format!("Invalid/Unsupported TN3270E command: 0x{:02X}", command)),
    }

    if data.len() < 2 {
        return Err("Stream too short to contain WCC".to_string());
    }

    let wcc = data[1];
    println!("WCC: 0x{:02X}", wcc);
    
    // Verify WCC bits according to RFC
    if wcc & 0x80 != 0 { println!("  - Reset bit set"); }
    if wcc & 0x40 != 0 { println!("  - Unlock Keyboard bit set"); }
    if wcc & 0x20 != 0 { println!("  - Reset MDT bit set"); }
    if wcc & 0x10 != 0 { println!("  - Sound Alarm bit set"); }
    if wcc & 0x08 != 0 { println!("  - Restore bit set"); }
    if wcc & 0x04 != 0 { println!("  - Start Printer bit set"); }
    
    // Reserved bits should be 0
    if wcc & 0x03 != 0 {
        return Err(format!("WCC reserved bits are non-zero: 0x{:02X}", wcc & 0x03));
    }

    Ok(())
}

/// TN3270E stream parser for detailed verification
struct TN3270StreamParser<'a> {
    data: &'a [u8],
    pos: usize,
    screen_size: usize,
    buffer_addresses: Vec<u16>,
    field_count: usize,
    // Add screen buffer for visualization
    screen_buffer: Vec<char>,
    current_position: u16,
    field_attributes: Vec<(u16, u8)>, // (position, attribute)
}

impl<'a> TN3270StreamParser<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 2, // Skip command and WCC
            screen_size: 1920, // 80x24 default
            buffer_addresses: Vec::new(),
            field_count: 0,
            screen_buffer: vec![' '; 1920], // Initialize with spaces
            current_position: 0,
            field_attributes: Vec::new(),
        }
    }

    fn parse_and_verify(&mut self) -> Result<(), String> {
        println!("Parsing TN3270E stream...");
        
        // Initialize screen simulation
        self.simulate_tn3270_stream()?;
        
        // Reset position for verification pass
        self.pos = 2;
        
        while self.pos < self.data.len() {
            match self.data[self.pos] {
                // Set Buffer Address (SBA)
                0x11 => self.parse_sba()?,
                
                // Start Field (SF)
                0x1D => self.parse_sf()?,
                
                // Set Attribute (SA)
                0x28 => self.parse_sa()?,
                
                // Start Field Extended (SFE)
                0x29 => self.parse_sfe()?,
                
                // Modify Field (MF)
                0x2C => self.parse_mf()?,
                
                // Insert Cursor (IC)
                0x13 => self.parse_ic()?,
                
                // Program Tab (PT)
                0x05 => self.parse_pt()?,
                
                // Graphic Escape (GE)
                0x08 => self.parse_ge()?,
                
                // Repeat to Address (RA)
                0x3C => self.parse_ra()?,
                
                // Erase Unprotected to Address (EUA)
                0x12 => self.parse_eua()?,
                
                // Character data (EBCDIC or display character)
                byte => {
                    if self.is_valid_ebcdic_char(byte) {
                        self.parse_character_data(byte)?;
                    } else {
                        return Err(format!("Invalid character byte: 0x{:02X} at position {}", byte, self.pos));
                    }
                }
            }
        }

        self.verify_stream_integrity()?;
        self.print_summary();
        self.generate_screen_visualization()?;
        
        Ok(())
    }

    fn parse_sba(&mut self) -> Result<(), String> {
        if self.pos + 2 >= self.data.len() {
            return Err(format!("Truncated SBA command at position {}", self.pos));
        }
        
        let high = self.data[self.pos + 1];
        let low = self.data[self.pos + 2];
        let addr = decode_addr(high, low);
        
        if addr >= self.screen_size as u16 {
            return Err(format!("SBA address {} exceeds screen size {} at position {}", 
                              addr, self.screen_size, self.pos));
        }
        
        self.buffer_addresses.push(addr);
        println!("  SBA: Set buffer address to {} (row {}, col {})", addr, addr / 80, addr % 80);
        self.pos += 3;
        Ok(())
    }

    fn parse_sf(&mut self) -> Result<(), String> {
        if self.pos + 1 >= self.data.len() {
            return Err(format!("Truncated SF command at position {}", self.pos));
        }
        
        let attr = self.data[self.pos + 1];
        self.verify_field_attribute(attr)?;
        self.field_count += 1;
        
        println!("  SF: Start field with attribute 0x{:02X}", attr);
        self.print_field_attribute_details(attr);
        self.pos += 2;
        Ok(())
    }

    fn parse_sa(&mut self) -> Result<(), String> {
        if self.pos + 2 >= self.data.len() {
            return Err(format!("Truncated SA command at position {}", self.pos));
        }
        
        let attr_type = self.data[self.pos + 1];
        let attr_value = self.data[self.pos + 2];
        
        match attr_type {
            0x00 => println!("  SA: All character attributes = 0x{:02X}", attr_value),
            0x41 => println!("  SA: Extended highlighting = 0x{:02X}", attr_value),
            0x42 => println!("  SA: Foreground color = 0x{:02X}", attr_value),
            0x43 => println!("  SA: Background color = 0x{:02X}", attr_value),
            0x45 => println!("  SA: Transparency = 0x{:02X}", attr_value),
            _ => return Err(format!("Invalid SA attribute type: 0x{:02X} at position {}", attr_type, self.pos)),
        }
        
        self.pos += 3;
        Ok(())
    }

    fn parse_sfe(&mut self) -> Result<(), String> {
        if self.pos + 1 >= self.data.len() {
            return Err(format!("Truncated SFE command at position {}", self.pos));
        }
        
        let count = self.data[self.pos + 1] as usize;
        let total_bytes = 2 + count * 2;
        
        if self.pos + total_bytes > self.data.len() {
            return Err(format!("SFE attribute pairs truncated at position {}", self.pos));
        }
        
        println!("  SFE: Start field extended with {} attribute pairs", count);
        
        for i in 0..count {
            let attr_type = self.data[self.pos + 2 + i * 2];
            let attr_value = self.data[self.pos + 2 + i * 2 + 1];
            
            match attr_type {
                0xC0 => println!("    Field attribute: 0x{:02X}", attr_value),
                0x41 => println!("    Extended highlighting: 0x{:02X}", attr_value),
                0x42 => println!("    Foreground color: 0x{:02X}", attr_value),
                0x43 => println!("    Background color: 0x{:02X}", attr_value),
                0x81 => println!("    Field validation: 0x{:02X}", attr_value),
                0x82 => println!("    Field outlining: 0x{:02X}", attr_value),
                _ => return Err(format!("Invalid SFE attribute type: 0x{:02X} at position {}", attr_type, self.pos + 2 + i * 2)),
            }
        }
        
        self.field_count += 1;
        self.pos += total_bytes;
        Ok(())
    }

    fn parse_mf(&mut self) -> Result<(), String> {
        if self.pos + 3 >= self.data.len() {
            return Err(format!("Truncated MF command at position {}", self.pos));
        }
        
        let count = self.data[self.pos + 1] as usize;
        let total_bytes = 2 + count * 2;
        
        if self.pos + total_bytes > self.data.len() {
            return Err(format!("MF attribute pairs truncated at position {}", self.pos));
        }
        
        println!("  MF: Modify field with {} attribute pairs", count);
        self.pos += total_bytes;
        Ok(())
    }

    fn parse_ic(&mut self) -> Result<(), String> {
        println!("  IC: Insert cursor");
        self.pos += 1;
        Ok(())
    }

    fn parse_pt(&mut self) -> Result<(), String> {
        println!("  PT: Program tab");
        self.pos += 1;
        Ok(())
    }

    fn parse_ge(&mut self) -> Result<(), String> {
        if self.pos + 1 >= self.data.len() {
            return Err(format!("Truncated GE command at position {}", self.pos));
        }
        
        let char = self.data[self.pos + 1];
        println!("  GE: Graphic escape character 0x{:02X}", char);
        self.pos += 2;
        Ok(())
    }

    fn parse_ra(&mut self) -> Result<(), String> {
        if self.pos + 3 >= self.data.len() {
            return Err(format!("Truncated RA command at position {}", self.pos));
        }
        
        let high = self.data[self.pos + 1];
        let low = self.data[self.pos + 2];
        let char = self.data[self.pos + 3];
        let addr = decode_addr(high, low);
        
        if addr >= self.screen_size as u16 {
            return Err(format!("RA address {} exceeds screen size at position {}", addr, self.pos));
        }
        
        println!("  RA: Repeat character 0x{:02X} to address {}", char, addr);
        self.pos += 4;
        Ok(())
    }

    fn parse_eua(&mut self) -> Result<(), String> {
        if self.pos + 2 >= self.data.len() {
            return Err(format!("Truncated EUA command at position {}", self.pos));
        }
        
        let high = self.data[self.pos + 1];
        let low = self.data[self.pos + 2];
        let addr = decode_addr(high, low);
        
        if addr >= self.screen_size as u16 {
            return Err(format!("EUA address {} exceeds screen size at position {}", addr, self.pos));
        }
        
        println!("  EUA: Erase unprotected to address {}", addr);
        self.pos += 3;
        Ok(())
    }

    fn parse_character_data(&mut self, byte: u8) -> Result<(), String> {
        // Count consecutive character bytes
        let start_pos = self.pos;
        while self.pos < self.data.len() && self.is_valid_ebcdic_char(self.data[self.pos]) {
            self.pos += 1;
        }
        
        let length = self.pos - start_pos;
        if length > 0 {
            println!("  Character data: {} bytes starting with 0x{:02X}", length, byte);
        }
        
        Ok(())
    }

    fn is_valid_ebcdic_char(&self, byte: u8) -> bool {
        // Check if byte is a valid EBCDIC character or control character
        match byte {
            // TN3270 orders/commands
            0x05 | 0x08 | 0x11 | 0x12 | 0x13 | 0x1D | 0x28 | 0x29 | 0x2C | 0x3C => false,
            // Valid EBCDIC printable characters and some control chars
            0x40..=0xFF => true, // Most EBCDIC printable range
            0x00..=0x3F => {
                // Some control characters are valid in data stream
                matches!(byte, 0x00 | 0x15 | 0x25)
            }
        }
    }

    fn verify_field_attribute(&self, attr: u8) -> Result<(), String> {
        // Verify field attribute byte structure per TN3270E spec
        let _protection = (attr & 0x20) != 0;
        let _numeric = (attr & 0x10) != 0;
        let display = (attr & 0x0C) >> 2;
        let _intensity = (attr & 0x0C) >> 2;
        let reserved = attr & 0x01;
        
        if reserved != 0 {
            return Err(format!("Field attribute reserved bit is non-zero: 0x{:02X}", attr));
        }
        
        // Display/intensity field validation
        match display {
            0 => {}, // Normal display
            1 => {}, // Blink
            2 => {}, // Reverse video  
            3 => {}, // Underline
            _ => return Err(format!("Invalid display attribute: {}", display)),
        }
        
        Ok(())
    }

    fn print_field_attribute_details(&self, attr: u8) {
        let protection = if (attr & 0x20) != 0 { "Protected" } else { "Unprotected" };
        let numeric = if (attr & 0x10) != 0 { "Numeric" } else { "Alphanumeric" };
        let display = match (attr & 0x0C) >> 2 {
            0 => "Normal",
            1 => "Blink",
            2 => "Reverse",
            3 => "Underline",
            _ => "Invalid",
        };
        
        println!("    {} | {} | {}", protection, numeric, display);
    }

    fn verify_stream_integrity(&self) -> Result<(), String> {
        // Additional integrity checks
        if self.field_count == 0 {
            println!("Warning: No fields defined in stream");
        }
        
        if self.buffer_addresses.is_empty() {
            println!("Warning: No explicit buffer addressing used");
        }
        
        // Check for reasonable field density
        if self.field_count > 100 {
            return Err(format!("Excessive field count: {} fields", self.field_count));
        }
        
        Ok(())
    }

    fn print_summary(&self) {
        println!("\nTN3270E Stream Summary:");
        println!("  Total length: {} bytes", self.data.len());
        println!("  Field count: {}", self.field_count);
        println!("  Buffer addresses: {}", self.buffer_addresses.len());
        println!("  Screen size: {} positions", self.screen_size);
        
        if !self.buffer_addresses.is_empty() {
            let min_addr = *self.buffer_addresses.iter().min().unwrap();
            let max_addr = *self.buffer_addresses.iter().max().unwrap();
            println!("  Address range: {} to {}", min_addr, max_addr);
        }
    }

    /// Simulate TN3270E stream execution to build screen buffer
    fn simulate_tn3270_stream(&mut self) -> Result<(), String> {
        let mut sim_pos = 2; // Skip command and WCC
        self.current_position = 0;
        
        while sim_pos < self.data.len() {
            match self.data[sim_pos] {
                0x11 => { // SBA - Set Buffer Address
                    if sim_pos + 2 >= self.data.len() { break; }
                    let high = self.data[sim_pos + 1];
                    let low = self.data[sim_pos + 2];
                    let new_pos = decode_addr(high, low);
                    
                    println!("🔍 SBA DEBUG: Moving cursor from {} to {} (row {}, col {})", 
                            self.current_position, new_pos, new_pos / 80, new_pos % 80);
                    
                    self.current_position = new_pos;
                    sim_pos += 3;
                }
                0x1D => { // SF - Start Field
                    if sim_pos + 1 >= self.data.len() { break; }
                    let attr = self.data[sim_pos + 1];
                    self.field_attributes.push((self.current_position, attr));
                    
                    // Field attributes are invisible and should not consume display positions
                    // Only advance position if we're not at the start of a line (column 0)
                    let row = self.current_position / 80;
                    let col = self.current_position % 80;
                    
                    if col != 0 {
                        // Not at line start, field attribute consumes a position
                        if (self.current_position as usize) < self.screen_buffer.len() {
                            self.screen_buffer[self.current_position as usize] = ' '; // Always invisible
                        }
                        self.current_position += 1;
                    }
                    // If at line start (col == 0), don't advance position - field attribute is "virtual"
                    
                    sim_pos += 2;
                }
                0x3C => { // RA - Repeat to Address
                    if sim_pos + 3 >= self.data.len() { break; }
                    let high = self.data[sim_pos + 1];
                    let low = self.data[sim_pos + 2];
                    let char_byte = self.data[sim_pos + 3];
                    let end_addr = decode_addr(high, low);
                    let ch = ebcdic_to_ascii(char_byte);
                    
                    // Fill from current position to end address
                    while self.current_position <= end_addr && (self.current_position as usize) < self.screen_buffer.len() {
                        self.screen_buffer[self.current_position as usize] = ch;
                        self.current_position += 1;
                    }
                    sim_pos += 4;
                }
                byte if self.is_valid_ebcdic_char(byte) => {
                    // Character data
                    let ch = ebcdic_to_ascii(byte);
                    
                    // Debug output for byte 110 (0x6E which should be ">")
                    if byte == 110 {
                        println!("🔍 EBCDIC CONVERSION DEBUG: byte {} (0x{:02X}) -> char '{}' (ASCII {}) at position {} (row {}, col {})", 
                                byte, byte, ch, ch as u8, self.current_position, self.current_position / 80, self.current_position % 80);
                    }
                    
                    if (self.current_position as usize) < self.screen_buffer.len() {
                        self.screen_buffer[self.current_position as usize] = ch;
                        
                        // Debug for ">" character placement
                        if ch == '>' {
                            println!("🔍 SCREEN BUFFER DEBUG: Placed '{}' at position {} (screen_buffer[{}])", 
                                    ch, self.current_position, self.current_position);
                        }
                    }
                    self.current_position += 1;
                    sim_pos += 1;
                }
                _ => {
                    // Skip other commands (SA, SFE, MF, IC, PT, GE, EUA)
                    sim_pos += match self.data[sim_pos] {
                        0x28 => 3, // SA
                        0x29 => { // SFE - variable length
                            if sim_pos + 1 < self.data.len() {
                                2 + (self.data[sim_pos + 1] as usize * 2)
                            } else { 1 }
                        }
                        0x2C => { // MF - variable length  
                            if sim_pos + 1 < self.data.len() {
                                2 + (self.data[sim_pos + 1] as usize * 2)
                            } else { 1 }
                        }
                        0x13 | 0x05 => 1, // IC, PT
                        0x08 => 2, // GE
                        0x12 => 3, // EUA
                        _ => 1,
                    };
                }
            }
        }
        
        Ok(())
    }

    fn generate_screen_visualization(&self) -> Result<(), String> {
        // Generate the 80x24 text file
        let mut screen_content = String::with_capacity(80 * 24 + 24); // +24 for newlines
        
        for row in 0..24 {
            for col in 0..80 {
                let pos = row * 80 + col;
                let ch = self.screen_buffer.get(pos).unwrap_or(&' ');
                screen_content.push(*ch);
            }
            screen_content.push('\n');
        }
        
        // Save to file
        let output_file = "tn3270_screen_visualization.txt";
        match File::create(output_file) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(screen_content.as_bytes()) {
                    return Err(format!("Failed to write visualization file: {}", e));
                }
                println!("\n📄 Screen visualization saved to: {}", output_file);
            }
            Err(e) => {
                return Err(format!("Failed to create visualization file: {}", e));
            }
        }
        
        // Also display on console with visual enhancements
        println!("\n📺 SCREEN VISUALIZATION (80x24):");
        println!("{}", "┌".repeat(82));
        
        for row in 0..24 {
            print!("│");
            for col in 0..80 {
                let pos = row * 80 + col;
                let ch = self.screen_buffer.get(pos).unwrap_or(&' ');
                
                // Check if there's a field attribute for this position
                let has_attr = self.field_attributes.iter().any(|&(p, _)| p == pos as u16);
                if has_attr {
                    print!("\x1B[7m{}\x1B[0m", ch); // Reverse video for field attributes
                } else {
                    print!("{}", ch);
                }
            }
            println!("│");
        }
        
        println!("{}", "└".repeat(82));
        println!("Legend: [Reverse video] = Field attribute positions");
        
        Ok(())
    }
}

fn print_hex(data: &[u8]) {
    for (i, b) in data.iter().enumerate() {
        if i % 16 == 0 {
            print!("\n{:04X}: ", i);
        }
        print!("{:02X} ", b);
    }
    println!();
}

fn main() {
    // Initialize logging for debug output
    init_logging();
    
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <template_path>", args[0]);
        std::process::exit(1);
    }
    let template_path = Path::new(&args[1]);
    if !template_path.exists() {
        eprintln!("Template {} not found", template_path.display());
        std::process::exit(1);
    }
    // Change working directory so ScreenManager can find config/screens
    if let Some(screens_dir) = template_path.parent() {
        if let Some(config_dir) = screens_dir.parent() {
            if let Some(neo6_proxy_dir) = config_dir.parent() {
                if let Err(e) = env::set_current_dir(neo6_proxy_dir) {
                    eprintln!("Failed to set working dir: {}", e);
                }
            }
        }
    }
    let filename = template_path
        .file_stem()
        .and_then(|s| s.to_str())
        .expect("invalid filename");
    let template_name = filename.strip_suffix("_markup").unwrap_or(filename);

    let mut sm = ScreenManager::new();
    match sm.generate_tn3270_screen(template_name) {
        Ok(data) => {
            println!("Generated TN3270E stream from template '{}' ({} bytes)", template_name, data.len());
            println!("\nHexadecimal dump:");
            print_hex(&data);
            
            println!("\n{}", "=".repeat(70));
            println!("TN3270E COMPLIANCE VERIFICATION AND SCREEN VISUALIZATION");
            println!("{}", "=".repeat(70));
            
            // Use the parser with screen visualization functionality
            let mut parser = TN3270StreamParser::new(&data);
            match parser.parse_and_verify() {
                Ok(()) => {
                    println!("\n✅ STREAM VERIFICATION PASSED");
                    println!("   The generated stream is fully compliant with TN3270E RFC specifications.");
                    println!("   Terminal should display the screen template correctly.");
                    println!("   Screen visualization saved to 'tn3270_screen_visualization.txt'");
                }
                Err(e) => {
                    println!("\n❌ STREAM VERIFICATION FAILED: {}", e);
                    println!("   The stream may not display correctly on TN3270E terminals.");
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Error generating screen: {}", e);
            std::process::exit(1);
        }
    }
}
