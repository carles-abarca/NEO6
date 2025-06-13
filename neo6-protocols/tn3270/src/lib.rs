use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use bytes::BytesMut;
use std::error::Error;
use async_trait::async_trait;
use serde_json::Value;
use serde_json::json;
use neo6_protocols_lib::protocol::{ProtocolHandler, TransactionConfig};
use tracing::{debug, info, warn, trace, error};

// Módulos del protocolo TN3270
pub mod tn3270_screens;
pub mod template_parser;
mod field_manager;
pub mod field_navigation;
pub mod tn3270_constants;
pub mod tn3270_sysvars;

// FFI support
mod ffi;
pub use ffi::*;

// Imports de los módulos
pub use tn3270_screens::ScreenManager;
use template_parser::TemplateParser;
use field_manager::{FieldManager, ScreenField};
use field_navigation::FieldNavigator;

// --- Constantes Telnet y TN3270E ---
// Comandos Telnet
const IAC: u8 = 255; // Interpret As Command
const DONT: u8 = 254;
const DO: u8 = 253;
const WONT: u8 = 252;
const WILL: u8 = 251;
const SB: u8 = 250;   // Subnegotiation Begin
const NOP: u8 = 241;  // No Operation (Timing Mark)
const SE: u8 = 240;   // Subnegotiation End
const EOR_TELNET_CMD: u8 = 239; // Comando Telnet EOR (diferente de la opción EOR)

// Opciones Telnet
const OPT_BINARY: u8 = 0;
const OPT_ECHO: u8 = 1; // No implementado actualmente
const OPT_SUPPRESS_GO_AHEAD: u8 = 3; // No implementado actualmente
const OPT_TERMINAL_TYPE: u8 = 24;
const OPT_EOR: u8 = 25; // Opción End Of Record
const OPT_TN3270E: u8 = 40;

// TN3270E Negotiation Operations (RFC 2355)
// These are the operation codes used in TN3270E subnegotiation messages

// TN3270E Operations (used for both client and server messages)
const TN3270E_OP_CONNECT: u8 = 1;
const TN3270E_OP_DEVICE_TYPE: u8 = 2;
const TN3270E_OP_FUNCTIONS: u8 = 3;
const TN3270E_OP_IS: u8 = 4;
const TN3270E_OP_REJECT: u8 = 6;
const TN3270E_OP_REQUEST: u8 = 7;

// TN3270E Functions (RFC 2355 Section 4.6)
const FUNC_BIND_IMAGE: u8 = 0x00;
const FUNC_DATA_STREAM_CTL: u8 = 0x01;
const FUNC_RESPONSES: u8 = 0x02;
const FUNC_SCS_CTL_CODES: u8 = 0x03;
const FUNC_SYSREQ_FUNC: u8 = 0x04; // Renombrado para evitar colisión con C2S_SYSREQ

// Aliases for consistency with code usage
const TN3270E_FUNC_BIND_IMAGE: u8 = FUNC_BIND_IMAGE;

// TN3270E Data Types (RFC 2355 Section 4.4)
const TN3270E_DATATYPE_3270_DATA: u8 = 0x00;
const TN3270E_DT_BIND_IMAGE: u8 = 0x03;    // BIND-IMAGE data type

// Additional constants
const EOR: u8 = EOR_TELNET_CMD;  // Alias for consistency
// --- Fin Constantes ---

// Estados de negociación Telnet
#[derive(Debug, Default)]
struct TelnetState {
    binary_mode: bool, // True si BINARY ha sido negociado (WILL/DO intercambiados)
    tn3270e_enabled: bool, // True si TN3270E ha sido negociado (WILL/DO intercambiados)
    termtype_negotiated: bool,
    binary_negotiated: bool,
    eor_negotiated: bool,
    tn3270e_negotiated: bool,
    // Indica si todas las negociaciones Telnet base requeridas están completas
    base_telnet_ready: bool,
}

// Estado TN3270E
#[derive(Debug, Default)]
struct TN3270EState {
    // Funciones solicitadas por el servidor en FUNCTIONS REQUEST
    requested_functions_by_server: Vec<u8>,
    // Funciones aceptadas por el cliente en C2S_MESSAGE_FUNCTIONS con REASON_IS
    accepted_functions_by_client: Vec<u8>,
    // bind_image_data: Vec<u8>, // No se usa para el servidor enviando BIND-IMAGE simple
    sequence_number: u16, // Para mensajes TN3270E DATA

    // Flags para seguir el progreso de la negociación TN3270E por parte del servidor
    connect_sent: bool,                     // Servidor envió TN3270E_OP_CONNECT
    device_type_request_sent: bool,         // Servidor envió DEVICE-TYPE REQUEST
    client_device_type_is_received: bool,   // Servidor recibió C2S_MESSAGE_DEVICE_TYPE con REASON_IS
    functions_request_sent: bool,           // Servidor envió FUNCTIONS REQUEST
    client_functions_is_received: bool,     // Servidor recibió C2S_MESSAGE_FUNCTIONS con REASON_IS
    bind_image_sent: bool,                  // Servidor envió S2C_BIND_IMAGE (si fue negociado)
}

// Funciones de conversión EBCDIC manual para evitar bug de truncación
fn ascii_to_ebcdic_byte(ascii: u8) -> u8 {
    match ascii {
        0x00 => 0x00, // NULL
        0x01 => 0x01, // SOH
        0x02 => 0x02, // STX
        0x03 => 0x03, // ETX
        0x04 => 0x37, // EOT
        0x05 => 0x2D, // ENQ
        0x06 => 0x2E, // ACK
        0x07 => 0x2F, // BEL
        0x08 => 0x16, // BS
        0x09 => 0x05, // HT
        0x0A => 0x25, // LF
        0x0B => 0x0B, // VT
        0x0C => 0x0C, // FF
        0x0D => 0x0D, // CR
        0x0E => 0x0E, // SO
        0x0F => 0x0F, // SI
        0x10 => 0x10, // DLE
        0x11 => 0x11, // DC1
        0x12 => 0x12, // DC2
        0x13 => 0x13, // DC3
        0x14 => 0x3C, // DC4
        0x15 => 0x3D, // NAK
        0x16 => 0x32, // SYN
        0x17 => 0x26, // ETB
        0x18 => 0x18, // CAN
        0x19 => 0x19, // EM
        0x1A => 0x3F, // SUB
        0x1B => 0x27, // ESC
        0x1C => 0x1C, // FS
        0x1D => 0x1D, // GS
        0x1E => 0x1E, // RS
        0x1F => 0x1F, // US
        0x20 => 0x40, // SP
        0x21 => 0x5A, // !
        0x22 => 0x7F, // "
        0x23 => 0x7B, // #
        0x24 => 0x5B, // $
        0x25 => 0x6C, // %
        0x26 => 0x50, // &
        0x27 => 0x7D, // '
        0x28 => 0x4D, // (
        0x29 => 0x5D, // )
        0x2A => 0x5C, // *
        0x2B => 0x4E, // +
        0x2C => 0x6B, // ,
        0x2D => 0x60, // -
        0x2E => 0x4B, // .
        0x2F => 0x61, // /
        0x30 => 0xF0, // 0
        0x31 => 0xF1, // 1
        0x32 => 0xF2, // 2
        0x33 => 0xF3, // 3
        0x34 => 0xF4, // 4
        0x35 => 0xF5, // 5
        0x36 => 0xF6, // 6
        0x37 => 0xF7, // 7
        0x38 => 0xF8, // 8
        0x39 => 0xF9, // 9
        0x3A => 0x7A, // :
        0x3B => 0x5E, // ;
        0x3C => 0x4C, // <
        0x3D => 0x7E, // =
        0x3E => 0x6E, // >
        0x3F => 0x6F, // ?
        0x40 => 0x7C, // @
        0x41 => 0xC1, // A
        0x42 => 0xC2, // B
        0x43 => 0xC3, // C
        0x44 => 0xC4, // D
        0x45 => 0xC5, // E
        0x46 => 0xC6, // F
        0x47 => 0xC7, // G
        0x48 => 0xC8, // H
        0x49 => 0xC9, // I
        0x4A => 0xD1, // J
        0x4B => 0xD2, // K
        0x4C => 0xD3, // L
        0x4D => 0xD4, // M
        0x4E => 0xD5, // N
        0x4F => 0xD6, // O
        0x50 => 0xD7, // P
        0x51 => 0xD8, // Q
        0x52 => 0xD9, // R
        0x53 => 0xE2, // S
        0x54 => 0xE3, // T
        0x55 => 0xE4, // U
        0x56 => 0xE5, // V
        0x57 => 0xE6, // W
        0x58 => 0xE7, // X
        0x59 => 0xE8, // Y
        0x5A => 0xE9, // Z
        0x5B => 0xAD, // [
        0x5C => 0xE0, // \
        0x5D => 0xBD, // ]
        0x5E => 0x5F, // ^
        0x5F => 0x6D, // _
        0x60 => 0x79, // `
        0x61 => 0x81, // a
        0x62 => 0x82, // b
        0x63 => 0x83, // c
        0x64 => 0x84, // d
        0x65 => 0x85, // e
        0x66 => 0x86, // f
        0x67 => 0x87, // g
        0x68 => 0x88, // h
        0x69 => 0x89, // i
        0x6A => 0x91, // j
        0x6B => 0x92, // k
        0x6C => 0x93, // l
        0x6D => 0x94, // m
        0x6E => 0x95, // n
        0x6F => 0x96, // o
        0x70 => 0x97, // p
        0x71 => 0x98, // q
        0x72 => 0x99, // r
        0x73 => 0xA2, // s
        0x74 => 0xA3, // t
        0x75 => 0xA4, // u
        0x76 => 0xA5, // v
        0x77 => 0xA6, // w
        0x78 => 0xA7, // x
        0x79 => 0xA8, // y
        0x7A => 0xA9, // z
        0x7B => 0xC0, // {
        0x7C => 0x4F, // |
        0x7D => 0xD0, // }
        0x7E => 0xA1, // ~
        0x7F => 0x07, // DEL
        _ => 0x40,    // Default to space for unknown characters
    }
}

