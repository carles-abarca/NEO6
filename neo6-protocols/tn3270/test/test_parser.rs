use std::env;
use std::path::Path;
use tn3270::ScreenManager;

fn decode_addr(high: u8, low: u8) -> u16 {
    (((high & 0x3F) as u16) << 6) | ((low & 0x3F) as u16)
}

fn verify_stream(data: &[u8]) -> Result<(), String> {
    if data.len() < 2 || data[0] != 0xF5 {
        return Err("TN3270E stream must start with Erase/Write".to_string());
    }
    let mut i = 2; // after F5 and WCC
    while i < data.len() {
        match data[i] {
            0x11 => {
                if i + 2 >= data.len() {
                    return Err("Truncated SBA command".to_string());
                }
                let addr = decode_addr(data[i + 1], data[i + 2]);
                if addr >= 1920 {
                    return Err(format!("Buffer address {} out of range", addr));
                }
                i += 3;
            }
            0x1D => {
                if i + 1 >= data.len() {
                    return Err("Truncated SF command".to_string());
                }
                i += 2; // attribute byte
            }
            0x28 => {
                if i + 2 >= data.len() {
                    return Err("Truncated SA command".to_string());
                }
                i += 3;
            }
            0x29 => {
                if i + 1 >= data.len() {
                    return Err("Truncated SFE command".to_string());
                }
                let count = data[i + 1] as usize;
                if i + 2 + (count * 2) > data.len() {
                    return Err("SFE attribute bytes truncated".to_string());
                }
                i += 2 + count * 2;
            }
            0x13 => {
                i += 1; // IC
            }
            _ => {
                i += 1; // regular data byte
            }
        }
    }
    Ok(())
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
    if let Some(config_dir) = template_path.parent().and_then(|p| p.parent()) {
        if let Err(e) = env::set_current_dir(config_dir) {
            eprintln!("Failed to set working dir: {}", e);
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
            println!("Generated TN3270E stream ({} bytes)", data.len());
            print_hex(&data);
            match verify_stream(&data) {
                Ok(()) => println!("\nStream verification passed."),
                Err(e) => println!("\nStream verification failed: {}", e),
            }
        }
        Err(e) => {
            eprintln!("Error generating screen: {}", e);
            std::process::exit(1);
        }
    }
}
