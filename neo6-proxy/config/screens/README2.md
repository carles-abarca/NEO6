# Lenguaje de Marcado TN3270 v2.0 - Sintaxis con Corchetes

Este documento describe la nueva sintaxis del lenguaje de marcado para pantallas TN3270 basada en corchetes `[TAG]` y `[/TAG]`, que reemplaza la sintaxis anterior basada en `<>`.

## Conceptos Básicos

Las plantillas de pantalla TN3270 v2.0 utilizan un sistema de marcado basado en corchetes que permite definir:
- **Colores**: Diferentes colores de texto según el estándar 3270
- **Posicionamiento**: Ubicación exacta de elementos en la pantalla (fila/columna)
- **Campos de entrada**: Áreas donde el usuario puede introducir datos
- **Atributos especiales**: Intensidad, parpadeo, subrayado

## Sintaxis General

### Etiquetas de Color

Las etiquetas de color envuelven el texto que debe aparecer en un color específico:

```
[COLOR_NAME]Texto a colorear[/COLOR_NAME]
```

#### Colores Disponibles

| Color | Tag de Apertura | Tag de Cierre | Ejemplo |
|-------|-----------------|---------------|---------|
| Azul | `[BLUE]` | `[/BLUE]` | `[BLUE]Mensaje en azul[/BLUE]` |
| Rojo | `[RED]` | `[/RED]` | `[RED]Error crítico[/RED]` |
| Rosa/Magenta | `[PINK]` | `[/PINK]` | `[PINK]Advertencia[/PINK]` |
| Verde | `[GREEN]` | `[/GREEN]` | `[GREEN]Estado: ACTIVO[/GREEN]` |
| Turquesa/Cyan | `[TURQUOISE]` | `[/TURQUOISE]` | `[TURQUOISE]Información[/TURQUOISE]` |
| Amarillo | `[YELLOW]` | `[/YELLOW]` | `[YELLOW]Título Principal[/YELLOW]` |
| Blanco | `[WHITE]` | `[/WHITE]` | `[WHITE]Texto destacado[/WHITE]` |
| Por defecto | `[DEFAULT]` | `[/DEFAULT]` | `[DEFAULT]Texto normal[/DEFAULT]` |

### Etiquetas de Posicionamiento

#### Posicionamiento por Columna

Mueve el cursor a una columna específica en la fila actual:

```
[Xnn]
```

Donde `nn` es un número de 1 a 80.

Ejemplos:
- `[X10]` - Mueve a la columna 10
- `[X40]` - Mueve a la columna 40
- `[X80]` - Mueve a la columna 80

#### Posicionamiento por Fila

Mueve el cursor a una fila específica manteniendo la columna actual:

```
[Ynn]
```

Donde `nn` es un número de 1 a 24.

Ejemplos:
- `[Y5]` - Mueve a la fila 5
- `[Y12]` - Mueve a la fila 12
- `[Y24]` - Mueve a la fila 24

#### Posicionamiento Absoluto

Mueve el cursor a una posición específica (fila, columna):

```
[XYff,cc]
```

Donde:
- `ff` es la fila (1-24)
- `cc` es la columna (1-80)

Ejemplos:
- `[XY5,10]` - Posición fila 5, columna 10
- `[XY1,1]` - Esquina superior izquierda
- `[XY24,80]` - Esquina inferior derecha

### Etiquetas de Campos de Entrada

Los campos de entrada permiten capturar datos del usuario:

```
[FIELD nombre_campo]valor_por_defecto[/FIELD]
```

#### Atributos de Campo

Los campos pueden tener atributos adicionales separados por comas:

```
[FIELD nombre_campo,atributos]valor_por_defecto[/FIELD]
```

##### Atributos Disponibles

| Atributo | Descripción | Ejemplo |
|----------|-------------|---------|
| `length=N` | Longitud máxima del campo | `[FIELD usuario,length=8]admin[/FIELD]` |
| `hidden` | Campo oculto (para contraseñas) | `[FIELD password,hidden][/FIELD]` |
| `numeric` | Solo acepta números | `[FIELD edad,numeric]25[/FIELD]` |
| `uppercase` | Convierte a mayúsculas | `[FIELD codigo,uppercase]ABC[/FIELD]` |
| `protected` | Campo de solo lectura | `[FIELD timestamp,protected]{timestamp}[/FIELD]` |

