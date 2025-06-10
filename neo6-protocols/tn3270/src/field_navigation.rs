use tracing::{debug, warn, error};

/// Estructura para manejar la navegación entre campos en TN3270
#[derive(Debug, Clone)]
pub struct FieldNavigator {
    /// Posición actual del cursor (fila, columna) en coordenadas 0-based
    pub cursor_position: (u16, u16),
    /// Lista de campos de entrada ordenados por posición en pantalla
    input_fields: Vec<InputField>,
    /// Índice del campo actualmente activo
    current_field_index: Option<usize>,
}

/// Representa un campo de entrada en la pantalla
#[derive(Debug, Clone)]
pub struct InputField {
    /// Nombre del campo
    pub name: String,
    /// Posición del campo en la pantalla (0-based)
    pub position: (u16, u16),
    /// Longitud del campo en caracteres
    pub length: usize,
    /// Si el campo está protegido
    pub protected: bool,
    /// Si el campo es numérico
    pub numeric: bool,
}

impl FieldNavigator {
    /// Crea un nuevo navegador de campos
    pub fn new() -> Self {
        debug!("Creating new FieldNavigator");
        Self {
            cursor_position: (0, 0),
            input_fields: Vec::new(),
            current_field_index: None,
        }
    }

    /// Inicializa el navegador con los campos del FieldManager
    pub fn initialize_from_field_manager(&mut self, field_manager: &crate::FieldManager) -> Result<(), String> {
        debug!("Initializing FieldNavigator from FieldManager");
        
        self.input_fields.clear();
        
        // Obtener todos los campos y filtrar los de entrada
        let all_fields = field_manager.get_fields_by_position();
        debug!("Found {} total fields in FieldManager", all_fields.len());
        
        for field in all_fields {
            if !field.attributes.protected {
                // Este es un campo de entrada
                let input_field = InputField {
                    name: field.attributes.name.clone(),
                    position: (field.row - 1, field.col - 1), // Convertir a 0-based
                    length: field.attributes.length.unwrap_or(10), // Valor por defecto si no se especifica
                    protected: field.attributes.protected,
                    numeric: field.attributes.numeric,
                };
                
                debug!("Added input field: '{}' at ({},{}) length={}", 
                       input_field.name, input_field.position.0, input_field.position.1, input_field.length);
                self.input_fields.push(input_field);
            }
        }

        // Ordenar campos por posición (fila primero, luego columna)
        self.input_fields.sort_by(|a, b| {
            a.position.0.cmp(&b.position.0)
                .then_with(|| a.position.1.cmp(&b.position.1))
        });

        debug!("Sorted {} input fields by position", self.input_fields.len());

        // Posicionar el cursor en el primer campo si existe
        if !self.input_fields.is_empty() {
            self.current_field_index = Some(0);
            let first_field = &self.input_fields[0];
            // Posicionar el cursor en la primera posición de datos del campo (después del atributo)
            self.cursor_position = (first_field.position.0, first_field.position.1 + 1);
            debug!("Positioned cursor at first input field '{}' - cursor at ({},{})", 
                   first_field.name, self.cursor_position.0, self.cursor_position.1);
        } else {
            self.current_field_index = None;
            self.cursor_position = (0, 0);
            warn!("No input fields found - cursor positioned at origin");
        }

        Ok(())
    }

    /// Encuentra el siguiente campo no protegido (equivalente a next_unprotected de lib3270)
    pub fn find_next_unprotected(&self, from_position: (u16, u16)) -> Option<&InputField> {
        debug!("Finding next unprotected field from position ({},{})", from_position.0, from_position.1);
        
        // Calcular la dirección de buffer para comparación
        let from_addr = from_position.0 * 80 + from_position.1;
        
        // Buscar el primer campo que esté después de la posición actual
        for field in &self.input_fields {
            let field_addr = field.position.0 * 80 + field.position.1;
            if field_addr > from_addr && !field.protected {
                debug!("Found next unprotected field: '{}' at ({},{})", 
                       field.name, field.position.0, field.position.1);
                return Some(field);
            }
        }

        // Si no encontramos ninguno después, buscar desde el principio (wrap around)
        for field in &self.input_fields {
            if !field.protected {
                debug!("Wrapped to first unprotected field: '{}' at ({},{})", 
                       field.name, field.position.0, field.position.1);
                return Some(field);
            }
        }

        warn!("No unprotected fields found");
        None
    }

