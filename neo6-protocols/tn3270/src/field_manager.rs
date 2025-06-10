// Gestor de campos de entrada para pantallas TN3270
use crate::template_parser::FieldAttributes;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

/// Errores relacionados con la gestión de campos
#[derive(Debug)]
pub enum FieldError {
    FieldNotFound(String),
    InvalidFieldValue(String),
    FieldOverlap(String, String),
    ValidationError(String),
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FieldError::FieldNotFound(name) => write!(f, "Campo no encontrado: {}", name),
            FieldError::InvalidFieldValue(msg) => write!(f, "Valor de campo inválido: {}", msg),
            FieldError::FieldOverlap(field1, field2) => {
                write!(f, "Solapamiento de campos: {} y {}", field1, field2)
            }
            FieldError::ValidationError(msg) => write!(f, "Error de validación: {}", msg),
        }
    }
}

impl Error for FieldError {}

/// Representa un campo de entrada en la pantalla
#[derive(Debug, Clone)]
pub struct ScreenField {
    pub name: String,
    pub row: u16,
    pub col: u16,
    pub length: usize,
    pub value: String,
    pub attributes: FieldAttributes,
}

impl ScreenField {
    pub fn new(name: String, row: u16, col: u16, attributes: FieldAttributes) -> Self {
        let length = attributes.length.unwrap_or(10); // Longitud por defecto
        Self {
            name,
            row,
            col,
            length,
            value: attributes.default_value.clone(),
            attributes,
        }
    }

    /// Valida el valor del campo según sus atributos
    pub fn validate_value(&self, value: &str) -> Result<String, FieldError> {
        let mut validated_value = value.to_string();

        // Verificar longitud máxima
        if validated_value.len() > self.length {
            validated_value = validated_value[..self.length].to_string();
        }

        // Validaciones específicas por tipo
        if self.attributes.numeric {
            if !validated_value.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-') {
                return Err(FieldError::InvalidFieldValue(
                    format!("El campo {} solo acepta valores numéricos", self.name)
                ));
            }
        }

        // Convertir a mayúsculas si es necesario
        if self.attributes.uppercase {
            validated_value = validated_value.to_uppercase();
        }

        // Verificar campo protegido
        if self.attributes.protected {
            return Err(FieldError::ValidationError(
                format!("El campo {} es de solo lectura", self.name)
            ));
        }

        Ok(validated_value)
    }

    /// Establece el valor del campo con validación
    pub fn set_value(&mut self, value: &str) -> Result<(), FieldError> {
        let validated_value = self.validate_value(value)?;
        self.value = validated_value;
        Ok(())
    }

    /// Obtiene el valor formateado para mostrar en pantalla
    pub fn get_display_value(&self) -> String {
        if self.attributes.hidden {
            "*".repeat(self.value.len())
        } else {
            format!("{:<width$}", self.value, width = self.length)
        }
    }

    /// Verifica si este campo se solapa con otro
    pub fn overlaps_with(&self, other: &ScreenField) -> bool {
        // Si están en filas diferentes, no se solapan
        if self.row != other.row {
            return false;
        }

        // Verificar solapamiento horizontal
        let self_end = self.col + self.length as u16;
        let other_end = other.col + other.length as u16;

        !(self_end <= other.col || other_end <= self.col)
    }

    /// Genera los bytes 3270 para este campo
    pub fn to_3270_bytes(&self, codec: &crate::Codec) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Atributo del campo
        let attr_byte = if self.attributes.protected {
            if self.attributes.hidden {
                0x6C // Protegido + No display
            } else {
                0xF0 // Protegido + Normal
            }
        } else {
            0x60 // Desprotegido + Normal
        };

        bytes.push(0x1D); // SF (Start Field)
        bytes.push(attr_byte);

        // Agregar color si no es el por defecto
        // (Esto se implementaría con SFE - Start Field Extended)

        // Contenido del campo
        let display_value = self.get_display_value();
        let ebcdic_content = codec.to_host(display_value.as_bytes());
        bytes.extend_from_slice(&ebcdic_content);

        bytes
    }
}

/// Gestor principal de campos de pantalla
#[derive(Debug)]
pub struct FieldManager {
    fields: HashMap<String, ScreenField>,
    field_order: Vec<String>, // Para mantener el orden de tabulación
}