### Etiquetas de Atributos Especiales

#### Alta Intensidad

```
[BRIGHT]Texto con alta intensidad[/BRIGHT]
```

#### Texto Parpadeante

```
[BLINK]Texto que parpadea[/BLINK]
```

#### Texto Subrayado

```
[UNDERLINE]Texto subrayado[/UNDERLINE]
```

### Combinación de Etiquetas

Las etiquetas se pueden combinar para crear efectos complejos:

```
[XY2,10][YELLOW][BRIGHT]TÍTULO PRINCIPAL[/BRIGHT][/YELLOW]
[XY5,5][GREEN]Estado: [/GREEN][FIELD status,protected]ACTIVO[/FIELD]
```

## Variables del Sistema

Las plantillas pueden incluir variables que se reemplazan dinámicamente (sintaxis sin cambios):

| Variable | Descripción | Ejemplo |
|----------|-------------|---------|
| `{timestamp}` | Fecha y hora actual | `2024-06-05 14:30:25` |
| `{terminal_type}` | Tipo de terminal | `IBM-3278-2-E` |
| `{user_id}` | ID del usuario actual | `ADMIN01` |
| `{session_id}` | ID de la sesión | `SES001` |
| `{system_date}` | Fecha del sistema | `2024-06-05` |
| `{system_time}` | Hora del sistema | `14:30:25` |

## Ejemplo Completo de Plantilla v2.0

```txt
[XY1,1][BLUE]+============================================================================+[/BLUE]
[XY2,1][BLUE]|[/BLUE][XY2,25][YELLOW][BRIGHT]ACCESO TN3270 A ENTORNO NEO6[/BRIGHT][/YELLOW][XY2,80][BLUE]|[/BLUE]
[XY3,1][BLUE]+============================================================================+[/BLUE]
[XY4,1][BLUE]|[/BLUE][XY4,80][BLUE]|[/BLUE]
[XY5,1][BLUE]|[/BLUE]  [WHITE]Bienvenido al Sistema NEO6 - Plataforma de Integracion Mainframe[/WHITE]  [XY5,80][BLUE]|[/BLUE]
[XY6,1][BLUE]|[/BLUE][XY6,80][BLUE]|[/BLUE]
[XY7,1][BLUE]|[/BLUE]  [GREEN]Estado del sistema: ACTIVO[/GREEN][XY7,80][BLUE]|[/BLUE]
[XY8,1][BLUE]|[/BLUE]  [TURQUOISE]Sesion iniciada: [/TURQUOISE][FIELD timestamp,protected]{timestamp}[/FIELD][XY8,80][BLUE]|[/BLUE]
[XY9,1][BLUE]|[/BLUE]  [TURQUOISE]Terminal: [/TURQUOISE][FIELD terminal,protected]{terminal_type}[/FIELD][XY9,80][BLUE]|[/BLUE]
[XY10,1][BLUE]|[/BLUE][XY10,80][BLUE]|[/BLUE]
[XY11,1][BLUE]+============================================================================+[/BLUE]

[XY13,1][WHITE][BRIGHT]Comandos disponibles:[/BRIGHT][/WHITE]
[XY14,3][TURQUOISE]> MENU     - Mostrar menu principal de opciones[/TURQUOISE]
[XY15,3][TURQUOISE]> STATUS   - Ver estado detallado del sistema[/TURQUOISE]
[XY16,3][TURQUOISE]> TRANS    - Ejecutar transaccion CICS[/TURQUOISE]
[XY17,3][TURQUOISE]> HELP     - Mostrar ayuda del sistema[/TURQUOISE]
[XY18,3][TURQUOISE]> EXIT     - Salir del sistema[/TURQUOISE]

[XY21,1][PINK]COMANDO ===> [/PINK][FIELD command,length=20,uppercase][/FIELD]
```

## Comparación con Sintaxis Anterior