fn ebcdic_to_ascii_byte(ebcdic: u8) -> u8 {
    match ebcdic {
        0x00 => 0x00, // NULL
        0x01 => 0x01, // SOH
        0x02 => 0x02, // STX
        0x03 => 0x03, // ETX
        0x05 => 0x09, // HT
        0x07 => 0x7F, // DEL
        0x0B => 0x0B, // VT
        0x0C => 0x0C, // FF
        0x0D => 0x0D, // CR
        0x0E => 0x0E, // SO
        0x0F => 0x0F, // SI
        0x10 => 0x10, // DLE
        0x11 => 0x11, // DC1
        0x12 => 0x12, // DC2
        0x13 => 0x13, // DC3
        0x16 => 0x08, // BS
        0x18 => 0x18, // CAN
        0x19 => 0x19, // EM
        0x1C => 0x1C, // FS
        0x1D => 0x1D, // GS
        0x1E => 0x1E, // RS
        0x1F => 0x1F, // US
        0x25 => 0x0A, // LF
        0x26 => 0x17, // ETB
        0x27 => 0x1B, // ESC
        0x2D => 0x05, // ENQ
        0x2E => 0x06, // ACK
        0x2F => 0x07, // BEL
        0x32 => 0x16, // SYN
        0x37 => 0x04, // EOT
        0x3C => 0x14, // DC4
        0x3D => 0x15, // NAK
        0x3F => 0x1A, // SUB
        0x40 => 0x20, // SP
        0x4B => 0x2E, // .
        0x4C => 0x3C, // <
        0x4D => 0x28, // (
        0x4E => 0x2B, // +
        0x4F => 0x7C, // |
        0x50 => 0x26, // &
        0x5A => 0x21, // !
        0x5B => 0x24, // $
        0x5C => 0x2A, // *
        0x5D => 0x29, // )
        0x5E => 0x3B, // ;
        0x5F => 0x5E, // ^
        0x60 => 0x2D, // -
        0x61 => 0x2F, // /
        0x6B => 0x2C, // ,
        0x6C => 0x25, // %
        0x6D => 0x5F, // _
        0x6E => 0x3E, // >
        0x6F => 0x3F, // ?
        0x79 => 0x60, // `
        0x7A => 0x3A, // :
        0x7B => 0x23, // #
        0x7C => 0x40, // @
        0x7D => 0x27, // '
        0x7E => 0x3D, // =
        0x7F => 0x22, // "
        0x81 => 0x61, // a
        0x82 => 0x62, // b
        0x83 => 0x63, // c
        0x84 => 0x64, // d
        0x85 => 0x65, // e
        0x86 => 0x66, // f
        0x87 => 0x67, // g
        0x88 => 0x68, // h
        0x89 => 0x69, // i
        0x91 => 0x6A, // j
        0x92 => 0x6B, // k
        0x93 => 0x6C, // l
        0x94 => 0x6D, // m
        0x95 => 0x6E, // n
        0x96 => 0x6F, // o
        0x97 => 0x70, // p
        0x98 => 0x71, // q
        0x99 => 0x72, // r
        0xA1 => 0x7E, // ~
        0xA2 => 0x73, // s
        0xA3 => 0x74, // t
        0xA4 => 0x75, // u
        0xA5 => 0x76, // v
        0xA6 => 0x77, // w
        0xA7 => 0x78, // x
        0xA8 => 0x79, // y
        0xA9 => 0x7A, // z
        0xAD => 0x5B, // [
        0xBD => 0x5D, // ]
        0xC0 => 0x7B, // {
        0xC1 => 0x41, // A
        0xC2 => 0x42, // B
        0xC3 => 0x43, // C
        0xC4 => 0x44, // D
        0xC5 => 0x45, // E
        0xC6 => 0x46, // F
        0xC7 => 0x47, // G
        0xC8 => 0x48, // H
        0xC9 => 0x49, // I
        0xD0 => 0x7D, // }
        0xD1 => 0x4A, // J
        0xD2 => 0x4B, // K
        0xD3 => 0x4C, // L
        0xD4 => 0x4D, // M
        0xD5 => 0x4E, // N
        0xD6 => 0x4F, // O
        0xD7 => 0x50, // P
        0xD8 => 0x51, // Q
        0xD9 => 0x52, // R
        0xE0 => 0x5C, // \
        0xE2 => 0x53, // S
        0xE3 => 0x54, // T
        0xE4 => 0x55, // U
        0xE5 => 0x56, // V
        0xE6 => 0x57, // W
        0xE7 => 0x58, // X
        0xE8 => 0x59, // Y
        0xE9 => 0x5A, // Z
        0xF0 => 0x30, // 0
        0xF1 => 0x31, // 1
        0xF2 => 0x32, // 2
        0xF3 => 0x33, // 3
        0xF4 => 0x34, // 4
        0xF5 => 0x35, // 5
        0xF6 => 0x36, // 6
        0xF7 => 0x37, // 7
        0xF8 => 0x38, // 8
        0xF9 => 0x39, // 9
        _ => 0x20,    // Default to space for unknown characters
    }
}

// Conversión de caracteres
#[derive(Debug)]
pub struct Codec {
    use_ebcdic: bool,
}

impl Codec {
    pub fn new() -> Self {
        Codec {
            use_ebcdic: true,
        }
    }

    pub fn to_host(&self, data: &[u8]) -> Vec<u8> {
        if self.use_ebcdic {
            // Convertir ASCII a EBCDIC para enviar al host
            // Usar nuestra implementación manual para evitar el bug de truncación
            let input_text = String::from_utf8_lossy(data);
            trace!("EBCDIC CONVERSION INPUT: {:?} (len={})", input_text, data.len());
            let result: Vec<u8> = data.iter().map(|&byte| ascii_to_ebcdic_byte(byte)).collect();
            trace!("EBCDIC CONVERSION OUTPUT: {} bytes", result.len());
            trace!("EBCDIC HEX: {:02X?}", result);
            result
        } else {
            data.to_vec()
        }
    }

    pub fn from_host(&self, data: &[u8]) -> Vec<u8> {
        if self.use_ebcdic {
            // Convertir EBCDIC a ASCII al recibir del host
            // Usar nuestra implementación manual para evitar el bug de truncación
            data.iter().map(|&byte| ebcdic_to_ascii_byte(byte)).collect()
        } else {
            data.to_vec()
        }
    }
}

// Sesión de cliente
#[derive(Debug)]
#[allow(dead_code)] // These fields are part of WIP implementation
pub struct Session {
    stream: TcpStream,
    telnet_state: TelnetState,
    tn3270e: TN3270EState,
    codec: Codec,
    screen_manager: ScreenManager,
    field_navigator: FieldNavigator, // For managing field navigation and cursor positioning
    terminal_type: String, // Se llenará durante la negociación
    logical_unit: String, // Nombre de la LU, puede ser asignado por el servidor o negociado
    tn3270e_bound: bool, // True cuando la negociación TN3270E está completa y se puede enviar datos 3270
}

impl Session {
    pub async fn new(stream: TcpStream) -> Self {
        debug!("Session::new() - Iniciando nueva sesión TN3270");
        Session {
            stream,
            telnet_state: TelnetState::default(),
            tn3270e: TN3270EState::default(),
            codec: Codec::new(),
            screen_manager: ScreenManager::new(),
            field_navigator: FieldNavigator::new(),
            terminal_type: String::new(), 
            logical_unit: "NEO6LU".to_string(), // LU por defecto, puede ser sobrescrita por el cliente
            tn3270e_bound: false, // Se pone a true cuando la negociación TN3270E está completa
        }
    }

    // Comprueba si todas las negociaciones Telnet base están listas
    // Esto es un prerrequisito para iniciar la negociación TN3270E.
    fn check_base_telnet_ready(&mut self) {
        debug!("Session::check_base_telnet_ready() - Verificando estado de negociaciones Telnet");
        // Para TN3270E necesitamos tener negociado como mínimo: BINARY, EOR y TN3270E
        // TERMINAL-TYPE es deseable pero algunos clientes pueden no enviarlo hasta después
        let tn3270e_requisites_met = self.telnet_state.binary_negotiated &&
                                     self.telnet_state.eor_negotiated &&
                                     self.telnet_state.tn3270e_negotiated;
                                     
        // Para Telnet clásico 3270 necesitamos tener negociado: BINARY, EOR y TERMINAL-TYPE (sin TN3270E)
        let classic_telnet_requisites_met = self.telnet_state.binary_negotiated && 
                                            self.telnet_state.eor_negotiated &&
                                            self.telnet_state.termtype_negotiated &&
                                            !self.telnet_state.tn3270e_enabled;

        // Si se cumplen los requisitos, marcar como listo
        if !self.telnet_state.base_telnet_ready && (tn3270e_requisites_met || classic_telnet_requisites_met) {
            self.telnet_state.base_telnet_ready = true;
            
            if tn3270e_requisites_met {
                debug!("Negociaciones Telnet básicas para TN3270E completadas");
                debug!("Estado actual: BINARY={}, EOR={}, TN3270E={}, TERMINAL-TYPE={}",
                         self.telnet_state.binary_negotiated, self.telnet_state.eor_negotiated,
                         self.telnet_state.tn3270e_negotiated, self.telnet_state.termtype_negotiated);
            } else {
                debug!("Negociaciones Telnet base completadas. Usando modo Telnet clásico 3270");
            }
        }
    }

