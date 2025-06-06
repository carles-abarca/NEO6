# Lenguaje de Marcado para Pantallas TN3270

Este documento describe la sintaxis del lenguaje de marcado personalizado para definir pantallas TN3270 con colores, posicionamiento y campos de entrada.

## Conceptos Básicos

Las plantillas de pantalla TN3270 utilizan un sistema de marcado basado en etiquetas que permite definir:
- **Colores**: Diferentes colores de texto según el estándar 3270
- **Posicionamiento**: Ubicación exacta de elementos en la pantalla (fila/columna)
- **Campos de entrada**: Áreas donde el usuario puede introducir datos
- **Atributos especiales**: Intensidad, parpadeo, subrayado

## Sintaxis General

### Etiquetas de Color

Las etiquetas de color envuelven el texto que debe aparecer en un color específico:

```
<color:nombre_color>Texto a colorear</color>
```

#### Colores Disponibles

| Color | Descripción | Ejemplo |
|-------|-------------|---------|
| `default` | Color por defecto del terminal | `<color:default>Texto normal</color>` |
| `blue` | Azul | `<color:blue>Mensaje en azul</color>` |
| `red` | Rojo | `<color:red>Error crítico</color>` |
| `pink` | Rosa/Magenta | `<color:pink>Advertencia</color>` |
| `green` | Verde | `<color:green>Estado: ACTIVO</color>` |
| `turquoise` | Turquesa/Cyan | `<color:turquoise>Información</color>` |
| `yellow` | Amarillo | `<color:yellow>Título Principal</color>` |
| `white` | Blanco | `<color:white>Texto destacado</color>` |

### Etiquetas de Posicionamiento

#### Posicionamiento Absoluto

Coloca el cursor en una posición específica (fila, columna):

```
<pos:fila,columna>
```

El tag `<pos>` mueve el cursor a la posición indicada. No es un tag contenedor y, por lo tanto, no utiliza una etiqueta de cierre `</pos>`.

Ejemplo:
```
<pos:5,10>Este texto aparece en fila 5, columna 10
```

#### Posicionamiento Solo de Columna

Mantiene la fila actual pero cambia la columna:

```
<col:columna>Texto alineado a columna específica</col>
```

Ejemplo:
```
Texto inicial <col:40>Texto alineado a columna 40</col>
```

### Etiquetas de Campos de Entrada

Los campos de entrada permiten capturar datos del usuario:

```
<field:nombre_campo>valor_por_defecto</field>
```

#### Atributos de Campo

Los campos pueden tener atributos adicionales:

```
<field:nombre_campo,atributos>valor_por_defecto</field>
```

##### Atributos Disponibles

| Atributo | Descripción | Ejemplo |
|----------|-------------|---------|
| `length=N` | Longitud máxima del campo | `<field:usuario,length=8>admin</field>` |
| `hidden` | Campo oculto (para contraseñas) | `<field:password,hidden></field>` |
| `numeric` | Solo acepta números | `<field:edad,numeric>25</field>` |
| `uppercase` | Convierte a mayúsculas | `<field:codigo,uppercase>ABC</field>` |
| `protected` | Campo de solo lectura | `<field:timestamp,protected>{timestamp}</field>` |

### Etiquetas de Atributos Especiales

#### Alta Intensidad

```
<bright>Texto con alta intensidad</bright>
```

#### Texto Parpadeante

```
<blink>Texto que parpadea</blink>
```

#### Texto Subrayado

```
<underline>Texto subrayado</underline>
```

### Combinación de Etiquetas

Las etiquetas se pueden combinar para crear efectos complejos:

```
<pos:2,10><color:yellow><bright>TÍTULO PRINCIPAL</bright></color></pos>
<pos:5,5><color:green>Estado: </color><field:status,protected>ACTIVO</field></pos>
```

## Variables del Sistema

Las plantillas pueden incluir variables que se reemplazan dinámicamente:

| Variable | Descripción | Ejemplo |
|----------|-------------|---------|
| `{timestamp}` | Fecha y hora actual | `2024-06-05 14:30:25` |
| `{terminal_type}` | Tipo de terminal | `IBM-3278-2-E` |
| `{user_id}` | ID del usuario actual | `ADMIN01` |
| `{session_id}` | ID de la sesión | `SES001` |
| `{system_date}` | Fecha del sistema | `2024-06-05` |
| `{system_time}` | Hora del sistema | `14:30:25` |

## Ejemplo Completo de Plantilla