### Sintaxis v1.0 (Anterior)
```txt
<pos:5,10><color:yellow><bright>Título</bright></color>
<field:usuario,length=8>admin</field>
```

### Sintaxis v2.0 (Nueva)
```txt
[XY5,10][YELLOW][BRIGHT]Título[/BRIGHT][/YELLOW]
[FIELD usuario,length=8]admin[/FIELD]
```

## Ventajas de la Nueva Sintaxis

1. **Claridad visual**: Los corchetes son más distintivos que los símbolos `<>`
2. **Menos conflictos**: Evita conflictos con contenido HTML/XML
3. **Simplicidad en posicionamiento**: `[X10]`, `[Y5]`, `[XY5,10]` es más intuitivo
4. **Consistencia**: Todos los tags usan la misma estructura `[TAG]...[/TAG]`
5. **Legibilidad**: El código es más fácil de leer y escribir

## Casos de Uso Comunes v2.0

### Pantalla de Login

```txt
[XY8,30][YELLOW][BRIGHT]LOGIN NEO6[/BRIGHT][/YELLOW]
[XY10,25][WHITE]Usuario: [/WHITE][FIELD username,length=8,uppercase][/FIELD]
[XY11,25][WHITE]Password:[/WHITE][FIELD password,length=12,hidden][/FIELD]
[XY13,25][GREEN]Presione ENTER para continuar[/GREEN]
```

### Pantalla de Error

```txt
[XY5,30][RED][BRIGHT][BLINK]*** ERROR ***[/BLINK][/BRIGHT][/RED]
[XY7,10][WHITE]Ha ocurrido un error en el sistema:[/WHITE]
[XY9,10][RED][FIELD error_message,protected]Error desconocido[/FIELD][/RED]
[XY11,10][TURQUOISE]Presione CLEAR para continuar[/TURQUOISE]
```

### Menú Principal

```txt
[XY2,30][YELLOW][BRIGHT]MENU PRINCIPAL[/BRIGHT][/YELLOW]
[XY5,20][WHITE]1. [/WHITE][TURQUOISE]Gestión de Usuarios[/TURQUOISE]
[XY6,20][WHITE]2. [/WHITE][TURQUOISE]Consultas[/TURQUOISE]
[XY7,20][WHITE]3. [/WHITE][TURQUOISE]Reportes[/TURQUOISE]
[XY8,20][WHITE]4. [/WHITE][TURQUOISE]Configuración[/TURQUOISE]
[XY10,20][PINK]Seleccione opción:[/PINK] [FIELD option,length=1,numeric][/FIELD]
```

### Tabla de Datos

```txt
[XY3,1][BLUE]+[/BLUE][XY3,10][BLUE]+[/BLUE][XY3,30][BLUE]+[/BLUE][XY3,50][BLUE]+[/BLUE][XY3,80][BLUE]+[/BLUE]
[XY4,1][BLUE]|[/BLUE][XY4,2][YELLOW][BRIGHT]ID[/BRIGHT][/YELLOW][XY4,10][BLUE]|[/BLUE][XY4,11][YELLOW][BRIGHT]NOMBRE[/BRIGHT][/YELLOW][XY4,30][BLUE]|[/BLUE][XY4,31][YELLOW][BRIGHT]ESTADO[/BRIGHT][/YELLOW][XY4,50][BLUE]|[/BLUE][XY4,51][YELLOW][BRIGHT]FECHA[/BRIGHT][/YELLOW][XY4,80][BLUE]|[/BLUE]
[XY5,1][BLUE]+[/BLUE][XY5,10][BLUE]+[/BLUE][XY5,30][BLUE]+[/BLUE][XY5,50][BLUE]+[/BLUE][XY5,80][BLUE]+[/BLUE]
[XY6,1][BLUE]|[/BLUE][XY6,2][WHITE]001[/WHITE][XY6,10][BLUE]|[/BLUE][XY6,11][WHITE]Juan Pérez[/WHITE][XY6,30][BLUE]|[/BLUE][XY6,31][GREEN]ACTIVO[/GREEN][XY6,50][BLUE]|[/BLUE][XY6,51][WHITE]2024-06-05[/WHITE][XY6,80][BLUE]|[/BLUE]
```