    // Inicia la secuencia de negociación TN3270E por parte del servidor.
    // Se llama cuando base_telnet_ready es true y TN3270E está habilitado.
    async fn initiate_tn3270e_server_negotiation(&mut self) -> Result<(), Box<dyn Error>> {
        debug!("Session::initiate_tn3270e_server_negotiation() - Iniciando negociación TN3270E del servidor");
        // Verificar que se cumplan las condiciones para la negociación TN3270E
        if !self.telnet_state.base_telnet_ready || !self.telnet_state.tn3270e_enabled {
            debug!("No se cumplen las condiciones para negociación TN3270E. base_ready={}, tn3270e_enabled={}",
                     self.telnet_state.base_telnet_ready, self.telnet_state.tn3270e_enabled);
            return Ok(());
        }

        // Secuencia de negociación TN3270E iniciada por el servidor:
        // 1. Enviar S2C_CONNECT (si no se ha enviado y no se ha recibido respuesta de DEVICE-TYPE)
        if !self.tn3270e.connect_sent && !self.tn3270e.client_device_type_is_received {
            info!("Iniciando negociación TN3270E: Paso 1 - Enviando DEVICE-TYPE IS");
            
            // Formato correcto según RFC 2355 y c3270:
            // IAC SB TN3270E DEVICE-TYPE IS <terminal-type> CONNECT <lu-name> IAC SE
            let mut connect_msg = vec![IAC, SB, OPT_TN3270E, TN3270E_OP_DEVICE_TYPE, TN3270E_OP_IS];

            // Agregar el tipo de terminal (usamos IBM-3278-2-E que es compatible con c3270)
            connect_msg.extend_from_slice("IBM-3278-2-E".as_bytes());
            
            // Agregar el separador CONNECT y el nombre de LU
            connect_msg.push(TN3270E_OP_CONNECT);
            connect_msg.extend_from_slice(self.logical_unit.as_bytes());
            
            connect_msg.extend_from_slice(&[IAC, SE]);
            
            // Mostrar log detallado del mensaje para diagnosticar problemas
            trace!("TN3270E DEVICE-TYPE IS IBM-3278-2-E CONNECT {} Raw: {:02X?}", self.logical_unit, connect_msg);
            self.stream.write_all(&connect_msg).await?;
            self.stream.flush().await?;
            self.tn3270e.connect_sent = true;
            
            // También registramos este evento para depuración
            debug!("DEVICE-TYPE IS with CONNECT enviado correctamente, esperando respuesta del cliente");
        }

        // 2. Enviar S2C_DEVICE_TYPE_REQUEST (si CONNECT se envió y DEVICE_TYPE_REQUEST no, y no se ha recibido respuesta de DEVICE-TYPE)
        if self.tn3270e.connect_sent && !self.tn3270e.device_type_request_sent && !self.tn3270e.client_device_type_is_received {
            info!("Negociación TN3270E: Paso 2 - Enviando S2C_DEVICE_TYPE_REQUEST");
            
            // Formato según RFC 2355 para DEVICE-TYPE-REQUEST:
            // IAC SB TN3270E DEVICE-TYPE REQUEST <device-type list> IAC SE
            // REQUEST es un sub-comando, no parte del data-type
            
            let mut req_dev_type_msg = vec![IAC, SB, OPT_TN3270E, TN3270E_OP_DEVICE_TYPE, TN3270E_OP_REQUEST];
            
            // Agregar "IBM-3278-2-E" - un tipo compatible con c3270
            req_dev_type_msg.extend_from_slice("IBM-3278-2-E".as_bytes());
                        
            req_dev_type_msg.extend_from_slice(&[IAC, SE]);
            trace!("TN3270E S2C_DEVICE_TYPE_REQUEST ({:02X?})", req_dev_type_msg);
            self.stream.write_all(&req_dev_type_msg).await?;
            self.stream.flush().await?;
            self.tn3270e.device_type_request_sent = true;
            
            debug!("DEVICE-TYPE-REQUEST enviado correctamente");
        }
        
        // Los siguientes pasos (S2C_FUNCTIONS_REQUEST, S2C_BIND_IMAGE) se envían en respuesta
        // a los mensajes del cliente (C2S_MESSAGE_DEVICE_TYPE, C2S_MESSAGE_FUNCTIONS).
        Ok(())
    }

    // Procesa un mensaje TN3270E entrante del cliente.
    // El payload es el contenido de la subnegociación TN3270E, sin IAC SB OPT_TN3270E ... IAC SE.
    async fn process_incoming_tn3270e_message(&mut self, payload: &[u8]) -> Result<(), Box<dyn Error>> {
        debug!("Session::process_incoming_tn3270e_message() - Procesando mensaje TN3270E del cliente (payload size: {})", payload.len());
        if payload.is_empty() {
            warn!("Payload TN3270E vacío");
            return Ok(());
        }

        let tn3270e_cmd = payload[0]; // Este es el data-type del cliente.
        let data = &payload[1..];     // Resto del payload.

        debug!("TN3270E Mensaje del Cliente: cmd=0x{:02X}, data={:02X?}", tn3270e_cmd, data);
        
        // Mostrar la interpretación del comando para mejor diagnóstico
        let cmd_name = match tn3270e_cmd {
            TN3270E_OP_DEVICE_TYPE => "DEVICE-TYPE",
            TN3270E_OP_FUNCTIONS => "FUNCTIONS",
            _ => "UNKNOWN",
        };
        debug!("TN3270E cmd interpretado: {}", cmd_name);

        match tn3270e_cmd {
            TN3270E_OP_DEVICE_TYPE => { // Cliente envía su tipo de dispositivo (0x02)
                if data.is_empty() {
                    warn!("TN3270E_OP_DEVICE_TYPE sin datos de sub-operación");
                    return Ok(());
                }
                let sub_operation = data[0];
                let payload = &data[1..];
                
                // Log más detallado para diagnóstico del payload
                debug!("DEVICE-TYPE sub-operation: 0x{:02X} ({}) - Payload: {:02X?}", 
                         sub_operation, 
                         match sub_operation {
                             TN3270E_OP_IS => "IS",
                             TN3270E_OP_REQUEST => "REQUEST",
                             TN3270E_OP_REJECT => "REJECT",
                             _ => "UNKNOWN"
                         },
                         payload);

                match sub_operation {
                    TN3270E_OP_IS => {
                        // Cliente envía DEVICE-TYPE IS con su tipo de dispositivo seleccionado
                        info!("Negociación TN3270E: Recibido DEVICE-TYPE IS del cliente");
                        
                        // Imprimir los datos raw y en ASCII para diagnóstico
                        let device_info_ascii = String::from_utf8_lossy(payload);
                        trace!("DEVICE-TYPE raw data: {:02X?}", payload);
                        debug!("DEVICE-TYPE ASCII data: '{}'", device_info_ascii);
                        
                        // Usar el TN3270E_OP_CONNECT (0x01) como separador si está presente
                        let mut parts_iter = payload.split(|&byte| byte == TN3270E_OP_CONNECT);
                        let device_type_bytes = parts_iter.next().unwrap_or_default();
                        let resource_name_bytes = parts_iter.next();

                        if !device_type_bytes.is_empty() {
                            self.terminal_type = String::from_utf8_lossy(device_type_bytes).to_string();
                            info!("Cliente seleccionó DEVICE-TYPE: {}", self.terminal_type);
                            // Update ScreenManager context with new terminal type
                            self.screen_manager.set_session_context(
                                self.terminal_type.clone(),
                                self.logical_unit.clone()
                            );
                        }
                        
                        if let Some(res_name_bytes) = resource_name_bytes {
                            if !res_name_bytes.is_empty() {
                                self.logical_unit = String::from_utf8_lossy(res_name_bytes).to_string();
                                info!("Cliente especificó RESOURCE-NAME (LU): {}", self.logical_unit);
                                // Update ScreenManager context with new logical unit
                                self.screen_manager.set_session_context(
                                    self.terminal_type.clone(),
                                    self.logical_unit.clone()
                                );
                            }
                        }
                        
                        self.tn3270e.client_device_type_is_received = true;

                        // Paso 3: Enviar S2C_FUNCTIONS_REQUEST si no se ha enviado
                        if !self.tn3270e.functions_request_sent {
                            info!("Negociación TN3270E: Paso 3 - Enviando S2C_FUNCTIONS_REQUEST.");
                            
                            // Para c3270, empecemos con un conjunto mínimo de funciones
                            self.tn3270e.requested_functions_by_server = vec![
                                FUNC_DATA_STREAM_CTL, // Requerido para mensajes DATA TN3270E
                            ];
                            
                            let mut func_req_msg = vec![IAC, SB, OPT_TN3270E, TN3270E_OP_FUNCTIONS, TN3270E_OP_REQUEST];
                            func_req_msg.extend_from_slice(&self.tn3270e.requested_functions_by_server);
                            func_req_msg.extend_from_slice(&[IAC, SE]);
                            
                            trace!("TN3270E S2C_FUNCTIONS_REQUEST ({:02X?})", func_req_msg);
                            self.stream.write_all(&func_req_msg).await?;
                            self.stream.flush().await?;
                            self.tn3270e.functions_request_sent = true;
                            
                            debug!("FUNCTIONS-REQUEST enviado correctamente");
                        }
                    }
                    TN3270E_OP_REQUEST => {
                        // Cliente envía DEVICE-TYPE REQUEST (poco común, normalmente el servidor envía esto)
                        info!("Recibido DEVICE-TYPE REQUEST del cliente (poco común)");
                        // Responder con IS si es necesario
                    }
                    TN3270E_OP_REJECT => {
                        warn!("Cliente RECHAZÓ DEVICE-TYPE. Payload: {:02X?}", payload);
                        // Manejar rechazo
                    }
                    _ => {
                        warn!("Sub-operación DEVICE-TYPE desconocida: 0x{:02X}", sub_operation);
                    }
                }
            }
            TN3270E_OP_FUNCTIONS => { // Cliente envía sus funciones aceptadas (0x03)
                if data.is_empty() {
                    warn!("TN3270E_OP_FUNCTIONS sin datos de sub-operación.");
                    return Ok(());
                }
                let sub_operation = data[0];
                let payload = &data[1..];
                
                debug!("FUNCTIONS sub-operation: 0x{:02X} ({}) - Payload: {:02X?}", 
                         sub_operation, 
                         match sub_operation {
                             TN3270E_OP_IS => "IS",
                             TN3270E_OP_REQUEST => "REQUEST", 
                             TN3270E_OP_REJECT => "REJECT",
                             _ => "UNKNOWN"
                         },
                         payload);

                match sub_operation {
                    TN3270E_OP_REQUEST => {
                        // Cliente envía FUNCTIONS REQUEST con lista de funciones que quiere
                        info!("Negociación TN3270E: Recibido FUNCTIONS REQUEST del cliente.");
                        info!("Cliente solicita funciones: {:02X?}", payload);
                        
                        // Mostrar las funciones específicas solicitadas para mejor diagnóstico
                        let mut functions_names = Vec::new();
                        for &func in payload {
                            match func {
                                FUNC_BIND_IMAGE => functions_names.push("BIND-IMAGE"),
                                FUNC_DATA_STREAM_CTL => functions_names.push("DATA-STREAM-CTL"),
                                FUNC_RESPONSES => functions_names.push("RESPONSES"),
                                FUNC_SCS_CTL_CODES => functions_names.push("SCS-CTL-CODES"),
                                FUNC_SYSREQ_FUNC => functions_names.push("SYSREQ"),
                                _ => functions_names.push("UNKNOWN"),
                            }
                        }
                        info!("Cliente solicita funciones: {:?} (códigos: {:02X?})", 
                                 functions_names, payload);
                        
                        // Responder con FUNCTIONS IS indicando qué funciones aceptamos
                        // Por ahora, aceptamos todas las funciones solicitadas por el cliente
                        let accepted_functions = payload.to_vec();
                        
                        let mut func_is_msg = vec![IAC, SB, OPT_TN3270E, TN3270E_OP_FUNCTIONS, TN3270E_OP_IS];
                        func_is_msg.extend_from_slice(&accepted_functions);
                        func_is_msg.extend_from_slice(&[IAC, SE]);
                        
                        trace!("TN3270E FUNCTIONS IS ({:02X?})", func_is_msg);
                        self.stream.write_all(&func_is_msg).await?;
                        self.stream.flush().await?;
                        
                        // Verificar si el cliente negoció BIND-IMAGE antes de mover accepted_functions
                        let client_wants_bind_image = accepted_functions.contains(&TN3270E_FUNC_BIND_IMAGE);
                        
                        // Guardar las funciones aceptadas
                        self.tn3270e.accepted_functions_by_client = accepted_functions;
                        self.tn3270e.client_functions_is_received = true;

                        // Enviar BIND-IMAGE si el cliente negoció esta función
                        if client_wants_bind_image {
                            info!("Cliente negoció BIND-IMAGE, enviando comando BIND.");
                            if let Err(e) = self.send_bind_image().await {
                                warn!("Error enviando BIND-IMAGE: {}", e);
                                return Err(e);
                            }
                        }
                        
                        // Marcar la sesión como BOUND después de intercambiar funciones
                        info!("Negociación TN3270E completada. Sesión BOUND.");
                        self.tn3270e_bound = true;
                        self.tn3270e.bind_image_sent = true;
                        
                        // Enviar pantalla inicial inmediatamente después de que la sesión esté bound
                        if let Err(e) = self.maybe_send_screen().await {
                            warn!("Error enviando pantalla inicial: {}", e);
                            return Err(e);
                        }
                        
                        debug!("Sesión TN3270E bound completada, esperando entrada del usuario.");
                    }
                    TN3270E_OP_IS => {
                        // Cliente envía FUNCTIONS IS (respuesta a nuestro FUNCTIONS REQUEST)
                        info!("Negociación TN3270E: Recibido FUNCTIONS IS del cliente.");
                        self.tn3270e.accepted_functions_by_client = payload.to_vec();
                        
                        let mut functions_names = Vec::new();
                        for &func in payload {
                            match func {
                                FUNC_BIND_IMAGE => functions_names.push("BIND-IMAGE"),
                                FUNC_DATA_STREAM_CTL => functions_names.push("DATA-STREAM-CTL"),
                                FUNC_RESPONSES => functions_names.push("RESPONSES"),
                                FUNC_SCS_CTL_CODES => functions_names.push("SCS-CTL-CODES"),
                                FUNC_SYSREQ_FUNC => functions_names.push("SYSREQ"),
                                _ => functions_names.push("UNKNOWN"),
                            }
                        }
                        info!("Cliente aceptó funciones: {:?} (códigos: {:02X?})", 
                                 functions_names, self.tn3270e.accepted_functions_by_client);
                        self.tn3270e.client_functions_is_received = true;

                        // Marcar la sesión como BOUND
                        info!("Negociación TN3270E completada. Sesión BOUND.");
                        self.tn3270e_bound = true;
                        self.tn3270e.bind_image_sent = true;
                        
                        // Enviar pantalla inicial
                        if let Err(e) = self.maybe_send_screen().await {
                            warn!("Error enviando pantalla inicial: {}", e);
                            return Err(e);
                        }
                        
                        debug!(" Sesión TN3270E bound completada, esperando entrada del usuario.");
                    }
                    TN3270E_OP_REJECT => {
                        warn!("Cliente RECHAZÓ FUNCTIONS. Funciones rechazadas: {:02X?}", payload);
                        // Manejar el rechazo según sea necesario
                    }
                    _ => {
                        warn!("Sub-operación FUNCTIONS desconocida: 0x{:02X}", sub_operation);
                    }
                }
            }
            // TODO: Manejar C2S_MESSAGE_DATA, C2S_RESPONSES_REQUEST, C2S_SCS, C2S_SYSREQ si es necesario.
            _ => {
                warn!("Comando TN3270E desconocido del cliente: 0x{:02X}", tn3270e_cmd);
            }
        }
        Ok(())
    }


