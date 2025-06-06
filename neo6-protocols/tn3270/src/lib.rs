use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use bytes::BytesMut;
use ebcdic::ebcdic::Ebcdic;
use std::error::Error;
use async_trait::async_trait;
use serde_json::Value;
use serde_json::json;
use neo6_protocols_lib::protocol::{ProtocolHandler, TransactionConfig};

// Módulo de pantallas 3270
mod tn3270_screens;
use tn3270_screens::ScreenManager;

// --- Constantes Telnet y TN3270E ---
// Comandos Telnet
const IAC: u8 = 255; // Interpret As Command
const DONT: u8 = 254;
const DO: u8 = 253;
const WONT: u8 = 252;
const WILL: u8 = 251;
const SB: u8 = 250;   // Subnegotiation Begin
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
const TN3270E_OP_ASSOCIATE: u8 = 0;
const TN3270E_OP_CONNECT: u8 = 1;
const TN3270E_OP_DEVICE_TYPE: u8 = 2;
const TN3270E_OP_FUNCTIONS: u8 = 3;
const TN3270E_OP_IS: u8 = 4;
const TN3270E_OP_REASON: u8 = 5;
const TN3270E_OP_REJECT: u8 = 6;
const TN3270E_OP_REQUEST: u8 = 7;
const TN3270E_OP_SEND: u8 = 8;

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
    will_echo: bool,
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

// Conversión de caracteres
#[derive(Debug)]
pub struct Codec {
    use_ebcdic: bool,
    code_page: String,
}

impl Codec {
    pub fn new() -> Self {
        Codec {
            use_ebcdic: true,
            code_page: "CP037".to_string(),
        }
    }

    pub fn to_host(&self, data: &[u8]) -> Vec<u8> {
        if self.use_ebcdic {
            let mut output = vec![0u8; data.len()];
            Ebcdic::ebcdic_to_ascii(data, &mut output, data.len(), false, true);
            output
        } else {
            data.to_vec()
        }
    }

    pub fn from_host(&self, data: &[u8]) -> Vec<u8> {
        if self.use_ebcdic {
            // Asegurar que el buffer de salida tenga suficiente capacidad.
            // La conversión EBCDIC puede necesitar más espacio si hay caracteres multibyte,
            // pero para nombres de dispositivo y LU típicos, la misma longitud suele ser suficiente.
            // Para estar seguros, podríamos usar data.len() * 2, pero es improbable para estos strings.
            let mut output = vec![0u8; data.len()];
            Ebcdic::ascii_to_ebcdic(data, &mut output, data.len(), true);
            // La función ascii_to_ebcdic modifica directamente el vector output, 
            // y no necesitamos truncarlo ya que se ajusta automáticamente
            output
        } else {
            data.to_vec()
        }
    }
}

// Sesión de cliente
#[derive(Debug)]
struct Session {
    stream: TcpStream,
    telnet_state: TelnetState,
    tn3270e: TN3270EState,
    codec: Codec,
    screen_manager: ScreenManager,
    terminal_type: String, // Se llenará durante la negociación
    logical_unit: String, // Nombre de la LU, puede ser asignado por el servidor o negociado
    tn3270e_bound: bool, // True cuando la negociación TN3270E está completa y se puede enviar datos 3270
}

impl Session {
    async fn new(stream: TcpStream) -> Self {
        println!("[tn3270][DEBUG] Nueva sesión creada");
        Session {
            stream,
            telnet_state: TelnetState::default(),
            tn3270e: TN3270EState::default(),
            codec: Codec::new(),
            screen_manager: ScreenManager::new(),
            terminal_type: String::new(), 
            logical_unit: "NEO6LU".to_string(), // LU por defecto, puede ser sobrescrita por el cliente
            tn3270e_bound: false, // Se pone a true cuando la negociación TN3270E está completa
        }
    }