## Reglas de Sintaxis v2.0

### Posicionamiento
- `[Xnn]` - Solo columna (1-80)
- `[Ynn]` - Solo fila (1-24)  
- `[XYff,cc]` - Fila y columna absoluta
- No requieren tags de cierre

### Colores y Atributos
- Todos requieren tag de cierre: `[COLOR]...[/COLOR]`
- Se pueden anidar: `[YELLOW][BRIGHT]texto[/BRIGHT][/YELLOW]`
- Nombres en mayúsculas: `[WHITE]`, `[RED]`, etc.

### Campos
- Siempre requieren cierre: `[FIELD nombre]valor[/FIELD]`
- Atributos separados por comas: `[FIELD nombre,length=10,numeric]`
- Valores por defecto entre tags de apertura y cierre

### Validación
- Las posiciones deben estar en rango válido
- Los tags deben estar balanceados
- Los nombres de color deben ser válidos
- Los atributos de campo deben tener sintaxis correcta

## Migración desde v1.0

### Script de Conversión Automática

```bash
# Convertir posicionamiento
s/<pos:(\d+),(\d+)>/[XY\1,\2]/g

# Convertir colores
s/<color:(\w+)>/[\U\1]/g
s/<\/color>/[\/\U\1]/g

# Convertir atributos
s/<bright>/[BRIGHT]/g
s/<\/bright>/[\/BRIGHT]/g
s/<blink>/[BLINK]/g
s/<\/blink>/[\/BLINK]/g
s/<underline>/[UNDERLINE]/g
s/<\/underline>/[\/UNDERLINE]/g

# Convertir campos
s/<field:([^>]+)>/[FIELD \1]/g
s/<\/field>/[\/FIELD]/g
```

## Implementación Técnica

### Parser v2.0

El nuevo parser debe reconocer:
1. Tags de posicionamiento: `[X\d+]`, `[Y\d+]`, `[XY\d+,\d+]`
2. Tags de color: `[COLOR_NAME]...[/COLOR_NAME]`
3. Tags de atributos: `[BRIGHT]...[/BRIGHT]`, etc.
4. Tags de campo: `[FIELD ...]...[/FIELD]`

### Expresiones Regulares

```rust
// Posicionamiento
let pos_x_regex = Regex::new(r"\[X(\d+)\]")?;
let pos_y_regex = Regex::new(r"\[Y(\d+)\]")?;  
let pos_xy_regex = Regex::new(r"\[XY(\d+),(\d+)\]")?;

// Colores
let color_regex = Regex::new(r"\[(BLUE|RED|PINK|GREEN|TURQUOISE|YELLOW|WHITE|DEFAULT)\](.*?)\[/\1\]")?;

// Atributos
let bright_regex = Regex::new(r"\[BRIGHT\](.*?)\[/BRIGHT\]")?;
let blink_regex = Regex::new(r"\[BLINK\](.*?)\[/BLINK\]")?;
let underline_regex = Regex::new(r"\[UNDERLINE\](.*?)\[/UNDERLINE\]")?;

// Campos
let field_regex = Regex::new(r"\[FIELD ([^\]]+)\](.*?)\[/FIELD\]")?;
```

## Extensiones Futuras v2.0

### Condicionales
```txt
[IF user_is_admin]
  [XY20,1][YELLOW]Opciones de administrador disponibles[/YELLOW]
[/IF]
```

### Loops
```txt
[LOOP items]
  [XY{row},5][WHITE]{item.name}[/WHITE][XY{row},30][GREEN]{item.status}[/GREEN]
[/LOOP]
```

### Includes
```txt
[INCLUDE header.txt]
[INCLUDE footer.txt]
```

### Estilos
```txt
[STYLE title][YELLOW][BRIGHT][UNDERLINE][/STYLE]
[APPLY title]Mi Título[/APPLY]
```

---

*Documentación v2.0 - Diciembre 2024*
*Sintaxis basada en corchetes para mayor claridad y consistencia*