    async fn maybe_send_screen(&mut self) -> Result<(), Box<dyn Error>> {
        // Solo enviar pantalla si:
        // 1. TN3270E está bound (negociación TN3270E completada exitosamente)
        // O
        // 2. El modo Telnet clásico está completamente negociado Y el tipo de terminal es 3270
        // Y
        // 3. La pantalla no se ha enviado aún.

        let classic_telnet_ready_for_screen = 
            !self.tn3270e_bound && // TN3270E no fue bound exitosamente
            !self.telnet_state.tn3270e_enabled && // TN3270E no está actualmente habilitado
            self.telnet_state.binary_negotiated &&
            self.telnet_state.eor_negotiated &&
            self.telnet_state.termtype_negotiated && self.terminal_type.contains("327");

        // Para TN3270E: si la sesión está bound, ya podemos enviar pantalla
        // independientemente del estado actual de tn3270e_enabled (que puede cambiar después del binding)
        let tn3270e_ready_for_screen = self.tn3270e_bound;

        debug!(" maybe_send_screen - Estado de verificación:");
        debug!("   screen_sent: {}", self.screen_manager.is_screen_sent());
        debug!("   tn3270e_ready_for_screen: {}", tn3270e_ready_for_screen);
        debug!("   classic_telnet_ready_for_screen: {}", classic_telnet_ready_for_screen);
        debug!("   tn3270e_enabled: {}", self.telnet_state.tn3270e_enabled);
        debug!("   tn3270e_bound: {}", self.tn3270e_bound);

        if !self.screen_manager.is_screen_sent() && (tn3270e_ready_for_screen || classic_telnet_ready_for_screen) {
            debug!(" Condiciones cumplidas para enviar pantalla inicial. TN3270E: {}, Bound: {}, Classic: {}",
                self.telnet_state.tn3270e_enabled, self.tn3270e_bound, classic_telnet_ready_for_screen);

            // Generar pantalla de bienvenida usando ScreenManager
            let screen_data = self.screen_manager.generate_welcome_screen()?;

            if tn3270e_ready_for_screen {
                // Formato TN3270E DATA según RFC 2355:
                // Para datos de aplicación 3270, el formato es:
                // <TN3270E-header> <3270-data> IAC EOR
                // donde TN3270E-header = data-type(1) + request-flag(1) + response-flag(1) + seq-number(2)
                
                let mut tn3270e_data_msg = Vec::new();
                
                // TN3270E Header (5 bytes total)
                tn3270e_data_msg.push(TN3270E_DATATYPE_3270_DATA); // data-type = 0x00
                tn3270e_data_msg.push(0x00); // request-flag
                tn3270e_data_msg.push(0x00); // response-flag

                // Número de secuencia (big-endian, 2 bytes)
                let seq_num = self.tn3270e.sequence_number;
                tn3270e_data_msg.push((seq_num >> 8) as u8);
                tn3270e_data_msg.push((seq_num & 0xFF) as u8);
                self.tn3270e.sequence_number = self.tn3270e.sequence_number.wrapping_add(1);

                // Datos 3270 (contenido de la pantalla)
                tn3270e_data_msg.extend_from_slice(&screen_data);

                // Terminar con IAC EOR (no va dentro de una subnegociación)
                tn3270e_data_msg.extend_from_slice(&[IAC, EOR_TELNET_CMD]);

                debug!("[tn3270][SEND] TN3270E DATA (seq: {}, total {} bytes)", seq_num, tn3270e_data_msg.len());
                trace!("[tn3270][SEND] TN3270E Header: {:02X?}", &tn3270e_data_msg[0..5]);
                trace!("[tn3270][SEND] 3270 Data (full): {:02X?}", &tn3270e_data_msg[5..tn3270e_data_msg.len()-2]);
                debug!("[tn3270][SEND] 3270 Command: {:02X} (Erase/Write), WCC: {:02X}", 
                         tn3270e_data_msg[5], tn3270e_data_msg[6]);
                if tn3270e_data_msg.len() > 20 {
                    trace!("[tn3270][SEND] 3270 Data structure: {:02X?}...", &tn3270e_data_msg[7..std::cmp::min(25, tn3270e_data_msg.len())]);
                }
                self.stream.write_all(&tn3270e_data_msg).await?;
                
            } else {
                // Enviar como flujo Telnet clásico con IAC EOR
                let mut classic_data_msg = screen_data;
                classic_data_msg.extend_from_slice(&[IAC, EOR_TELNET_CMD]); // EOR Telnet command
                debug!("[tn3270][SEND] Pantalla inicial como Telnet clásico con EOR ({:02X?})", classic_data_msg);
                self.stream.write_all(&classic_data_msg).await?;
            }
            
            self.stream.flush().await?;
            self.screen_manager.mark_screen_sent();
            debug!(" Pantalla inicial enviada. Manteniendo conexión abierta para entrada del usuario.");
        } else {
            debug!(" NO se enviará pantalla. Razones:");
            debug!("   screen_sent: {}", self.screen_manager.is_screen_sent());
            debug!("   tn3270e_ready: {}", tn3270e_ready_for_screen);
            debug!("   classic_ready: {}", classic_telnet_ready_for_screen);
        }
        Ok(())
    }