```txt
<pos:1,1><color:blue>+============================================================================+</color></pos>
<pos:2,1><color:blue>|</color><pos:2,25><color:yellow><bright>ACCESO TN3270 A ENTORNO NEO6</bright></color></pos><pos:2,80><color:blue>|</color></pos>
<pos:3,1><color:blue>+============================================================================+</color></pos>
<pos:4,1><color:blue>|</color><pos:4,80><color:blue>|</color></pos>
<pos:5,1><color:blue>|</color>  <color:white>Bienvenido al Sistema NEO6 - Plataforma de Integracion Mainframe</color>  <pos:5,80><color:blue>|</color></pos>
<pos:6,1><color:blue>|</color><pos:6,80><color:blue>|</color></pos>
<pos:7,1><color:blue>|</color>  <color:green>Estado del sistema: ACTIVO</color><pos:7,80><color:blue>|</color></pos>
<pos:8,1><color:blue>|</color>  <color:turquoise>Sesion iniciada: </color><field:timestamp,protected>{timestamp}</field><pos:8,80><color:blue>|</color></pos>
<pos:9,1><color:blue>|</color>  <color:turquoise>Terminal: </color><field:terminal,protected>{terminal_type}</field><pos:9,80><color:blue>|</color></pos>
<pos:10,1><color:blue>|</color><pos:10,80><color:blue>|</color></pos>
<pos:11,1><color:blue>+============================================================================+</color></pos>

<pos:13,1><color:white><bright>Comandos disponibles:</bright></color></pos>
<pos:14,3><color:turquoise>> MENU     - Mostrar menu principal de opciones</color></pos>
<pos:15,3><color:turquoise>> STATUS   - Ver estado detallado del sistema</color></pos>
<pos:16,3><color:turquoise>> TRANS    - Ejecutar transaccion CICS</color></pos>
<pos:17,3><color:turquoise>> HELP     - Mostrar ayuda del sistema</color></pos>
<pos:18,3><color:turquoise>> EXIT     - Salir del sistema</color></pos>

<pos:21,1><color:pink>COMANDO ===></color> <field:command,length=20,uppercase></field>
```

## Procesamiento del Marcado

### Orden de Procesamiento

1. **Variables del sistema**: Se reemplazan primero las variables `{variable}`
2. **Posicionamiento**: Se procesan las etiquetas `<pos>` y `<col>`
3. **Campos**: Se crean los campos de entrada `<field>`
4. **Colores y atributos**: Se aplican colores y efectos visuales
5. **Normalización**: Se ajusta el contenido al formato 80x24

### Reglas de Validación

- Las posiciones deben estar dentro del rango válido (1-24 filas, 1-80 columnas)
- Los nombres de colores deben ser válidos según la especificación
- Los campos no pueden solaparse
- El contenido final debe respetar el límite de 1920 caracteres (80x24)

## Casos de Uso Comunes

### Pantalla de Login

```txt
<pos:8,30><color:yellow><bright>LOGIN NEO6</bright></color></pos>
<pos:10,25><color:white>Usuario: </color><field:username,length=8,uppercase></field></pos>
<pos:11,25><color:white>Password:</color><field:password,length=12,hidden></field></pos>
<pos:13,25><color:green>Presione ENTER para continuar</color></pos>
```

### Pantalla de Error

```txt
<pos:5,30><color:red><bright><blink>*** ERROR ***</blink></bright></color></pos>
<pos:7,10><color:white>Ha ocurrido un error en el sistema:</color></pos>
<pos:9,10><color:red><field:error_message,protected>Error desconocido</field></color></pos>
<pos:11,10><color:turquoise>Presione CLEAR para continuar</color></pos>
```

### Menú Principal

```txt
<pos:2,30><color:yellow><bright>MENU PRINCIPAL</bright></color></pos>
<pos:5,20><color:white>1. <color:turquoise>Gestión de Usuarios</color></pos>
<pos:6,20><color:white>2. <color:turquoise>Consultas</color></pos>
<pos:7,20><color:white>3. <color:turquoise>Reportes</color></pos>
<pos:8,20><color:white>4. <color:turquoise>Configuración</color></pos>
<pos:10,20><color:pink>Seleccione opción:</color> <field:option,length=1,numeric></field></pos>
```

## Implementación Técnica

La implementación del parser de marcado se encuentra en:
- `src/tn3270_screens.rs` - Funciones de parsing y procesamiento
- `src/template_parser.rs` - Parser específico del lenguaje de marcado
- `src/field_manager.rs` - Gestión de campos de entrada

## Extensiones Futuras

### Posibles Mejoras

- **Tablas**: Soporte para estructuras tabulares
- **Condicionales**: Mostrar contenido basado en condiciones
- **Loops**: Repetir estructuras para listas dinámicas
- **Includes**: Incluir otras plantillas como módulos
- **Estilos**: Definir estilos reutilizables
- **Validación en tiempo real**: Verificar sintaxis al guardar

### Ejemplo de Extensión Condicional

```txt
<if:user_is_admin>
  <pos:20,1><color:yellow>Opciones de administrador disponibles</color></pos>
</if>
```

---

*Documentación actualizada: Junio 2024*
*Versión del lenguaje: 1.0*