impl FieldManager {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            field_order: Vec::new(),
        }
    }

    /// Añade un campo al gestor
    pub fn add_field(&mut self, field: ScreenField) -> Result<(), FieldError> {
        // Verificar solapamientos con campos existentes
        for existing_field in self.fields.values() {
            if field.overlaps_with(existing_field) {
                return Err(FieldError::FieldOverlap(
                    field.name.clone(),
                    existing_field.name.clone(),
                ));
            }
        }

        self.field_order.push(field.name.clone());
        self.fields.insert(field.name.clone(), field);
        Ok(())
    }

    /// Obtiene un campo por nombre
    pub fn get_field(&self, name: &str) -> Option<&ScreenField> {
        self.fields.get(name)
    }

    /// Obtiene un campo mutable por nombre
    pub fn get_field_mut(&mut self, name: &str) -> Option<&mut ScreenField> {
        self.fields.get_mut(name)
    }

    /// Establece el valor de un campo
    pub fn set_field_value(&mut self, name: &str, value: &str) -> Result<(), FieldError> {
        let field = self.fields.get_mut(name)
            .ok_or_else(|| FieldError::FieldNotFound(name.to_string()))?;
        
        field.set_value(value)
    }

    /// Obtiene el valor de un campo
    pub fn get_field_value(&self, name: &str) -> Result<String, FieldError> {
        let field = self.fields.get(name)
            .ok_or_else(|| FieldError::FieldNotFound(name.to_string()))?;
        
        Ok(field.value.clone())
    }

    /// Obtiene todos los valores de campos como HashMap
    pub fn get_all_values(&self) -> HashMap<String, String> {
        self.fields.iter()
            .map(|(name, field)| (name.clone(), field.value.clone()))
            .collect()
    }

    /// Limpia todos los campos (excepto los protegidos)
    pub fn clear_fields(&mut self) {
        for field in self.fields.values_mut() {
            if !field.attributes.protected {
                field.value.clear();
            }
        }
    }

    /// Obtiene el siguiente campo en el orden de tabulación
    pub fn get_next_field(&self, current_field: &str) -> Option<&str> {
        if let Some(current_index) = self.field_order.iter().position(|name| name == current_field) {
            let next_index = (current_index + 1) % self.field_order.len();
            self.field_order.get(next_index).map(|s| s.as_str())
        } else {
            self.field_order.first().map(|s| s.as_str())
        }
    }

    /// Obtiene el campo anterior en el orden de tabulación
    pub fn get_previous_field(&self, current_field: &str) -> Option<&str> {
        if let Some(current_index) = self.field_order.iter().position(|name| name == current_field) {
            let prev_index = if current_index == 0 {
                self.field_order.len() - 1
            } else {
                current_index - 1
            };
            self.field_order.get(prev_index).map(|s| s.as_str())
        } else {
            self.field_order.last().map(|s| s.as_str())
        }
    }

    /// Obtiene todos los campos ordenados por posición (fila, luego columna)
    pub fn get_fields_by_position(&self) -> Vec<&ScreenField> {
        let mut fields: Vec<&ScreenField> = self.fields.values().collect();
        fields.sort_by(|a, b| {
            a.row.cmp(&b.row).then(a.col.cmp(&b.col))
        });
        fields
    }

    /// Valida todos los campos
    pub fn validate_all_fields(&self) -> Result<(), FieldError> {
        for field in self.fields.values() {
            field.validate_value(&field.value)?;
        }
        Ok(())
    }

    /// Busca el campo en una posición específica
    pub fn find_field_at_position(&self, row: u16, col: u16) -> Option<&ScreenField> {
        self.fields.values().find(|field| {
            field.row == row && col >= field.col && col < field.col + field.length as u16
        })
    }

    /// Obtiene estadísticas de los campos
    pub fn get_field_stats(&self) -> FieldStats {
        let total_fields = self.fields.len();
        let protected_fields = self.fields.values().filter(|f| f.attributes.protected).count();
        let hidden_fields = self.fields.values().filter(|f| f.attributes.hidden).count();
        let numeric_fields = self.fields.values().filter(|f| f.attributes.numeric).count();
        
        FieldStats {
            total_fields,
            protected_fields,
            hidden_fields,
            numeric_fields,
            input_fields: total_fields - protected_fields,
        }
    }

    /// Exporta la configuración de campos para depuración
    pub fn debug_export(&self) -> String {
        let mut output = String::new();
        output.push_str("=== CONFIGURACIÓN DE CAMPOS ===\n");
        
        for field in self.get_fields_by_position() {
            output.push_str(&format!(
                "Campo: {} | Pos: ({},{}) | Longitud: {} | Valor: '{}' | Protegido: {} | Oculto: {}\n",
                field.name, field.row, field.col, field.length, field.value,
                field.attributes.protected, field.attributes.hidden
            ));
        }
        
        output.push_str(&format!("\nEstadísticas: {:?}\n", self.get_field_stats()));
        output
    }
}

/// Estadísticas de campos
#[derive(Debug)]
pub struct FieldStats {
    pub total_fields: usize,
    pub protected_fields: usize,
    pub hidden_fields: usize,
    pub numeric_fields: usize,
    pub input_fields: usize,
}

impl Default for FieldManager {
    fn default() -> Self {
        Self::new()
    }
}