    async fn handle_negotiation(&mut self, buf: &[u8]) -> Result<(), Box<dyn Error>> {
        debug!("[tn3270][RECV] handle_negotiation: buffer size={}, data={:02X?}", buf.len(), buf);
        let mut i = 0;
        while i < buf.len() {
            if buf[i] == IAC { // IAC
                if i + 1 >= buf.len() {
                    warn!("IAC al final del buffer, esperando más datos.");
                    // Podríamos necesitar almacenar este IAC y esperar el siguiente byte.
                    // Por ahora, rompemos y esperamos más datos en el buffer.
                    break; 
                }
                let cmd = buf[i+1];
                if cmd == SB { // Subnegociation Begin
                    // Buscar IAC SE para delimitar la subnegociación
                    if let Some(se_idx) = buf[i+2..].windows(2).position(|w| w == [IAC, SE]) {
                        let sub_end_idx = i + 2 + se_idx; // Índice del IAC en IAC SE
                        let sub_data_payload = &buf[i+2..sub_end_idx]; // Datos entre SB y IAC SE
                        debug!(" Subnegociación detectada: Opt=0x{:02X}, Payload: {:02X?}", sub_data_payload[0], &sub_data_payload[1..]);
                        self.handle_subnegotiation(sub_data_payload).await?;
                        i = sub_end_idx + 2; // Avanzar el índice más allá de IAC SE
                        continue;
                    } else {
                        debug!(" Subnegociación incompleta (sin IAC SE), esperando más datos.");
                        // Almacenar datos parciales o simplemente esperar más.
                        break;
                    }
                }
                
                // Primero verificar comandos de 2 bytes (como NOP, EOR)
                if cmd == NOP {
                    debug!(" Recibido IAC NOP (comando Telnet, no necesita respuesta)");
                    // El comando NOP (No Operation) no requiere respuesta, solo continuar
                    i += 2; // Consumir IAC NOP (2 bytes)
                    continue;
                } else if cmd == EOR_TELNET_CMD {
                    debug!(" Recibido IAC EOR (comando Telnet de 2 bytes)");
                    // Esto es una marca de fin de registro en modo Telnet clásico.
                    // El cliente envía esto después de sus datos de entrada.
                    i += 2; // Consumir IAC EOR (2 bytes)
                    continue;
                }
                
                // Comandos Telnet de 3 bytes (IAC CMD OPT)
                if i + 2 >= buf.len() {
                    warn!("Comando Telnet incompleto (IAC CMD sin OPT), esperando más datos.");
                    break;
                }
                let opt = buf[i+2];
                debug!(" Telnet cmd: 0x{:02X} opt: 0x{:02X}", cmd, opt);
                match cmd {
                    WILL => self.handle_will(opt).await?,
                    WONT => self.handle_wont(opt).await?,
                    DO => self.handle_do(opt).await?,
                    DONT => self.handle_dont(opt).await?,
                    _ => {
                        warn!("Comando Telnet desconocido o no manejado: 0x{:02X}", cmd);
                    }
                }
                i += 3; // Avanzar el índice para el comando de 3 bytes
            } else {
                // Datos no Telnet (flujo 3270 o NVT ASCII)
                // Si estamos en modo TN3270E, los datos 3270 deberían venir en mensajes TN3270E DATA.
                // Si estamos en modo Telnet clásico 3270, los datos vienen seguidos de IAC EOR.
                // Si no es ninguno de los dos, son datos NVT.
                let data_chunk = &buf[i..];
                if self.telnet_state.tn3270e_enabled && self.tn3270e_bound {
                    debug!(" *** DATOS POSIBLEMENTE DE USUARIO *** en modo TN3270E bound");
                    debug!(" Tamaño del chunk: {} bytes", data_chunk.len());
                    trace!(" Datos hex: {:02X?}", data_chunk);
                    debug!(" Datos ASCII (ignorando EBCDIC): {}", String::from_utf8_lossy(data_chunk));
                    // En lugar de tratarlos como error, procesarlos como posible entrada del usuario
                    if !data_chunk.is_empty() {
                        debug!(" Enviando datos a send_3270_data...");
                        self.send_3270_data(data_chunk).await?;
                    }
                } else if !self.telnet_state.tn3270e_enabled && self.telnet_state.binary_negotiated && self.telnet_state.eor_negotiated {
                     debug!("[tn3270][RECV] Datos 3270 (modo clásico Telnet) ({:02X?}). Esperando IAC EOR.", data_chunk);
                     // Aquí se acumularían estos datos hasta ver IAC EOR.
                     if !data_chunk.is_empty() {
                         self.send_3270_data(data_chunk).await?;
                     }
                } else {
                    debug!("[tn3270][RECV] Datos NVT/Raw ({:02X?})", data_chunk);
                    // Para datos NVT durante la negociación, simplemente los ignoramos o procesamos como comandos básicos
                }
                i = buf.len(); // Consumir el resto del buffer como datos (simplificación por ahora)
            }
        }

        // Comprobar si las negociaciones base Telnet están listas después de procesar el buffer.
        self.check_base_telnet_ready();
        
        // Mostrar el estado actual para diagnóstico
        debug!(" Estado tras procesar buffer: base_ready={}, tn3270e_enabled={}, binary_negotiated={}, eor_negotiated={}, tn3270e_negotiated={}, connect_sent={}",
                 self.telnet_state.base_telnet_ready, self.telnet_state.tn3270e_enabled,
                 self.telnet_state.binary_negotiated, self.telnet_state.eor_negotiated, 
                 self.telnet_state.tn3270e_negotiated, self.tn3270e.connect_sent);

        // Si las negociaciones Telnet base están listas y TN3270E está habilitado,
        // y aún no hemos iniciado la negociación TN3270E por parte del servidor, hacerlo.
        if self.telnet_state.tn3270e_enabled && 
           self.telnet_state.binary_negotiated && 
           self.telnet_state.eor_negotiated && 
           !self.tn3270e.connect_sent {
            info!("[tn3270][INFO] Condiciones cumplidas para iniciar negociación TN3270E");
            self.initiate_tn3270e_server_negotiation().await?;
        }

        // Intentar enviar la pantalla si las condiciones son adecuadas (podría ser después de que TN3270E se complete)
        self.maybe_send_screen().await?;
        Ok(())
    }

    // Maneja subnegociaciones Telnet (ej. TERMINAL-TYPE, TN3270E)
    // payload = <OPTION> <parameters...>
    async fn handle_subnegotiation(&mut self, payload: &[u8]) -> Result<(), Box<dyn Error>> {
        if payload.is_empty() {
            warn!("Subnegociación con payload vacío.");
            return Ok(());
        }
        let option = payload[0];
        let params = &payload[1..];

        match option {
            OPT_TERMINAL_TYPE => {
                // Cliente envía IAC SB TERMINAL-TYPE IS <type> IAC SE
                // Aquí params = IS <type>
                if params.len() > 1 && params[0] == 0 { // IS = 0
                    let term_type_bytes = &params[1..];
                    self.terminal_type = String::from_utf8_lossy(term_type_bytes).to_string();
                    info!("[tn3270][INFO] Tipo de terminal recibido: {}", self.terminal_type);
                    self.telnet_state.termtype_negotiated = true; // Marcar como negociado
                    // Update ScreenManager context with negotiated terminal type
                    self.screen_manager.set_session_context(
                        self.terminal_type.clone(),
                        self.logical_unit.clone()
                    );
                    // No necesitamos responder a TERMINAL-TYPE IS.
                } else {
                    warn!("Subnegociación TERMINAL-TYPE mal formada o no es 'IS': {:02X?}", params);
                }
            }
            OPT_TN3270E => {
                // Cliente envía un mensaje TN3270E.
                // params = <tn3270e_cmd_or_response> <data...>
                debug!(" Recibida subnegociación TN3270E. Payload: {:02X?}", params);
                self.process_incoming_tn3270e_message(params).await?;
            }
            _ => {
                warn!("Subnegociación para opción desconocida o no manejada: 0x{:02X}", option);
            }
        }
        Ok(())
    }


    async fn handle_will(&mut self, opt: u8) -> Result<(), Box<dyn Error>> {
        debug!("[tn3270][RECV] WILL 0x{:02X}", opt);
        match opt {
            OPT_BINARY => {
                if !self.telnet_state.binary_negotiated { // Solo negociar una vez
                    debug!("[tn3270][SEND] DO OPT_BINARY");
                    self.stream.write_all(&[IAC, DO, OPT_BINARY]).await?;
                    self.telnet_state.binary_mode = true; // El cliente quiere, nosotros aceptamos
                    self.telnet_state.binary_negotiated = true;
                }
            }
            OPT_EOR => {
                if !self.telnet_state.eor_negotiated {
                    debug!("[tn3270][SEND] DO OPT_EOR");
                    self.stream.write_all(&[IAC, DO, OPT_EOR]).await?;
                    self.telnet_state.eor_negotiated = true;
                }
            }
            OPT_TN3270E => {
                if !self.telnet_state.tn3270e_negotiated {
                    debug!("[tn3270][SEND] DO OPT_TN3270E");
                    self.stream.write_all(&[IAC, DO, OPT_TN3270E]).await?;
                    self.telnet_state.tn3270e_enabled = true;
                    self.telnet_state.tn3270e_negotiated = true;
                }
            }
            OPT_TERMINAL_TYPE => {
                 // El cliente dice WILL TERMINAL-TYPE. Nosotros ya enviamos DO TERMINAL-TYPE.
                 // Esto significa que el cliente está de acuerdo en enviar su tipo de terminal.
                 // Respondemos con SB TERMINAL-TYPE SEND IAC SE para solicitarlo.
                 // Sin embargo, la mayoría de los clientes envían su tipo después de WILL sin SEND.
                 // El servidor ya envió DO TERMINAL-TYPE. El cliente responde WILL TERMINAL-TYPE.
                 // Luego el cliente DEBERÍA enviar IAC SB TERMINAL-TYPE IS <type> IAC SE.
                 // No necesitamos hacer nada aquí más que registrarlo, ya que esperamos la subnegociación.
                 // Si quisiéramos ser estrictos, podríamos enviar DONT si no lo queremos.
                 // Pero como ya enviamos DO, este WILL es una confirmación.
                 info!("[tn3270][INFO] Cliente WILL TERMINAL-TYPE. Esperando subnegociación con el tipo.");
                 // No marcamos termtype_negotiated aquí, sino cuando recibimos el tipo en subnegociación.
            }
            OPT_ECHO | OPT_SUPPRESS_GO_AHEAD => {
                // No soportamos ECHO ni SGA activamente, así que respondemos DONT.
                debug!("[tn3270][SEND] DONT 0x{:02X}", opt);
                self.stream.write_all(&[IAC, DONT, opt]).await?;
            }
            _ => {
                debug!("[tn3270][SEND] DONT 0x{:02X} (opción desconocida que el cliente ofrece)", opt);
                self.stream.write_all(&[IAC, DONT, opt]).await?;
            }
        }
        self.check_base_telnet_ready();
        
        // Debug para entender el estado actual de la negociación
        debug!(" Estado tras DO 0x{:02X}: base_ready={}, tn3270e_enabled={}, connect_sent={}, device_type_received={}", 
                 opt, self.telnet_state.base_telnet_ready, self.telnet_state.tn3270e_enabled, 
                 self.tn3270e.connect_sent, self.tn3270e.client_device_type_is_received);
        
        // Forzar la revisión de la negociación TN3270E después de recibir cada comando DO
        if self.telnet_state.tn3270e_enabled && self.telnet_state.binary_negotiated &&            self.telnet_state.eor_negotiated && !self.tn3270e.connect_sent {
            info!("[tn3270][INFO] Iniciando negociación TN3270E después de recibir DO 0x{:02X}", opt);
            self.initiate_tn3270e_server_negotiation().await?;
        }
        
        Ok(())
    }