    /// Navega al siguiente campo (Tab)
    pub fn tab_to_next_field(&mut self) -> Result<(u16, u16), String> {
        debug!("Tabbing to next field from current position ({},{})", 
               self.cursor_position.0, self.cursor_position.1);

        // Encontrar el índice del siguiente campo no protegido
        let from_addr = self.cursor_position.0 * 80 + self.cursor_position.1;
        
        // Buscar el primer campo que esté después de la posición actual
        let mut next_field_index = None;
        for (index, field) in self.input_fields.iter().enumerate() {
            let field_addr = field.position.0 * 80 + field.position.1;
            if field_addr > from_addr && !field.protected {
                next_field_index = Some(index);
                break;
            }
        }
        
        // Si no encontramos ninguno después, buscar desde el principio (wrap around)
        if next_field_index.is_none() {
            for (index, field) in self.input_fields.iter().enumerate() {
                if !field.protected {
                    next_field_index = Some(index);
                    break;
                }
            }
        }

        if let Some(index) = next_field_index {
            let next_field = &self.input_fields[index];
            self.current_field_index = Some(index);
            // Posicionar cursor en la primera posición de datos del campo
            self.cursor_position = (next_field.position.0, next_field.position.1 + 1);
            debug!("Tabbed to field '{}' - cursor at ({},{})", 
                   next_field.name, self.cursor_position.0, self.cursor_position.1);
            Ok(self.cursor_position)
        } else {
            warn!("No next field available for tab navigation");
            Err("No next field available".to_string())
        }
    }

    /// Navega al campo anterior (Shift+Tab / BackTab)
    pub fn backtab_to_previous_field(&mut self) -> Result<(u16, u16), String> {
        debug!("BackTabbing to previous field from current position ({},{})", 
               self.cursor_position.0, self.cursor_position.1);

        let current_addr = self.cursor_position.0 * 80 + self.cursor_position.1;
        
        // Buscar el último campo no protegido que esté antes de la posición actual
        let mut prev_field_index = None;
        for (index, field) in self.input_fields.iter().enumerate().rev() {
            let field_addr = field.position.0 * 80 + field.position.1;
            if field_addr < current_addr && !field.protected {
                prev_field_index = Some(index);
                break;
            }
        }
        
        // Si no encontramos ninguno antes, buscar el último campo (wrap around)
        if prev_field_index.is_none() {
            for (index, field) in self.input_fields.iter().enumerate().rev() {
                if !field.protected {
                    prev_field_index = Some(index);
                    break;
                }
            }
        }

        if let Some(index) = prev_field_index {
            let prev_field = &self.input_fields[index];
            self.current_field_index = Some(index);
            // Posicionar cursor en la primera posición de datos del campo
            self.cursor_position = (prev_field.position.0, prev_field.position.1 + 1);
            debug!("BackTabbed to field '{}' - cursor at ({},{})", 
                   prev_field.name, self.cursor_position.0, self.cursor_position.1);
            Ok(self.cursor_position)
        } else {
            warn!("No previous field available for backtab navigation");
            Err("No previous field available".to_string())
        }
    }

    /// Obtiene el campo actualmente activo
    pub fn get_current_field(&self) -> Option<&InputField> {
        if let Some(index) = self.current_field_index {
            self.input_fields.get(index)
        } else {
            None
        }
    }

    /// Obtiene la posición actual del cursor
    pub fn get_cursor_position(&self) -> (u16, u16) {
        self.cursor_position
    }

