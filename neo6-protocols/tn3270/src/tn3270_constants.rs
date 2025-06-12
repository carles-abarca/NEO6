// TN3270 protocol constants for field attributes, colors, and control codes
// Use these named constants instead of hardcoded hex values throughout the TN3270 implementation.

// Field Attribute (FA) bits
pub const FA_PRINTABLE: u8 = 0xC0; // Bits that must always be set for printable fields
pub const FA_PROTECT: u8 = 0x20;   // Field is protected (read-only)
pub const FA_NUMERIC: u8 = 0x10;   // Field is numeric only
pub const FA_INT_ZERO_NSEL: u8 = 0x0C; // Invisible field (intensity zero, not selectable)
pub const FA_INT_NORM_SEL: u8 = 0x04;  // Normal intensity, selectable

// Start Field (SF) order
pub const SF: u8 = 0x1D;

// Color codes (EBCDIC)
pub const COLOR_DEFAULT: u8 = 0x00;
pub const COLOR_BLUE: u8 = 0xF1;
pub const COLOR_RED: u8 = 0xF2;
pub const COLOR_PINK: u8 = 0xF3;
pub const COLOR_GREEN: u8 = 0xF4;
pub const COLOR_TURQUOISE: u8 = 0xF5;
pub const COLOR_YELLOW: u8 = 0xF6;
pub const COLOR_WHITE: u8 = 0xF7;

// TN3270 screen constraints
pub const SCREEN_COLUMNS: u16 = 80; // Number of columns per row
pub const SCREEN_ROWS: u16 = 24;   // Number of rows per screen

// Add more TN3270 protocol constants as needed