    async fn handle_wont(&mut self, opt: u8) -> Result<(), Box<dyn Error>> {
        debug!("[tn3270][RECV] WONT 0x{:02X}", opt);
        match opt {
            OPT_BINARY => {
                warn!("Cliente WONT BINARY. Esto puede causar problemas.");
                self.telnet_state.binary_mode = false;
                self.telnet_state.binary_negotiated = true; // La negociación (fallida) ha terminado.
                // Respondemos DONT para confirmar que no lo usaremos.
                self.stream.write_all(&[IAC, DONT, OPT_BINARY]).await?;
            }
            OPT_EOR => {
                warn!("Cliente WONT EOR. Necesario para modo Telnet clásico.");
                self.telnet_state.eor_negotiated = true; // Negociación terminada.
                self.stream.write_all(&[IAC, DONT, OPT_EOR]).await?;
            }
            OPT_TN3270E => {
                info!("[tn3270][INFO] Cliente WONT TN3270E. Se usará Telnet clásico si es posible.");
                self.telnet_state.tn3270e_enabled = false;
                self.telnet_state.tn3270e_negotiated = true; // Negociación terminada.
                // Respondemos DONT para confirmar.
                self.stream.write_all(&[IAC, DONT, OPT_TN3270E]).await?;
            }
            OPT_TERMINAL_TYPE => {
                info!("[tn3270][INFO] Cliente WONT TERMINAL-TYPE.");
                // No podemos obtener el tipo de terminal.
                self.telnet_state.termtype_negotiated = true; // Negociación terminada.
                self.stream.write_all(&[IAC, DONT, OPT_TERMINAL_TYPE]).await?;
            }
            // Otras opciones que podríamos haber solicitado con WILL y el cliente rechaza con WONT.
            _ => {
                info!("[tn3270][INFO] Cliente WONT opción 0x{:02X}. Confirmando con DONT.", opt);
                self.stream.write_all(&[IAC, DONT, opt]).await?;
            }
        }
        self.check_base_telnet_ready();
        // Si TN3270E fue rechazado, no intentar iniciar su negociación.
        // La lógica en check_base_telnet_ready y initiate_tn3270e_server_negotiation se encarga de esto.
        Ok(())
    }

    async fn handle_do(&mut self, opt: u8) -> Result<(), Box<dyn Error>> {
        debug!("[tn3270][RECV] DO 0x{:02X}", opt);
        match opt {
            OPT_TERMINAL_TYPE => {
                // Cliente nos pide (DO) que habilitemos TERMINAL-TYPE.
                // Esto significa que el cliente quiere que nosotros solicitemos su tipo de terminal.
                // Respondemos WILL TERMINAL-TYPE para indicar que lo haremos (o que ya lo hicimos).
                // Y luego enviamos IAC SB TERMINAL-TYPE SEND IAC SE.
                // Sin embargo, nuestro flujo es: Server envía DO TERMINAL-TYPE. Cliente responde WILL.
                // Si el cliente envía DO TERMINAL-TYPE, es un poco inusual si nosotros no enviamos WILL primero.
                // Pero si lo hace, aceptamos y solicitamos.
                if !self.telnet_state.termtype_negotiated { // Solo si no hemos negociado ya
                    debug!("[tn3270][SEND] WILL OPT_TERMINAL_TYPE");
                    self.stream.write_all(&[IAC, WILL, OPT_TERMINAL_TYPE]).await?;
                    // Ahora solicitamos el tipo de terminal
                    debug!("[tn3270][SEND] SB TERMINAL-TYPE SEND");
                    self.stream.write_all(&[IAC, SB, OPT_TERMINAL_TYPE, 1, IAC, SE]).await?; // 1 = SEND
                }
            }
            OPT_BINARY => {
                // Cliente solicita que operemos en modo binario (DO BINARY)
                if !self.telnet_state.binary_negotiated {
                    debug!("[tn3270][SEND] WILL OPT_BINARY");
                    self.stream.write_all(&[IAC, WILL, OPT_BINARY]).await?;
                    self.telnet_state.binary_mode = true;
                    self.telnet_state.binary_negotiated = true;
                } else if !self.telnet_state.binary_mode {
                    // Ya negociado, pero estaba apagado, y cliente pide DO
                    debug!("[tn3270][SEND] WILL OPT_BINARY (reactivando)");
                    self.stream.write_all(&[IAC, WILL, OPT_BINARY]).await?;
                    self.telnet_state.binary_mode = true;
                }
            }
            OPT_EOR => {
                // Cliente solicita que soportemos EOR (DO EOR)
                if !self.telnet_state.eor_negotiated {
                    debug!("[tn3270][SEND] WILL OPT_EOR");
                    self.stream.write_all(&[IAC, WILL, OPT_EOR]).await?;
                    self.telnet_state.eor_negotiated = true;
                }
            }
            OPT_TN3270E => {
                // Cliente solicita que habilitemos TN3270E (DO TN3270E)
                if !self.telnet_state.tn3270e_negotiated {
                    debug!("[tn3270][SEND] WILL OPT_TN3270E");
                    self.stream.write_all(&[IAC, WILL, OPT_TN3270E]).await?;
                    self.telnet_state.tn3270e_enabled = true;
                    self.telnet_state.tn3270e_negotiated = true;
                    
                    // Intentar iniciar la negociación TN3270E si también tenemos BINARY y EOR
                    if self.telnet_state.binary_negotiated && self.telnet_state.eor_negotiated {
                        info!("[tn3270][INFO] Requisitos básicos para TN3270E cumplidos, iniciando negociación");
                        // Forzar base_telnet_ready a true si se cumplen los requisitos mínimos
                        self.telnet_state.base_telnet_ready = true;
                        self.initiate_tn3270e_server_negotiation().await?;
                    }
                } else if !self.telnet_state.tn3270e_enabled {
                    // Ya negociado, pero estaba apagado, y cliente pide DO
                    debug!("[tn3270][SEND] WILL OPT_TN3270E (reactivando)");
                    self.stream.write_all(&[IAC, WILL, OPT_TN3270E]).await?;
                    self.telnet_state.tn3270e_enabled = true;
                    
                    // También intenta iniciar negociación TN3270E al reactivar
                    if self.telnet_state.binary_negotiated && self.telnet_state.eor_negotiated {
                        self.telnet_state.base_telnet_ready = true;
                        self.initiate_tn3270e_server_negotiation().await?;
                    }
                }
            }
            OPT_ECHO | OPT_SUPPRESS_GO_AHEAD => {
                // No queremos que el cliente habilite estas opciones para nosotros. Respondemos WONT.
                debug!("[tn3270][SEND] WONT 0x{:02X}", opt);
                self.stream.write_all(&[IAC, WONT, opt]).await?;
            }
            _ => {
                debug!("[tn3270][SEND] WONT 0x{:02X} (opción desconocida que el cliente nos pide habilitar)", opt);
                self.stream.write_all(&[IAC, WONT, opt]).await?;
            }
        }
        self.check_base_telnet_ready();
        if self.telnet_state.base_telnet_ready && self.telnet_state.tn3270e_enabled && 
           !self.tn3270e.connect_sent && !self.tn3270e.client_device_type_is_received {
            self.initiate_tn3270e_server_negotiation().await?;
        }
        Ok(())
    }

    async fn handle_dont(&mut self, opt: u8) -> Result<(), Box<dyn Error>> {
        debug!("[tn3270][RECV] DONT 0x{:02X}", opt);
        match opt {
            OPT_TERMINAL_TYPE | OPT_BINARY | OPT_EOR | OPT_TN3270E => {
                // Cliente nos pide (DONT) que deshabilitemos una opción que podríamos haber ofrecido (WILL).
                // O confirma nuestro WONT.
                info!("[tn3270][INFO] Cliente DONT 0x{:02X}. Confirmando con WONT.", opt);
                self.stream.write_all(&[IAC, WONT, opt]).await?;
                if opt == OPT_BINARY { self.telnet_state.binary_mode = false; self.telnet_state.binary_negotiated = true; }
                if opt == OPT_EOR { self.telnet_state.eor_negotiated = true; } // Marcar como negociada (a off)
                if opt == OPT_TN3270E { self.telnet_state.tn3270e_enabled = false; self.telnet_state.tn3270e_negotiated = true; }
                if opt == OPT_TERMINAL_TYPE { self.telnet_state.termtype_negotiated = true; }
            }
            _ => {
                // Para otras opciones, simplemente confirmamos con WONT.
                self.stream.write_all(&[IAC, WONT, opt]).await?;
            }
        }
        self.check_base_telnet_ready();
        // Si TN3270E fue deshabilitado, no intentar iniciar su negociación.
        Ok(())
    }

    // Envía un comando BIND-IMAGE TN3270E cuando el cliente negoció esta función
    async fn send_bind_image(&mut self) -> Result<(), Box<dyn Error>> {
        // Crear un BIND básico según la especificación TN3270E
        // Referencia: RFC 2355 y código c3270 telnet.c
        let bind_data = vec![
            0x31,  // BIND RU type
            0x00, 0x02, 0x88, 0x00, 0x10, 0x02, 0x85, 0x00,  // BIND fields básicos
            0x00, 0x00, 0x07, 0x85, 0x00, 0x01, 0x02, 0x00,  // Más campos BIND
            0x00, 0x02, 0x81, 0x87, 0x02, 0x00, 0x02, 0x00,  // Configuración de pantalla
            0x18, 0x50,  // Rows(24=0x18), Cols(80=0x50) en formato estándar
        ];

        // Crear header TN3270E para BIND-IMAGE
        let mut tn3270e_header = vec![
            TN3270E_DT_BIND_IMAGE,  // data_type = BIND-IMAGE
            0x00,                   // request_flag = 0
            0x00,                   // response_flag = 0
            0x00, 0x00             // seq_number = 0
        ];

        // Combinar header + datos BIND

        tn3270e_header.extend_from_slice(&bind_data);
        
        
        // Agregar IAC EOR
        tn3270e_header.push(IAC);
        tn3270e_header.push(EOR);

        info!("[tn3270][INFO] Enviando TN3270E BIND-IMAGE comando ({} bytes)", tn3270e_header.len());
        debug!(" BIND-IMAGE data: {:02X?}", tn3270e_header);

        self.stream.write_all(&tn3270e_header).await?;
        self.stream.flush().await?;

        info!("[tn3270][INFO] TN3270E BIND-IMAGE comando enviado exitosamente");
        Ok(())
    }

    // Esta función ya no es necesaria aquí, el flujo es más dinámico.
    // async fn send_tn3270e_device_type_request(&mut self) -> Result<(), Box<dyn Error>> { ... }