    /// Genera los bytes para posicionar el cursor en un campo específico
    pub fn generate_cursor_positioning_bytes(&self, field_name: &str) -> Result<Vec<u8>, String> {
        debug!("Generating cursor positioning bytes for field '{}'", field_name);
        
        if let Some(field) = self.input_fields.iter().find(|f| f.name == field_name) {
            let mut bytes = Vec::new();
            
            // Calcular dirección del cursor (primera posición de datos después del atributo de campo)
            let cursor_row = field.position.0;
            let cursor_col = field.position.1 + 1; // +1 para posicionarse después del atributo de campo
            
            // SBA (Set Buffer Address)
            let addr = cursor_row * 80 + cursor_col;
            let high = ((addr >> 6) & 0x3F) | 0x40;
            let low = (addr & 0x3F) | 0x40;
            
            bytes.push(0x11); // SBA
            bytes.push(high as u8);
            bytes.push(low as u8);
            bytes.push(0x13); // IC (Insert Cursor)
            
            debug!("Generated cursor positioning for field '{}': SBA to ({},{}) + IC", 
                   field_name, cursor_row, cursor_col);
            Ok(bytes)
        } else {
            error!("Field '{}' not found in input fields", field_name);
            Err(format!("Field '{}' not found", field_name))
        }
    }

    /// Valida que el cursor esté en una posición válida de campo de entrada
    pub fn validate_cursor_position(&self) -> bool {
        let cursor_addr = self.cursor_position.0 * 80 + self.cursor_position.1;
        
        for field in &self.input_fields {
            if !field.protected {
                let field_start = field.position.0 * 80 + field.position.1 + 1; // +1 para datos después del atributo
                let field_end = field_start + field.length as u16;
                
                if cursor_addr >= field_start && cursor_addr < field_end {
                    debug!("Cursor position ({},{}) is valid - inside field '{}'", 
                           self.cursor_position.0, self.cursor_position.1, field.name);
                    return true;
                }
            }
        }
        
        warn!("Cursor position ({},{}) is not in any input field", 
              self.cursor_position.0, self.cursor_position.1);
        false
    }

    /// Obtiene estadísticas de navegación para debugging
    pub fn get_navigation_stats(&self) -> NavigationStats {
        NavigationStats {
            total_input_fields: self.input_fields.len(),
            current_field_index: self.current_field_index,
            cursor_position: self.cursor_position,
            current_field_name: self.get_current_field().map(|f| f.name.clone()),
        }
    }

    fn find_next_field_index(&self) -> Option<usize> {
        if let Some(current_index) = self.current_field_index {
            // Buscar el siguiente campo no protegido
            for (index, field) in self.input_fields.iter().enumerate().skip(current_index + 1) {
                if !field.protected {
                    return Some(index);
                }
            }
        }
        None
    }

    fn find_previous_field_index(&self) -> Option<usize> {
        if let Some(current_index) = self.current_field_index {
            // Buscar el campo anterior no protegido
            for (index, field) in self.input_fields.iter().enumerate().rev() {
                if !field.protected && index < current_index {
                    return Some(index);
                }
            }
        }
        None
    }

    pub fn move_to_next_field(&mut self) {
        if let Some(next_index) = self.find_next_field_index() {
            self.current_field_index = Some(next_index);
        }
    }

    pub fn move_to_previous_field(&mut self) {
        if let Some(prev_index) = self.find_previous_field_index() {
            self.current_field_index = Some(prev_index);
        }
    }
}

/// Estadísticas de navegación para debugging
#[derive(Debug)]
pub struct NavigationStats {
    pub total_input_fields: usize,
    pub current_field_index: Option<usize>,
    pub cursor_position: (u16, u16),
    pub current_field_name: Option<String>,
}

impl Default for FieldNavigator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_navigator_creation() {
        let navigator = FieldNavigator::new();
        assert_eq!(navigator.cursor_position, (0, 0));
        assert!(navigator.input_fields.is_empty());
        assert!(navigator.current_field_index.is_none());
    }

    #[test]
    fn test_field_navigation_basic() {
        let mut navigator = FieldNavigator::new();
        
        // Agregar campos de prueba
        navigator.input_fields = vec![
            InputField {
                name: "field1".to_string(),
                position: (5, 10),
                length: 10,
                protected: false,
                numeric: false,
            },
            InputField {
                name: "field2".to_string(),
                position: (10, 15),
                length: 15,
                protected: false,
                numeric: false,
            },
        ];

        // Posicionar en el primer campo
        navigator.current_field_index = Some(0);
        navigator.cursor_position = (5, 11); // Después del atributo

        // Tab al siguiente campo
        let result = navigator.tab_to_next_field();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (10, 16)); // Segundo campo, después del atributo
    }
}