    // Comprueba si todas las negociaciones Telnet base están listas
    // Esto es un prerrequisito para iniciar la negociación TN3270E.
    fn check_base_telnet_ready(&mut self) {
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
                println!("[tn3270][DEBUG] Negociaciones Telnet básicas para TN3270E completadas.");
                println!("[tn3270][DEBUG] Estado actual: BINARY={}, EOR={}, TN3270E={}, TERMINAL-TYPE={}",
                         self.telnet_state.binary_negotiated, self.telnet_state.eor_negotiated,
                         self.telnet_state.tn3270e_negotiated, self.telnet_state.termtype_negotiated);
            } else {
                println!("[tn3270][DEBUG] Negociaciones Telnet base completadas. Usando modo Telnet clásico 3270.");
            }
        }
    }

    // Inicia la secuencia de negociación TN3270E por parte del servidor.
    // Se llama cuando base_telnet_ready es true y TN3270E está habilitado.
    async fn initiate_tn3270e_server_negotiation(&mut self) -> Result<(), Box<dyn Error>> {
        // Verificar que se cumplan las condiciones para la negociación TN3270E
        if !self.telnet_state.base_telnet_ready || !self.telnet_state.tn3270e_enabled {
            println!("[tn3270][DEBUG] No se cumplen las condiciones para negociación TN3270E. base_ready={}, tn3270e_enabled={}",
                     self.telnet_state.base_telnet_ready, self.telnet_state.tn3270e_enabled);
            return Ok(());
        }

        // Secuencia de negociación TN3270E iniciada por el servidor:
        // 1. Enviar S2C_CONNECT (si no se ha enviado y no se ha recibido respuesta de DEVICE-TYPE)
        if !self.tn3270e.connect_sent && !self.tn3270e.client_device_type_is_received {
            println!("[tn3270][INFO] Iniciando negociación TN3270E: Paso 1 - Enviando DEVICE-TYPE IS.");
            
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
            println!("[tn3270][SEND] TN3270E DEVICE-TYPE IS IBM-3278-2-E CONNECT {} Raw: {:02X?}", self.logical_unit, connect_msg);
            self.stream.write_all(&connect_msg).await?;
            self.stream.flush().await?;
            self.tn3270e.connect_sent = true;
            
            // También registramos este evento para depuración
            println!("[tn3270][DEBUG] DEVICE-TYPE IS with CONNECT enviado correctamente, esperando respuesta del cliente");
        }

        // 2. Enviar S2C_DEVICE_TYPE_REQUEST (si CONNECT se envió y DEVICE_TYPE_REQUEST no, y no se ha recibido respuesta de DEVICE-TYPE)
        if self.tn3270e.connect_sent && !self.tn3270e.device_type_request_sent && !self.tn3270e.client_device_type_is_received {
            println!("[tn3270][INFO] Negociación TN3270E: Paso 2 - Enviando S2C_DEVICE_TYPE_REQUEST.");
            
            // Formato según RFC 2355 para DEVICE-TYPE-REQUEST:
            // IAC SB TN3270E DEVICE-TYPE REQUEST <device-type list> IAC SE
            // REQUEST es un sub-comando, no parte del data-type
            
            let mut req_dev_type_msg = vec![IAC, SB, OPT_TN3270E, TN3270E_OP_DEVICE_TYPE, TN3270E_OP_REQUEST];
            
            // Agregar "IBM-3278-2-E" - un tipo compatible con c3270
            req_dev_type_msg.extend_from_slice("IBM-3278-2-E".as_bytes());
                        
            req_dev_type_msg.extend_from_slice(&[IAC, SE]);
            println!("[tn3270][SEND] TN3270E S2C_DEVICE_TYPE_REQUEST ({:02X?})", req_dev_type_msg);
            self.stream.write_all(&req_dev_type_msg).await?;
            self.stream.flush().await?;
            self.tn3270e.device_type_request_sent = true;
            
            println!("[tn3270][DEBUG] DEVICE-TYPE-REQUEST enviado correctamente");
        }
        
        // Los siguientes pasos (S2C_FUNCTIONS_REQUEST, S2C_BIND_IMAGE) se envían en respuesta
        // a los mensajes del cliente (C2S_MESSAGE_DEVICE_TYPE, C2S_MESSAGE_FUNCTIONS).
        Ok(())
    }

    // Procesa un mensaje TN3270E entrante del cliente.
    // El payload es el contenido de la subnegociación TN3270E, sin IAC SB OPT_TN3270E ... IAC SE.
    async fn process_incoming_tn3270e_message(&mut self, payload: &[u8]) -> Result<(), Box<dyn Error>> {
        if payload.is_empty() {
            println!("[tn3270][WARN] Payload TN3270E vacío.");
            return Ok(());
        }

        let tn3270e_cmd = payload[0]; // Este es el data-type del cliente.
        let data = &payload[1..];     // Resto del payload.

        println!("[tn3270][RECV] TN3270E Mensaje del Cliente: cmd=0x{:02X}, data={:02X?}", tn3270e_cmd, data);
        
        // Mostrar la interpretación del comando para mejor diagnóstico
        let cmd_name = match tn3270e_cmd {
            TN3270E_OP_DEVICE_TYPE => "DEVICE-TYPE",
            TN3270E_OP_FUNCTIONS => "FUNCTIONS",
            _ => "UNKNOWN",
        };
        println!("[tn3270][DEBUG] TN3270E cmd interpretado: {}", cmd_name);

        match tn3270e_cmd {
            TN3270E_OP_DEVICE_TYPE => { // Cliente envía su tipo de dispositivo (0x02)
                if data.is_empty() {
                    println!("[tn3270][WARN] TN3270E_OP_DEVICE_TYPE sin datos de sub-operación.");
                    return Ok(());
                }
                let sub_operation = data[0];
                let payload = &data[1..];
                
                // Log más detallado para diagnóstico del payload
                println!("[tn3270][DEBUG] DEVICE-TYPE sub-operation: 0x{:02X} ({}) - Payload: {:02X?}", 
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
                        println!("[tn3270][INFO] Negociación TN3270E: Recibido DEVICE-TYPE IS del cliente.");
                        
                        // Imprimir los datos raw y en ASCII para diagnóstico
                        let device_info_ascii = String::from_utf8_lossy(payload);
                        println!("[tn3270][DEBUG] DEVICE-TYPE raw data: {:02X?}", payload);
                        println!("[tn3270][DEBUG] DEVICE-TYPE ASCII data: '{}'", device_info_ascii);
                        
                        // Usar el TN3270E_OP_CONNECT (0x01) como separador si está presente
                        let mut parts_iter = payload.split(|&byte| byte == TN3270E_OP_CONNECT);
                        let device_type_bytes = parts_iter.next().unwrap_or_default();
                        let resource_name_bytes = parts_iter.next();

                        if !device_type_bytes.is_empty() {
                            self.terminal_type = String::from_utf8_lossy(device_type_bytes).to_string();
                            println!("[tn3270][INFO] Cliente seleccionó DEVICE-TYPE: {}", self.terminal_type);
                        }
                        
                        if let Some(res_name_bytes) = resource_name_bytes {
                            if !res_name_bytes.is_empty() {
                                self.logical_unit = String::from_utf8_lossy(res_name_bytes).to_string();
                                println!("[tn3270][INFO] Cliente especificó RESOURCE-NAME (LU): {}", self.logical_unit);
                            }
                        }
                        
                        self.tn3270e.client_device_type_is_received = true;

                        // Paso 3: Enviar S2C_FUNCTIONS_REQUEST si no se ha enviado
                        if !self.tn3270e.functions_request_sent {
                            println!("[tn3270][INFO] Negociación TN3270E: Paso 3 - Enviando S2C_FUNCTIONS_REQUEST.");
                            
                            // Para c3270, empecemos con un conjunto mínimo de funciones
                            self.tn3270e.requested_functions_by_server = vec![
                                FUNC_DATA_STREAM_CTL, // Requerido para mensajes DATA TN3270E
                            ];
                            
                            let mut func_req_msg = vec![IAC, SB, OPT_TN3270E, TN3270E_OP_FUNCTIONS, TN3270E_OP_REQUEST];
                            func_req_msg.extend_from_slice(&self.tn3270e.requested_functions_by_server);
                            func_req_msg.extend_from_slice(&[IAC, SE]);
                            
                            println!("[tn3270][SEND] TN3270E S2C_FUNCTIONS_REQUEST ({:02X?})", func_req_msg);
                            self.stream.write_all(&func_req_msg).await?;
                            self.stream.flush().await?;
                            self.tn3270e.functions_request_sent = true;
                            
                            println!("[tn3270][DEBUG] FUNCTIONS-REQUEST enviado correctamente");
                        }
                    }
                    TN3270E_OP_REQUEST => {
                        // Cliente envía DEVICE-TYPE REQUEST (poco común, normalmente el servidor envía esto)
                        println!("[tn3270][INFO] Recibido DEVICE-TYPE REQUEST del cliente (poco común)");
                        // Responder con IS si es necesario
                    }
                    TN3270E_OP_REJECT => {
                        println!("[tn3270][WARN] Cliente RECHAZÓ DEVICE-TYPE. Payload: {:02X?}", payload);
                        // Manejar rechazo
                    }
                    _ => {
                        println!("[tn3270][WARN] Sub-operación DEVICE-TYPE desconocida: 0x{:02X}", sub_operation);
                    }
                }
            }
            TN3270E_OP_FUNCTIONS => { // Cliente envía sus funciones aceptadas (0x03)
                if data.is_empty() {
                    println!("[tn3270][WARN] TN3270E_OP_FUNCTIONS sin datos de sub-operación.");
                    return Ok(());
                }
                let sub_operation = data[0];
                let payload = &data[1..];
                
                println!("[tn3270][DEBUG] FUNCTIONS sub-operation: 0x{:02X} ({}) - Payload: {:02X?}", 
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
                        println!("[tn3270][INFO] Negociación TN3270E: Recibido FUNCTIONS REQUEST del cliente.");
                        println!("[tn3270][INFO] Cliente solicita funciones: {:02X?}", payload);
                        
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
                        println!("[tn3270][INFO] Cliente solicita funciones: {:?} (códigos: {:02X?})", 
                                 functions_names, payload);
                        
                        // Responder con FUNCTIONS IS indicando qué funciones aceptamos
                        // Por ahora, aceptamos todas las funciones solicitadas por el cliente
                        let accepted_functions = payload.to_vec();
                        
                        let mut func_is_msg = vec![IAC, SB, OPT_TN3270E, TN3270E_OP_FUNCTIONS, TN3270E_OP_IS];
                        func_is_msg.extend_from_slice(&accepted_functions);
                        func_is_msg.extend_from_slice(&[IAC, SE]);
                        
                        println!("[tn3270][SEND] TN3270E FUNCTIONS IS ({:02X?})", func_is_msg);
                        self.stream.write_all(&func_is_msg).await?;
                        self.stream.flush().await?;
                        
                        // Verificar si el cliente negoció BIND-IMAGE antes de mover accepted_functions
                        let client_wants_bind_image = accepted_functions.contains(&TN3270E_FUNC_BIND_IMAGE);
                        
                        // Guardar las funciones aceptadas
                        self.tn3270e.accepted_functions_by_client = accepted_functions;
                        self.tn3270e.client_functions_is_received = true;

                        // Enviar BIND-IMAGE si el cliente negoció esta función
                        if client_wants_bind_image {
                            println!("[tn3270][INFO] Cliente negoció BIND-IMAGE, enviando comando BIND.");
                            if let Err(e) = self.send_bind_image().await {
                                println!("[tn3270][ERROR] Error enviando BIND-IMAGE: {}", e);
                                return Err(e);
                            }
                        }
                        
                        // Marcar la sesión como BOUND después de intercambiar funciones
                        println!("[tn3270][INFO] Negociación TN3270E completada. Sesión BOUND.");
                        self.tn3270e_bound = true;
                        self.tn3270e.bind_image_sent = true;
                        
                        // Enviar pantalla inicial inmediatamente después de que la sesión esté bound
                        if let Err(e) = self.maybe_send_screen().await {
                            println!("[tn3270][ERROR] Error enviando pantalla inicial: {}", e);
                            return Err(e);
                        }
                        
                        println!("[tn3270][DEBUG] Sesión TN3270E bound completada, esperando entrada del usuario.");
                    }
                    TN3270E_OP_IS => {
                        // Cliente envía FUNCTIONS IS (respuesta a nuestro FUNCTIONS REQUEST)
                        println!("[tn3270][INFO] Negociación TN3270E: Recibido FUNCTIONS IS del cliente.");
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
                        println!("[tn3270][INFO] Cliente aceptó funciones: {:?} (códigos: {:02X?})", 
                                 functions_names, self.tn3270e.accepted_functions_by_client);
                        self.tn3270e.client_functions_is_received = true;

                        // Marcar la sesión como BOUND
                        println!("[tn3270][INFO] Negociación TN3270E completada. Sesión BOUND.");
                        self.tn3270e_bound = true;
                        self.tn3270e.bind_image_sent = true;
                        
                        // Enviar pantalla inicial
                        if let Err(e) = self.maybe_send_screen().await {
                            println!("[tn3270][ERROR] Error enviando pantalla inicial: {}", e);
                            return Err(e);
                        }
                        
                        println!("[tn3270][DEBUG] Sesión TN3270E bound completada, esperando entrada del usuario.");
                    }
                    TN3270E_OP_REJECT => {
                        println!("[tn3270][WARN] Cliente RECHAZÓ FUNCTIONS. Funciones rechazadas: {:02X?}", payload);
                        // Manejar el rechazo según sea necesario
                    }
                    _ => {
                        println!("[tn3270][WARN] Sub-operación FUNCTIONS desconocida: 0x{:02X}", sub_operation);
                    }
                }
            }
            // TODO: Manejar C2S_MESSAGE_DATA, C2S_RESPONSES_REQUEST, C2S_SCS, C2S_SYSREQ si es necesario.
            _ => {
                println!("[tn3270][WARN] Comando TN3270E desconocido del cliente: 0x{:02X}", tn3270e_cmd);
            }
        }
        Ok(())
    }


    async fn maybe_send_screen(&mut self) -> Result<(), Box<dyn Error>> {
        // Solo enviar pantalla si:
        // 1. TN3270E está habilitado Y la negociación TN3270E está completa (bound)
        // O
        // 2. El modo Telnet clásico (no TN3270E) está completamente negociado Y el tipo de terminal es 3270
        // Y
        // 3. La pantalla no se ha enviado aún.

        let classic_telnet_ready_for_screen = 
            !self.telnet_state.tn3270e_enabled && // TN3270E no está en uso
            self.telnet_state.binary_negotiated &&
            self.telnet_state.eor_negotiated &&
            self.telnet_state.termtype_negotiated && self.terminal_type.contains("327");

        let tn3270e_ready_for_screen = 
            self.telnet_state.tn3270e_enabled && self.tn3270e_bound;

        if !self.screen_manager.is_screen_sent() && (tn3270e_ready_for_screen || classic_telnet_ready_for_screen) {
            println!("[tn3270][DEBUG] Condiciones cumplidas para enviar pantalla inicial. TN3270E: {}, Bound: {}, Classic: {}",
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

                println!("[tn3270][SEND] TN3270E DATA (seq: {}, total {} bytes)", seq_num, tn3270e_data_msg.len());
                println!("[tn3270][SEND] TN3270E Header: {:02X?}", &tn3270e_data_msg[0..5]);
                println!("[tn3270][SEND] 3270 Data (full): {:02X?}", &tn3270e_data_msg[5..tn3270e_data_msg.len()-2]);
                println!("[tn3270][SEND] 3270 Command: {:02X} (Erase/Write), WCC: {:02X}", 
                         tn3270e_data_msg[5], tn3270e_data_msg[6]);
                if tn3270e_data_msg.len() > 20 {
                    println!("[tn3270][SEND] 3270 Data structure: {:02X?}...", &tn3270e_data_msg[7..std::cmp::min(25, tn3270e_data_msg.len())]);
                }
                self.stream.write_all(&tn3270e_data_msg).await?;
            } else {
                // Enviar como flujo Telnet clásico con IAC EOR
                let mut classic_data_msg = screen_data;
                classic_data_msg.extend_from_slice(&[IAC, EOR_TELNET_CMD]); // EOR Telnet command
                println!("[tn3270][SEND] Pantalla inicial como Telnet clásico con EOR ({:02X?})", classic_data_msg);
                self.stream.write_all(&classic_data_msg).await?;
            }
            
            self.stream.flush().await?;
            self.screen_manager.mark_screen_sent();
            println!("[tn3270][DEBUG] Pantalla inicial enviada. Manteniendo conexión abierta para entrada del usuario.");
        }
        Ok(())
    }


    async fn handle_negotiation(&mut self, buf: &[u8]) -> Result<(), Box<dyn Error>> {
        println!("[tn3270][RECV] handle_negotiation: {:02X?}", buf);
        let mut i = 0;
        while i < buf.len() {
            if buf[i] == IAC { // IAC
                if i + 1 >= buf.len() {
                    println!("[tn3270][WARN] IAC al final del buffer, esperando más datos.");
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
                        println!("[tn3270][DEBUG] Subnegociación detectada: Opt=0x{:02X}, Payload: {:02X?}", sub_data_payload[0], &sub_data_payload[1..]);
                        self.handle_subnegotiation(sub_data_payload).await?;
                        i = sub_end_idx + 2; // Avanzar el índice más allá de IAC SE
                        continue;
                    } else {
                        println!("[tn3270][DEBUG] Subnegociación incompleta (sin IAC SE), esperando más datos.");
                        // Almacenar datos parciales o simplemente esperar más.
                        break;
                    }
                }
                // Comandos Telnet de 3 bytes (IAC CMD OPT)
                if i + 2 >= buf.len() {
                    println!("[tn3270][WARN] Comando Telnet incompleto (IAC CMD sin OPT), esperando más datos.");
                    break;
                }
                let opt = buf[i+2];
                println!("[tn3270][DEBUG] Telnet cmd: 0x{:02X} opt: 0x{:02X}", cmd, opt);
                match cmd {
                    WILL => self.handle_will(opt).await?,
                    WONT => self.handle_wont(opt).await?,
                    DO => self.handle_do(opt).await?,
                    DONT => self.handle_dont(opt).await?,
                    _ => {
                        println!("[tn3270][WARN] Comando Telnet desconocido o no manejado: 0x{:02X}", cmd);
                        // Podría ser un comando de 2 bytes como IAC EOR_TELNET_CMD (255 239)
                        if cmd == EOR_TELNET_CMD && opt == IAC { // Esto es un error de parseo, EOR es IAC EOR
                             println!("[tn3270][DEBUG] Recibido IAC EOR (comando Telnet, no opción)");
                             // Esto es una marca de fin de registro en modo Telnet clásico.
                             // El cliente envía esto después de sus datos de entrada.
                             // Aquí deberíamos procesar los datos 3270 recibidos antes de este EOR.
                             // Por ahora, solo lo registramos.
                             i += 2; // Consumir IAC EOR
                             continue;
                        }
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
                    println!("[tn3270][DEBUG] Datos inesperados ({:02X?}) fuera de subnegociación en modo TN3270E bound. Podrían ser datos de usuario.", data_chunk);
                    // En lugar de tratarlos como error, procesarlos como posible entrada del usuario
                    if !data_chunk.is_empty() {
                        self.send_3270_data(data_chunk).await?;
                    }
                } else if !self.telnet_state.tn3270e_enabled && self.telnet_state.binary_negotiated && self.telnet_state.eor_negotiated {
                     println!("[tn3270][RECV] Datos 3270 (modo clásico Telnet) ({:02X?}). Esperando IAC EOR.", data_chunk);
                     // Aquí se acumularían estos datos hasta ver IAC EOR.
                     if !data_chunk.is_empty() {
                         self.send_3270_data(data_chunk).await?;
                     }
                } else {
                    println!("[tn3270][RECV] Datos NVT/Raw ({:02X?})", data_chunk);
                    // Para datos NVT durante la negociación, simplemente los ignoramos o procesamos como comandos básicos
                }
                i = buf.len(); // Consumir el resto del buffer como datos (simplificación por ahora)
            }
        }

        // Comprobar si las negociaciones base Telnet están listas después de procesar el buffer.
        self.check_base_telnet_ready();
        
        // Mostrar el estado actual para diagnóstico
        println!("[tn3270][DEBUG] Estado tras procesar buffer: base_ready={}, tn3270e_enabled={}, binary_negotiated={}, eor_negotiated={}, tn3270e_negotiated={}, connect_sent={}",
                 self.telnet_state.base_telnet_ready, self.telnet_state.tn3270e_enabled,
                 self.telnet_state.binary_negotiated, self.telnet_state.eor_negotiated, 
                 self.telnet_state.tn3270e_negotiated, self.tn3270e.connect_sent);

        // Si las negociaciones Telnet base están listas y TN3270E está habilitado,
        // y aún no hemos iniciado la negociación TN3270E por parte del servidor, hacerlo.
        if self.telnet_state.tn3270e_enabled && 
           self.telnet_state.binary_negotiated && 
           self.telnet_state.eor_negotiated && 
           !self.tn3270e.connect_sent {
            println!("[tn3270][INFO] Condiciones cumplidas para iniciar negociación TN3270E");
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
            println!("[tn3270][WARN] Subnegociación con payload vacío.");
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
                    println!("[tn3270][INFO] Tipo de terminal recibido: {}", self.terminal_type);
                    self.telnet_state.termtype_negotiated = true; // Marcar como negociado
                    // No necesitamos responder a TERMINAL-TYPE IS.
                } else {
                    println!("[tn3270][WARN] Subnegociación TERMINAL-TYPE mal formada o no es 'IS': {:02X?}", params);
                }
            }
            OPT_TN3270E => {
                // Cliente envía un mensaje TN3270E.
                // params = <tn3270e_cmd_or_response> <data...>
                println!("[tn3270][DEBUG] Recibida subnegociación TN3270E. Payload: {:02X?}", params);
                self.process_incoming_tn3270e_message(params).await?;
            }
            _ => {
                println!("[tn3270][WARN] Subnegociación para opción desconocida o no manejada: 0x{:02X}", option);
            }
        }
        Ok(())
    }


    async fn handle_will(&mut self, opt: u8) -> Result<(), Box<dyn Error>> {
        println!("[tn3270][RECV] WILL 0x{:02X}", opt);
        match opt {
            OPT_BINARY => {
                if !self.telnet_state.binary_negotiated { // Solo negociar una vez
                    println!("[tn3270][SEND] DO OPT_BINARY");
                    self.stream.write_all(&[IAC, DO, OPT_BINARY]).await?;
                    self.telnet_state.binary_mode = true; // El cliente quiere, nosotros aceptamos
                    self.telnet_state.binary_negotiated = true;
                }
            }
            OPT_EOR => {
                if !self.telnet_state.eor_negotiated {
                    println!("[tn3270][SEND] DO OPT_EOR");
                    self.stream.write_all(&[IAC, DO, OPT_EOR]).await?;
                    self.telnet_state.eor_negotiated = true;
                }
            }
            OPT_TN3270E => {
                if !self.telnet_state.tn3270e_negotiated {
                    println!("[tn3270][SEND] DO OPT_TN3270E");
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
                 println!("[tn3270][INFO] Cliente WILL TERMINAL-TYPE. Esperando subnegociación con el tipo.");
                 // No marcamos termtype_negotiated aquí, sino cuando recibimos el tipo en subnegociación.
            }
            OPT_ECHO | OPT_SUPPRESS_GO_AHEAD => {
                // No soportamos ECHO ni SGA activamente, así que respondemos DONT.
                println!("[tn3270][SEND] DONT 0x{:02X}", opt);
                self.stream.write_all(&[IAC, DONT, opt]).await?;
            }
            _ => {
                println!("[tn3270][SEND] DONT 0x{:02X} (opción desconocida que el cliente ofrece)", opt);
                self.stream.write_all(&[IAC, DONT, opt]).await?;
            }
        }
        self.check_base_telnet_ready();
        
        // Debug para entender el estado actual de la negociación
        println!("[tn3270][DEBUG] Estado tras DO 0x{:02X}: base_ready={}, tn3270e_enabled={}, connect_sent={}, device_type_received={}", 
                 opt, self.telnet_state.base_telnet_ready, self.telnet_state.tn3270e_enabled, 
                 self.tn3270e.connect_sent, self.tn3270e.client_device_type_is_received);
        
        // Forzar la revisión de la negociación TN3270E después de recibir cada comando DO
        if self.telnet_state.tn3270e_enabled && self.telnet_state.binary_negotiated &&            self.telnet_state.eor_negotiated && !self.tn3270e.connect_sent {
            println!("[tn3270][INFO] Iniciando negociación TN3270E después de recibir DO 0x{:02X}", opt);
            self.initiate_tn3270e_server_negotiation().await?;
        }
        
        Ok(())
    }

    async fn handle_wont(&mut self, opt: u8) -> Result<(), Box<dyn Error>> {
        println!("[tn3270][RECV] WONT 0x{:02X}", opt);
        match opt {
            OPT_BINARY => {
                println!("[tn3270][WARN] Cliente WONT BINARY. Esto puede causar problemas.");
                self.telnet_state.binary_mode = false;
                self.telnet_state.binary_negotiated = true; // La negociación (fallida) ha terminado.
                // Respondemos DONT para confirmar que no lo usaremos.
                self.stream.write_all(&[IAC, DONT, OPT_BINARY]).await?;
            }
            OPT_EOR => {
                println!("[tn3270][WARN] Cliente WONT EOR. Necesario para modo Telnet clásico.");
                self.telnet_state.eor_negotiated = true; // Negociación terminada.
                self.stream.write_all(&[IAC, DONT, OPT_EOR]).await?;
            }
            OPT_TN3270E => {
                println!("[tn3270][INFO] Cliente WONT TN3270E. Se usará Telnet clásico si es posible.");
                self.telnet_state.tn3270e_enabled = false;
                self.telnet_state.tn3270e_negotiated = true; // Negociación terminada.
                // Respondemos DONT para confirmar.
                self.stream.write_all(&[IAC, DONT, OPT_TN3270E]).await?;
            }
            OPT_TERMINAL_TYPE => {
                println!("[tn3270][INFO] Cliente WONT TERMINAL-TYPE.");
                // No podemos obtener el tipo de terminal.
                self.telnet_state.termtype_negotiated = true; // Negociación terminada.
                self.stream.write_all(&[IAC, DONT, OPT_TERMINAL_TYPE]).await?;
            }
            // Otras opciones que podríamos haber solicitado con WILL y el cliente rechaza con WONT.
            _ => {
                println!("[tn3270][INFO] Cliente WONT opción 0x{:02X}. Confirmando con DONT.", opt);
                self.stream.write_all(&[IAC, DONT, opt]).await?;
            }
        }
        self.check_base_telnet_ready();
        // Si TN3270E fue rechazado, no intentar iniciar su negociación.
        // La lógica en check_base_telnet_ready y initiate_tn3270e_server_negotiation se encarga de esto.
        Ok(())
    }

    async fn handle_do(&mut self, opt: u8) -> Result<(), Box<dyn Error>> {
        println!("[tn3270][RECV] DO 0x{:02X}", opt);
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
                    println!("[tn3270][SEND] WILL OPT_TERMINAL_TYPE");
                    self.stream.write_all(&[IAC, WILL, OPT_TERMINAL_TYPE]).await?;
                    // Ahora solicitamos el tipo de terminal
                    println!("[tn3270][SEND] SB TERMINAL-TYPE SEND");
                    self.stream.write_all(&[IAC, SB, OPT_TERMINAL_TYPE, 1, IAC, SE]).await?; // 1 = SEND
                }
            }
            OPT_BINARY => {
                // Cliente solicita que operemos en modo binario (DO BINARY)
                if !self.telnet_state.binary_negotiated {
                    println!("[tn3270][SEND] WILL OPT_BINARY");
                    self.stream.write_all(&[IAC, WILL, OPT_BINARY]).await?;
                    self.telnet_state.binary_mode = true;
                    self.telnet_state.binary_negotiated = true;
                } else if !self.telnet_state.binary_mode {
                    // Ya negociado, pero estaba apagado, y cliente pide DO
                    println!("[tn3270][SEND] WILL OPT_BINARY (reactivando)");
                    self.stream.write_all(&[IAC, WILL, OPT_BINARY]).await?;
                    self.telnet_state.binary_mode = true;
                }
            }
            OPT_EOR => {
                // Cliente solicita que soportemos EOR (DO EOR)
                if !self.telnet_state.eor_negotiated {
                    println!("[tn3270][SEND] WILL OPT_EOR");
                    self.stream.write_all(&[IAC, WILL, OPT_EOR]).await?;
                    self.telnet_state.eor_negotiated = true;
                }
            }
            OPT_TN3270E => {
                // Cliente solicita que habilitemos TN3270E (DO TN3270E)
                if !self.telnet_state.tn3270e_negotiated {
                    println!("[tn3270][SEND] WILL OPT_TN3270E");
                    self.stream.write_all(&[IAC, WILL, OPT_TN3270E]).await?;
                    self.telnet_state.tn3270e_enabled = true;
                    self.telnet_state.tn3270e_negotiated = true;
                    
                    // Intentar iniciar la negociación TN3270E si también tenemos BINARY y EOR
                    if self.telnet_state.binary_negotiated && self.telnet_state.eor_negotiated {
                        println!("[tn3270][INFO] Requisitos básicos para TN3270E cumplidos, iniciando negociación");
                        // Forzar base_telnet_ready a true si se cumplen los requisitos mínimos
                        self.telnet_state.base_telnet_ready = true;
                        self.initiate_tn3270e_server_negotiation().await?;
                    }
                } else if !self.telnet_state.tn3270e_enabled {
                    // Ya negociado, pero estaba apagado, y cliente pide DO
                    println!("[tn3270][SEND] WILL OPT_TN3270E (reactivando)");
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
                println!("[tn3270][SEND] WONT 0x{:02X}", opt);
                self.stream.write_all(&[IAC, WONT, opt]).await?;
            }
            _ => {
                println!("[tn3270][SEND] WONT 0x{:02X} (opción desconocida que el cliente nos pide habilitar)", opt);
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
        println!("[tn3270][RECV] DONT 0x{:02X}", opt);
        match opt {
            OPT_TERMINAL_TYPE | OPT_BINARY | OPT_EOR | OPT_TN3270E => {
                // Cliente nos pide (DONT) que deshabilitemos una opción que podríamos haber ofrecido (WILL).
                // O confirma nuestro WONT.
                println!("[tn3270][INFO] Cliente DONT 0x{:02X}. Confirmando con WONT.", opt);
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

        println!("[tn3270][INFO] Enviando TN3270E BIND-IMAGE comando ({} bytes)", tn3270e_header.len());
        println!("[tn3270][DEBUG] BIND-IMAGE data: {:02X?}", tn3270e_header);

        self.stream.write_all(&tn3270e_header).await?;
        self.stream.flush().await?;

        println!("[tn3270][INFO] TN3270E BIND-IMAGE comando enviado exitosamente");
        Ok(())
    }

    // Esta función ya no es necesaria aquí, el flujo es más dinámico.
    // async fn send_tn3270e_device_type_request(&mut self) -> Result<(), Box<dyn Error>> { ... }

    // Esta función ya no es necesaria aquí, se integra en el flujo de process_incoming_tn3270e_message
    // async fn handle_tn3270e(...) -> Result<(), Box<dyn Error>> { ... }

    // Función de ayuda para enviar datos 3270 (AID, cursor, etc.)
    // Esta es una simplificación. Un servidor real tendría una lógica más compleja.
    async fn send_3270_data(&mut self, _data: &[u8]) -> Result<(), Box<dyn Error>> {
        // Ejemplo: enviar un AID nulo (como un Enter pasivo) o datos de pantalla.
        // Por ahora, esta función es un placeholder.
        // La pantalla inicial se envía en maybe_send_screen.
        // Las respuestas a AID del cliente se manejarían aquí.
        println!("[tn3270][INFO] send_3270_data (placeholder) - datos recibidos del cliente (AID, etc.) serían procesados aquí.");

        // Como ejemplo, si recibimos un AID, podríamos reenviar la pantalla (o una actualizada).
        // Esto es muy simplificado.
        if self.tn3270e_bound || (!self.telnet_state.tn3270e_enabled && self.terminal_type.contains("327")) {
            // Reenviar la misma pantalla de bienvenida como ejemplo de respuesta.
            // En una app real, se generaría una nueva pantalla basada en la entrada.
            
            // Marcar screen_sent = false para forzar el reenvío de la pantalla de ejemplo.
            // ¡CUIDADO! Esto es solo para demostración y causaría un bucle si no se maneja bien.
            // self.screen_manager.reset_screen_sent(); 
            // self.maybe_send_screen().await?;

            // En lugar de reenviar, solo un log por ahora.
             println!("[tn3270][DEBUG] Simulación de procesamiento de AID y posible actualización de pantalla.");
        }
        Ok(())
    }
}

async fn handle_client(stream: TcpStream) -> Result<(), Box<dyn Error>> {
    println!("[tn3270][DEBUG] handle_client: nueva conexión");
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

    println!("[tn3270][SEND] Iniciando handshake Telnet:");
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
    println!("[tn3270][DEBUG] Handshake Telnet inicial enviado.");

    // No es necesario enviar IAC NOP aquí generalmente.
    // session.stream.write_all(&[IAC, NOP]).await?;
    // println!("[tn3270][DEBUG] Enviado IAC NOP tras handshake");

    // El timeout para el REQUEST DEVICE-TYPE proactivo se elimina.
    // La negociación se conducirá por el estado y las respuestas del cliente.

    loop {
        // tokio::select! permite esperar en múltiples futuros.
        // Aquí solo esperamos datos del cliente.
        // Podríamos añadir un timeout general de inactividad si fuera necesario.
        match session.stream.read_buf(&mut buf).await {
            Ok(0) => {
                // Solo cerrar la conexión si realmente no hay más datos
                // y no es solo que el cliente esté esperando respuesta
                if buf.is_empty() {
                    println!("[tn3270][INFO] Cliente desconectado (stream cerrado).");
                    break Ok(());
                } else {
                    // Si hay datos en el buffer, procesarlos
                    println!("[tn3270][DEBUG] Socket cerrado pero hay datos en buffer para procesar");
                    continue;
                }
            }
            Ok(n) => {
                println!("[tn3270][RECV] Raw data ({} bytes): {:02X?}", n, &buf[..n]);
                // Clonar los datos recibidos para procesarlos.
                // handle_negotiation puede necesitar ver todo el fragmento.
                let received_data = buf[..n].to_vec();
                buf.clear(); // Limpiar el buffer para la próxima lectura.
                
                if let Err(e) = session.handle_negotiation(&received_data).await {
                    eprintln!("[tn3270][ERROR] Error en handle_negotiation: {}", e);
                    // Decidir si cerrar la conexión o intentar continuar.
                    // Por ahora, cerramos en caso de error grave de negociación.
                    break Err(e);
                }
                
                // Después de procesar los datos, si la sesión está bound pero no se ha enviado la pantalla,
                // asegurarse de que se envíe
                if !session.screen_manager.screen_sent && (session.tn3270e_bound || 
                    (!session.telnet_state.tn3270e_enabled && session.telnet_state.base_telnet_ready)) {
                    if let Err(e) = session.maybe_send_screen().await {
                        eprintln!("[tn3270][ERROR] Error enviando pantalla: {}", e);
                        break Err(e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[tn3270][ERROR] Error de lectura del stream: {}", e);
                break Err(Box::new(e));
            }
        }
    }
}
pub struct Tn3270Handler;

#[async_trait]
impl ProtocolHandler for Tn3270Handler {
    async fn invoke_transaction(&self, transaction_id: &str, parameters: Value) -> Result<Value, String> {
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
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    println!("[tn3270] Listening on {} (async)", addr);
    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream).await {
                eprintln!("Client error: {}", e);
            }
        });
    }
    #[allow(unreachable_code)]
    Ok(())
}