    // Función principal para procesar datos de entrada del cliente (AID, campo de datos, etc.)
    async fn send_3270_data(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {

        if data.is_empty() {
            debug!(" send_3270_data: datos vacíos recibidos");
            return Ok(());
        }

        warn!("*** PROCESANDO DATOS DE USUARIO DE MOCHA ***");
        warn!("Tamaño de datos: {} bytes", data.len());
        warn!("Datos hex completos: {:02X?}", data);
        warn!("Datos como string ASCII: {}", String::from_utf8_lossy(data));
        
        // Analizar cada byte individualmente para debug
        for (i, &byte) in data.iter().enumerate() {
            warn!("Byte[{}]: 0x{:02X} ({}) '{}'", 
                i, byte, byte, 
                if byte.is_ascii_graphic() || byte == b' ' { 
                    char::from(byte) 
                } else { 
                    '.' 
                });
        }

        info!("[tn3270][INFO] send_3270_data: procesando {} bytes de entrada del cliente", data.len());
        debug!(" Datos completos recibidos: {:02X?}", data);
        
        // Para TN3270E, los datos incluyen un header de 5 bytes seguido del stream 3270 real
        // Header TN3270E: [00, 00, 00, 00, 00] seguido de AID y datos
        let actual_data = if data.len() >= 6 && data[0] == 0x00 && data[1] == 0x00 && 
                            data[2] == 0x00 && data[3] == 0x00 && data[4] == 0x00 {
            debug!(" Detectado header TN3270E, saltando 5 bytes");
            &data[5..] // Saltar el header TN3270E
        } else {
            debug!(" Datos 3270 sin header TN3270E");
            data // Usar datos tal como están
        };

        if actual_data.is_empty() {
            debug!(" send_3270_data: datos 3270 vacíos después de procesar header");
            return Ok(());
        }
        
        // Primero verificar si tenemos datos de teclado especiales (Tab, BackTab)
        // Estos pueden aparecer como datos EBCDIC sin AID
        if self.contains_tab_or_backtab_input(actual_data) {
            info!("[tn3270][INFO] Detectado Tab/BackTab en entrada de teclado");
            return self.process_keyboard_navigation(actual_data).await;
        }
        
        // Analizar el primer byte del stream 3270 para identificar el AID (Attention Identifier)
        let aid_byte = actual_data[0];
        debug!(" AID recibido: 0x{:02X}", aid_byte);
        
        // Si el AID es 0x00, puede indicar datos sin AID válido o problemas de parsing
        if aid_byte == 0x00 {
            warn!("AID 0x00 recibido - posible problema de formato o entrada inválida");
            debug!(" Intentando interpretar datos como texto plano...");
            
            // Intentar extraer texto directamente de los datos
            if actual_data.len() > 1 {
                let text_data = String::from_utf8_lossy(&actual_data[1..]).trim().to_string();
                if !text_data.is_empty() {
                    info!("[tn3270][INFO] Texto extraído: '{}'", text_data);
                    // Procesar como comando directo
                    return self.process_text_command(&text_data).await;
                }
            }
            
            // Si no hay texto útil, enviar mensaje de error
            let error_msg = "Entrada no válida - verifique el formato";
            let error_screen = self.screen_manager.generate_error_screen(&error_msg)?;
            self.send_screen_data(error_screen).await?;
            return Ok(());
        }

        match aid_byte {
            0x60 => {
                // AID_NO (0x60) - no AID generated
                info!("[tn3270][INFO] Procesando AID_NO - confirmación sin acción específica");
                self.process_no_aid().await?;
            },
            0x7D => {
                // AID_ENTER (0x7D) - tecla Enter
                info!("[tn3270][INFO] Procesando AID_ENTER");
                self.process_enter_aid(actual_data).await?;
            },
            0x6C => {
                // AID_PA1 (0x6C) - tecla PA1
                info!("[tn3270][INFO] Procesando AID_PA1");
                self.process_pa_aid("PA1").await?;
            },
            0x6D => {
                // AID_CLEAR (0x6D) - tecla Clear
                info!("[tn3270][INFO] Procesando AID_CLEAR");
                self.process_clear_aid().await?;
            },
            0x6E => {
                // AID_PA2 (0x6E) - tecla PA2
                info!("[tn3270][INFO] Procesando AID_PA2");
                self.process_pa_aid("PA2").await?;
            },
            0x6B => {
                // AID_PA3 (0x6B) - tecla PA3
                info!("[tn3270][INFO] Procesando AID_PA3");
                self.process_pa_aid("PA3").await?;
            }
            0xF1..=0xFC => {
                // AID_PF1 through AID_PF24 - teclas de función
                let pf_num = match aid_byte {
                    0xF1 => 1, 0xF2 => 2, 0xF3 => 3, 0xF4 => 4, 0xF5 => 5,
                    0xF6 => 6, 0xF7 => 7, 0xF8 => 8, 0xF9 => 9, 0x7A => 10,
                    0x7B => 11, 0x7C => 12, 0xC1 => 13, 0xC2 => 14, 0xC3 => 15,
                    0xC4 => 16, 0xC5 => 17, 0xC6 => 18, 0xC7 => 19, 0xC8 => 20,
                    0xC9 => 21, 0x4A => 22, 0x4B => 23, 0x4C => 24,
                    _ => 0,
                };
                info!("[tn3270][INFO] Procesando AID_PF{}", pf_num);
                self.process_pf_aid(pf_num).await?;
            },
            _ => {
                warn!("AID desconocido: 0x{:02X}, procesando como entrada genérica", aid_byte);
                self.process_unknown_aid(actual_data).await?;
            }
        }

        Ok(())
    }

    // Procesa la tecla Enter - extrae datos de campos y navega según el input
    async fn process_enter_aid(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        debug!(" process_enter_aid: analizando datos de campo");
        
        if data.len() < 3 {
            warn!("Datos insuficientes para procesar Enter");
            return Ok(());
        }

        // Los bytes 1-2 contienen la posición del cursor (dirección de buffer)
        let cursor_high = data[1];
        let cursor_low = data[2];
        let cursor_addr = ((cursor_high & 0x3F) << 6) | (cursor_low & 0x3F);
        debug!(" Posición del cursor: {}", cursor_addr);

        // Extraer datos de campo del stream TN3270
        let field_data = self.extract_field_data(&data[3..]).await?;
        debug!(" Datos de campo extraídos: {:?}", field_data);

        // Si tenemos datos de campo, usar el primer valor como nombre de pantalla
        if let Some(first_value) = field_data.first() {
            if !first_value.trim().is_empty() {
                let screen_name = first_value.trim().to_uppercase();
                info!("[tn3270][INFO] Navegando a pantalla: {}", screen_name);
                
                // IMPORTANTE: Resetear el estado de pantalla antes de generar nueva pantalla
                self.screen_manager.reset_screen_sent();
                
                // Intentar generar la pantalla solicitada
                let screen_result = self.screen_manager.generate_screen_by_name(&screen_name);
                match screen_result {
                    Ok(screen_data) => {
                        self.send_screen_data(screen_data).await?;
                        return Ok(());
                    },
                    Err(_) => {
                        warn!("Error generando pantalla '{}'", screen_name);
                        // Resetear estado también para pantalla de error
                        self.screen_manager.reset_screen_sent();
                        // Enviar pantalla de error
                        let error_msg = format!("Pantalla '{}' no encontrada", screen_name);
                        let error_screen = self.screen_manager.generate_error_screen(&error_msg)?;
                        self.send_screen_data(error_screen).await?;
                        return Ok(());
                    }
                }
            }
        }

        // Si no hay entrada válida, verificar si estamos en una pantalla de error
        // En ese caso, volver a la pantalla de bienvenida en lugar de reenviar la misma pantalla de error
        info!("[tn3270][INFO] Sin entrada válida");
        
        // Comprobar si estamos en una pantalla de error (buscar indicadores típicos)
        // Si el buffer actual contiene texto de error, volver a welcome en lugar de reenviar
        let current_buffer = self.screen_manager.get_screen_buffer();
        let buffer_str = String::from_utf8_lossy(&current_buffer).to_lowercase();
        let buffer_contains_error = current_buffer.len() > 50 && 
            (buffer_str.contains("error") || 
             buffer_str.contains("no encontrada") ||
             buffer_str.contains("*** error ***") ||
             buffer_str.contains("pantalla") && buffer_str.contains("encontrada"));
        
        if buffer_contains_error {
            info!("[tn3270][INFO] Detectada pantalla de error, volviendo a pantalla de bienvenida");
            self.screen_manager.reset_screen_sent();
            let welcome_screen = self.screen_manager.generate_welcome_screen()?;
            self.send_screen_data(welcome_screen).await?;
        } else {
            info!("[tn3270][INFO] Reenviando pantalla actual");
            self.screen_manager.reset_screen_sent();
            self.maybe_send_screen().await?;
        }
        Ok(())
    }

    // Procesa la tecla Clear - limpia la pantalla y envía pantalla de bienvenida
    async fn process_clear_aid(&mut self) -> Result<(), Box<dyn Error>> {
        info!("[tn3270][INFO] Clear recibido, enviando pantalla de bienvenida");
        let welcome_screen = self.screen_manager.generate_welcome_screen()?;
        self.send_screen_data(welcome_screen).await?;
        Ok(())
    }

    // Procesa teclas PA (Program Attention) - usualmente no modifican datos
    async fn process_pa_aid(&mut self, pa_name: &str) -> Result<(), Box<dyn Error>> {
        info!("[tn3270][INFO] {} recibido, sin acción específica", pa_name);
        // Las teclas PA normalmente no requieren respuesta especial
        Ok(())
    }

    // Procesa AID_NO - confirmación del terminal sin acción específica del usuario
    async fn process_no_aid(&mut self) -> Result<(), Box<dyn Error>> {
        info!("[tn3270][INFO] AID_NO recibido - confirmación del terminal recibida");
        
        // AID_NO indica que el terminal confirmó la recepción de la pantalla
        // pero no hubo interacción específica del usuario
        // Según el protocolo TN3270, esto es normal después de enviar una pantalla
        
        // No enviar respuesta adicional para evitar bucles
        // El teclado ya debería estar desbloqueado desde el WCC enviado anteriormente
        debug!(" AID_NO procesado - esperando interacción del usuario");
        Ok(())
    }

    // Procesa teclas PF (Program Function) - pueden activar funciones especiales
    async fn process_pf_aid(&mut self, pf_num: u8) -> Result<(), Box<dyn Error>> {
        info!("[tn3270][INFO] PF{} recibido", pf_num);
        
        // Mapear teclas PF a funciones específicas
        match pf_num {
            1 => {
                // PF1 = Help
                let help_screen = self.screen_manager.generate_tn3270_screen("help")?;
                self.send_screen_data(help_screen).await?;
            },
            2 => {
                // PF2 = Menu
                let menu_options = [
                    ("1", "Pantalla de bienvenida"),
                    ("2", "Estado del sistema"),
                    ("3", "Demostración de colores"),
                    ("4", "Salir"),
                ];
                let menu_screen = self.screen_manager.generate_menu_screen("MENU PRINCIPAL NEO6", &menu_options)?;
                self.send_screen_data(menu_screen).await?;
            },
            3 => {
                // PF3 = Exit/Return
                let welcome_screen = self.screen_manager.generate_welcome_screen()?;
                self.send_screen_data(welcome_screen).await?;
            },
            12 => {
                // PF12 = Cancel/Exit
                let welcome_screen = self.screen_manager.generate_welcome_screen()?;
                self.send_screen_data(welcome_screen).await?;
            },
            _ => {
                // Tecla PF no definida
                let error_msg = format!("Función PF{} no implementada", pf_num);
                let error_screen = self.screen_manager.generate_error_screen(&error_msg)?;
                self.send_screen_data(error_screen).await?;
            }
        }
        Ok(())
    }

    // Procesa AIDs desconocidos
    async fn process_unknown_aid(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        warn!("AID desconocido, datos: {:02X?}", data);
        let error_msg = format!("Comando no reconocido (AID: 0x{:02X})", data[0]);
        let error_screen = self.screen_manager.generate_error_screen(&error_msg)?;
        self.send_screen_data(error_screen).await?;
        Ok(())
    }

    // Extrae datos de campo del stream TN3270
    async fn extract_field_data(&self, data: &[u8]) -> Result<Vec<String>, Box<dyn Error>> {
        let mut field_data = Vec::new();
        let mut i = 0;

        while i < data.len() {
            // Buscar SBA (Set Buffer Address) orders que indican posiciones de campo
            if i + 2 < data.len() && data[i] == 0x11 {
                // SBA encontrado, saltar dirección (2 bytes)
                i += 3;
                continue;
            }

            // Buscar datos de campo EBCDIC
            let field_start = i;
            while i < data.len() && data[i] != 0x11 && data[i] != 0x00 {
                i += 1;
            }

            if i > field_start {
                // Convertir de EBCDIC a ASCII
                let ebcdic_data = &data[field_start..i];
                let ascii_data = self.screen_manager.codec.from_host(ebcdic_data);
                let field_string = String::from_utf8_lossy(&ascii_data).trim().to_string();
                
                // Filtrar espacios, underscores y otros caracteres de relleno
                let cleaned_field = field_string.chars()
                    .filter(|&c| c != '_' && c != ' ')
                    .collect::<String>();
                
                if !cleaned_field.is_empty() {
                    debug!(" Campo extraído: '{}' (limpiado de: '{}')", cleaned_field, field_string);
                    field_data.push(cleaned_field);
                }
            }

            if i < data.len() {
                i += 1;
            }
        }

        Ok(field_data)
    }

    // Procesa comandos de texto directo (cuando no hay AID válido)
    async fn process_text_command(&mut self, command: &str) -> Result<(), Box<dyn Error>> {
        info!("[tn3270][INFO] Procesando comando de texto: '{}'", command);
        let upper_command = command.to_uppercase();
        match self.screen_manager.generate_screen_by_name(&upper_command) {
            Ok(screen_data) => {
                self.send_screen_data(screen_data).await?;
            },
            Err(_) => {
                let error_msg = format!("Comando '{}' no reconocido", command);
                let error_screen = self.screen_manager.generate_error_screen(&error_msg)?;
                self.send_screen_data(error_screen).await?;
            }
        }
        Ok(())
    }

    // Envía datos de pantalla usando el protocolo apropiado
    async fn send_screen_data(&mut self, screen_data: Vec<u8>) -> Result<(), Box<dyn Error>> {
        if self.tn3270e_bound {
            // Formato TN3270E
            self.send_tn3270e_data(&screen_data).await?;
        } else if self.telnet_state.tn3270e_enabled {
            // TN3270E habilitado pero no bound
            self.send_tn3270e_data(&screen_data).await?;
        } else {
            // Telnet clásico
            self.send_classic_telnet_data(&screen_data).await?;
        }
        
        self.screen_manager.mark_screen_sent();
        Ok(())
    }

    // Envía datos usando protocolo TN3270E
    async fn send_tn3270e_data(&mut self, screen_data: &[u8]) -> Result<(), Box<dyn Error>> {
        let mut tn3270e_data = Vec::new();
        
        // Header TN3270E para datos 3270
        tn3270e_data.push(0x00); // data_type: 3270-DATA
        tn3270e_data.push(0x00); // request_flag
        tn3270e_data.push(0x00); // response_flag  
        tn3270e_data.push(0x00); // seq_number[0]
        tn3270e_data.push(0x00); // seq_number[1]
        
        // Agregar datos de pantalla
        tn3270e_data.extend_from_slice(screen_data);
        
        // Terminar con IAC EOR
        tn3270e_data.push(0xFF); // IAC
        tn3270e_data.push(0xEF); // EOR
        
        self.stream.write_all(&tn3270e_data).await?;
        self.stream.flush().await?;
        
        debug!(" Enviados {} bytes vía TN3270E", tn3270e_data.len());
        Ok(())
    }

    // Envía datos usando Telnet clásico
    async fn send_classic_telnet_data(&mut self, screen_data: &[u8]) -> Result<(), Box<dyn Error>> {
        let mut telnet_data = Vec::new();
        
        // Agregar datos de pantalla
        telnet_data.extend_from_slice(screen_data);
        
        // Terminar con IAC EOR
        telnet_data.push(0xFF); // IAC
        telnet_data.push(0xEF); // EOR
        
        self.stream.write_all(&telnet_data).await?;
        self.stream.flush().await?;
        
        debug!(" Enviados {} bytes vía Telnet clásico", telnet_data.len());
        Ok(())
    }
}

// Move impl Session to module scope and update method stubs
impl Session {
    pub fn contains_tab_or_backtab_input(&self, _actual_data: &[u8]) -> bool {
        // TODO: Implement actual logic
        false
    }

