// Test para verificar los atributos de campo
use tn3270::template_parser::FieldAttributes;

fn main() {
    println!("Probando FieldAttributes::to_byte()");
    
    // Crear un campo desprotegido como el del template
    let mut field_attrs = FieldAttributes::new("command".to_string());
    field_attrs.protected = false;
    field_attrs.numeric = false;
    field_attrs.hidden = false;
    
    let attr_byte = field_attrs.to_byte();
    
    println!("Campo 'command' (desprotegido): 0x{:02X}", attr_byte);
    println!("Esperado: 0xC4 (0xC0 + 0x04)");
    println!("Actual: 0x{:02X}", attr_byte);
    
    if attr_byte == 0xC4 {
        println!("✅ CORRECTO!");
    } else {
        println!("❌ INCORRECTO! Debería ser 0xC4");
    }
}