    pub async fn process_keyboard_navigation(&mut self, _actual_data: &[u8]) -> Result<(), Box<dyn Error>> {
        // TODO: Implement actual logic
        Ok(())
    }

}

async fn handle_client(stream: TcpStream) -> Result<(), Box<dyn Error>> {
    debug!(" handle_client: nueva conexión");
    let mut session = Session::new(stream).await;
    let mut buf = BytesMut::with_capacity(4096); // Aumentar capacidad del buffer

    // Iniciar negociación Telnet por parte del servidor:
    // Pedimos al cliente que habilite (DO) ciertas opciones.
    // Ofrecemos habilitar (WILL) ciertas opciones desde nuestro lado.
    // RFC 1576 (TN3270) recomienda:
    // Server: DO TERMINAL-TYPE, DO BINARY, DO EOR
    // Server: WILL BINARY, WILL EOR
    // Para TN3270E (RFC 2355), también negociamos TN3270E:
    // Server: DO TN3270E
    // Server: WILL TN3270E (opcional si el cliente lo inicia, pero bueno ser explícito)

    debug!("[tn3270][SEND] Iniciando handshake Telnet:");
    session.stream.write_all(&[
        IAC, DO, OPT_TERMINAL_TYPE, // Queremos que el cliente nos diga su tipo
        IAC, DO, OPT_BINARY,       // Queremos que el cliente opere en binario
        IAC, DO, OPT_EOR,          // Queremos que el cliente reconozca EOR
        IAC, DO, OPT_TN3270E,      // Queremos que el cliente use TN3270E
        
        IAC, WILL, OPT_BINARY,     // Nosotros operaremos en binario
        IAC, WILL, OPT_EOR,        // Nosotros enviaremos EOR
        IAC, WILL, OPT_TN3270E,    // Nosotros soportamos TN3270E
        // No enviamos WILL SUPPRESS-GO-AHEAD ni WILL ECHO, ya que no los implementamos activamente.
        // El cliente puede ofrecerlos (WILL SGA, WILL ECHO), y responderemos DONT.
    ]).await?;
    session.stream.flush().await?; // Asegurar que el handshake se envíe inmediatamente.
    debug!(" Handshake Telnet inicial enviado.");

    // No es necesario enviar IAC NOP aquí generalmente.
    // session.stream.write_all(&[IAC, NOP]).await?;
    // debug!(" Enviado IAC NOP tras handshake");

    // El timeout para el REQUEST DEVICE-TYPE proactivo se elimina.
    // La negociación se conducirá por el estado y las respuestas del cliente.

    loop {
        // tokio::select! permite esperar en múltiples futuros.
        // Aquí solo esperamos datos del cliente.
        // Podríamos añadir un timeout general de inactividad si fuera necesario.
        match session.stream.read_buf(&mut buf).await {
            Ok(0) => {
                info!("[tn3270][INFO] Cliente desconectado (stream cerrado).");
                break Ok(());
            }
            Ok(n) => {
                debug!("[tn3270][RECV] Raw data ({} bytes): {:02X?}", n, &buf[..n]);
                // Clonar los datos recibidos para procesarlos.
                // handle_negotiation puede necesitar ver todo el fragmento.
                let received_data = buf[..n].to_vec();
                buf.clear(); // Limpiar el buffer para la próxima lectura.
                
                if let Err(e) = session.handle_negotiation(&received_data).await {
                    error!("[tn3270][ERROR] Error en handle_negotiation: {}", e);
                    // Decidir si cerrar la conexión o intentar continuar.
                    // Por ahora, cerramos en caso de error grave de negociación.
                    break Err(e);
                }
                
                // Después de procesar los datos, si la sesión está bound pero no se ha enviado la pantalla,
                // asegurarse de que se envíe
                if !session.screen_manager.is_screen_sent() && (session.tn3270e_bound || 
                    (!session.telnet_state.tn3270e_enabled && session.telnet_state.base_telnet_ready)) {
                    if let Err(e) = session.maybe_send_screen().await {
                        error!("[tn3270][ERROR] Error enviando pantalla: {}", e);
                        break Err(e);
                    }
                }
            }
            Err(e) => {
                error!("[tn3270][ERROR] Error de lectura del stream: {}", e);
                break Err(Box::new(e));
            }
        }
    }
}

pub struct Tn3270Handler;

#[async_trait]
impl ProtocolHandler for Tn3270Handler {
    async fn invoke_transaction(&self, transaction_id: &str, parameters: Value) -> Result<Value, String> {
        debug!("Tn3270Handler::invoke_transaction() - Invocando transacción: {} con parámetros: {:?}", transaction_id, parameters);
        Ok(json!({
            "protocol": "tn3270",
            "transaction_id": transaction_id,
            "parameters": parameters,
            "result": "mocked-tn3270-response"
        }))
    }
}

/// Lanza el listener TN3270 de forma asíncrona (para usar dentro de Tokio)
pub async fn start_tn3270_listener<ExecFn, Fut>(
    port: u16,
    _tx_map: std::sync::Arc<std::collections::HashMap<String, TransactionConfig>>,
    _exec_fn: ExecFn,
) -> std::io::Result<()> 
where
    ExecFn: Fn(String, serde_json::Value) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'static,
{
    debug!("start_tn3270_listener() - Iniciando listener TN3270 en puerto {}", port);
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    info!("Listening on {} (async)", addr);
    
    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream).await {
                error!("Client error: {}", e);
            }
        });
    }
    #[allow(unreachable_code)]
    Ok(())
}

// -- Re-export commonly used structs for external tools --
// These exports allow binary utilities within this package to reuse the
// screen generation capabilities without exposing the entire module
// structure.  They are intentionally narrow to keep the public API small.


