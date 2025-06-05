use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use bytes::{BytesMut, BufMut};
use ebcdic::ebcdic::Ebcdic;
use std::error::Error;
use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::Value;
use serde_json::json;
use tokio::runtime::Runtime;
use neo6_protocols_lib::protocol::{ProtocolHandler, TransactionConfig};

// Estados de negociación Telnet
#[derive(Debug, Default)]
struct TelnetState {
    will_echo: bool,
    binary_mode: bool,
    tn3270e_enabled: bool,
    termtype_negotiated: bool, // nuevo flag
    binary_negotiated: bool,   // nuevo flag
    eor_negotiated: bool,      // nuevo flag
    tn3270e_negotiated: bool,  // nuevo flag
}

// Estado TN3270E
#[derive(Debug, Default)]
struct TN3270EState {
    device_type: String,
    functions: u8,
    bind_image: Vec<u8>,
    device_status: u8,
    pending_commands: HashMap<u16, Vec<u8>>,
    sequence_number: u16,
}

// Conversión de caracteres
#[derive(Debug)]
struct Codec {
    use_ebcdic: bool,
    code_page: String,
}

impl Codec {
    fn new() -> Self {
        Codec {
            use_ebcdic: true,
            code_page: "CP037".to_string(),
        }
    }

    fn to_host(&self, data: &[u8]) -> Vec<u8> {
        if self.use_ebcdic {
            let mut output = vec![0u8; data.len()];
            Ebcdic::ebcdic_to_ascii(data, &mut output, data.len(), false, true);
            output
        } else {
            data.to_vec()
        }
    }

    fn from_host(&self, data: &[u8]) -> Vec<u8> {
        if self.use_ebcdic {
            let mut output = vec![0u8; data.len()];
            Ebcdic::ascii_to_ebcdic(data, &mut output, data.len(), true);
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
    screen_buffer: Vec<u8>,
    terminal_type: String,
    logical_unit: String,
    screen_sent: bool, // nuevo flag
    tn3270e_bound: bool, // nuevo flag: true tras DEVICE TYPE o BIND IMAGE
}

impl Session {
    async fn new(stream: TcpStream) -> Self {
        println!("[tn3270][DEBUG] Nueva sesión creada");
        Session {
            stream,
            telnet_state: TelnetState::default(),
            tn3270e: TN3270EState::default(),
            codec: Codec::new(),
            screen_buffer: vec![0x00; 1920], // 80x24
            terminal_type: String::new(),
            logical_unit: "DEFAULT".to_string(),
            screen_sent: false,
            tn3270e_bound: false,
        }
    }

    async fn maybe_send_screen(&mut self) -> Result<(), Box<dyn Error>> {
        if self.telnet_state.binary_mode && self.telnet_state.tn3270e_enabled && self.tn3270e_bound && !self.screen_sent {
            println!("[tn3270][DEBUG] Ambos flags y BIND activos, enviando pantalla (solo una vez)");
            self.send_screen().await?;
            self.screen_sent = true;
        }
        Ok(())
    }

    async fn handle_negotiation(&mut self, buf: &[u8]) -> Result<(), Box<dyn Error>> {
        println!("[tn3270][DEBUG] handle_negotiation: {:02X?}", buf);
        let mut i = 0;
        while i < buf.len() {
            if buf[i] == 0xFF { // IAC
                if i + 1 >= buf.len() { break; }
                let cmd = buf[i+1];
                if cmd == 0xFA { // SB
                    // Busca el final de subnegociación (IAC SE)
                    if let Some(se_idx) = buf[i+2..].windows(2).position(|w| w == [0xFF, 0xF0]) {
                        let end = i + 2 + se_idx + 2;
                        let sub_data = &buf[i..end];
                        println!("[tn3270][DEBUG] Subnegociación detectada: {:02X?}", sub_data);
                        self.handle_subnegotiation(sub_data).await?;
                        i = end;
                        self.maybe_send_screen().await?;
                        continue;
                    } else {
                        println!("[tn3270][DEBUG] Subnegociación incompleta, esperando más datos");
                        break;
                    }
                }
                if i + 2 >= buf.len() { break; }
                let opt = buf[i+2];
                println!("[tn3270][DEBUG] Telnet cmd: 0x{:02X} opt: 0x{:02X}", cmd, opt);
                match cmd {
                    0xFB => { self.handle_will(opt).await?; self.maybe_send_screen().await?; },
                    0xFD => { self.handle_do(opt).await?; self.maybe_send_screen().await?; },
                    _ => {}
                }
                i += 3;
            } else {
                i += 1;
            }
        }
        // Chequea flags tras procesar todo el buffer
        println!("[tn3270][DEBUG] Estado negociación: binary_negotiated={}, eor_negotiated={}, tn3270e_negotiated={}, termtype_negotiated={}",
            self.telnet_state.binary_negotiated,
            self.telnet_state.eor_negotiated,
            self.telnet_state.tn3270e_negotiated,
            self.telnet_state.termtype_negotiated
        );
        self.maybe_send_screen().await?;
        Ok(())
    }

    async fn handle_will(&mut self, opt: u8) -> Result<(), Box<dyn Error>> {
        println!("[tn3270][DEBUG] handle_will: opt=0x{:02X}", opt);
        match opt {
            0x00 => { // BINARY
                println!("[tn3270][DEBUG] WILL BINARY");
                self.stream.write_all(&[0xFF, 0xFD, 0x00]).await?; // DO BINARY
                self.telnet_state.binary_mode = true;
                self.telnet_state.binary_negotiated = true;
            },
            0x18 => { // TERMINAL-TYPE
                println!("[tn3270][DEBUG] WILL TERMINAL-TYPE");
                self.stream.write_all(&[0xFF, 0xFD, 0x18]).await?;
                // Enviar IAC SB TERMINAL-TYPE SEND IAC SE
                let send_termtype = [0xFF, 0xFA, 0x18, 0x01, 0xFF, 0xF0];
                println!("[tn3270][DEBUG] Enviando TERMINAL-TYPE SEND: {:02X?}", send_termtype);
                self.stream.write_all(&send_termtype).await?;
                self.telnet_state.termtype_negotiated = true;
            },
            0x19 => { // EOR
                println!("[tn3270][DEBUG] WILL EOR");
                self.stream.write_all(&[0xFF, 0xFD, 0x19]).await?;
                self.telnet_state.eor_negotiated = true;
            },
            0x28 => { // TN3270E
                println!("[tn3270][DEBUG] WILL TN3270E");
                self.telnet_state.tn3270e_enabled = true;
                self.stream.write_all(&[0xFF, 0xFB, 0x28]).await?; // WILL TN3270E
                self.stream.write_all(&[0xFF, 0xFD, 0x28]).await?; // DO TN3270E (simetría extra)
                self.telnet_state.tn3270e_negotiated = true;
            },
            _ => {}
        }
        Ok(())
    }

    async fn handle_do(&mut self, opt: u8) -> Result<(), Box<dyn Error>> {
        println!("[tn3270][DEBUG] handle_do: opt=0x{:02X}", opt);
        match opt {
            0x00 => { // BINARY
                println!("[tn3270][DEBUG] DO BINARY");
                self.stream.write_all(&[0xFF, 0xFB, 0x00]).await?;
                self.telnet_state.binary_mode = true;
                self.telnet_state.binary_negotiated = true;
            },
            0x28 => { // TN3270E
                println!("[tn3270][DEBUG] DO TN3270E");
                self.stream.write_all(&[0xFF, 0xFB, 0x28]).await?;
                self.telnet_state.tn3270e_enabled = true;
                self.telnet_state.tn3270e_negotiated = true;
            },
            0x18 => { // TERMINAL-TYPE
                println!("[tn3270][DEBUG] DO TERMINAL-TYPE");
                self.stream.write_all(&[0xFF, 0xFB, 0x18]).await?;
                self.telnet_state.termtype_negotiated = true;
            },
            0x19 => { // EOR
                println!("[tn3270][DEBUG] DO EOR");
                self.stream.write_all(&[0xFF, 0xFB, 0x19]).await?;
                self.telnet_state.eor_negotiated = true;
            },
            _ => {}
        }
        Ok(())
    }

    async fn handle_subnegotiation(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        println!("[tn3270][DEBUG] handle_subnegotiation: {:02X?}", data);
        // Corrige el parsing: data[2]=subopt, data[3]=comando
        if data.len() >= 4 && data[2] == 0x18 {
            // TERMINAL-TYPE subnegociation
            if data[3] == 0x01 {
                // SEND
                if self.terminal_type.is_empty() {
                    let mut response = vec![0xFF, 0xFA, 0x18, 0x00];
                    response.extend_from_slice(b"IBM-3278-2");
                    response.push(0xFF);
                    response.push(0xF0);
                    println!("[tn3270][DEBUG] Respondiendo TERMINAL-TYPE IS: {:02X?}", response);
                    self.stream.write_all(&response).await?;
                }
            } else if data[3] == 0x00 {
                // IS: [FF, FA, 18, 00, ...terminal..., FF, F0]
                let mut end = data.len();
                if end >= 2 && data[end-2] == 0xFF && data[end-1] == 0xF0 {
                    end -= 2;
                }
                let termtype = String::from_utf8_lossy(&data[4..end]).to_string();
                println!("[tn3270][DEBUG] TERMINAL-TYPE IS recibido: {}", termtype);
                self.terminal_type = termtype;
            } else {
                println!("[tn3270][DEBUG] Subnegociación TERMINAL-TYPE desconocida: {:02X?}", data);
            }
        } else if data.len() >= 3 && data[2] == 0x28 {
            // TN3270E subnegociation
            self.handle_tn3270e(&data[3..]).await?;
        } else {
            println!("[tn3270][DEBUG] Subnegociación desconocida: {:02X?}", data);
        }
        Ok(())
    }

    async fn handle_tn3270e(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        println!("[tn3270][DEBUG] handle_tn3270e: {:02X?}", data);
        if data.is_empty() { return Ok(()); }
        let data_type = data[0];
        match data_type {
            0x00 => self.handle_data(data).await?,
            0x02 => {
                // REQUEST DEVICE-TYPE
                println!("[tn3270][DEBUG] TN3270E REQUEST DEVICE-TYPE recibido");
                // Responder con DEVICE-TYPE IS
                let mut response = vec![0xFF, 0xFA, 0x28, 0x00, 0x02];
                response.extend_from_slice(b"IBM-3278-2");
                response.push(0xFF);
                response.push(0xF0);
                println!("[tn3270][DEBUG] Respondiendo TN3270E DEVICE-TYPE IS: {:02X?}", response);
                self.stream.write_all(&response).await?;
                self.stream.flush().await?;
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                let mut bind_image = vec![0xFF, 0xFA, 0x28, 0x04, 0x00, 0xFF, 0xF0];
                println!("[tn3270][DEBUG] Enviando TN3270E BIND-IMAGE con 0x00: {:02X?}", bind_image);
                self.stream.write_all(&bind_image).await?;
                self.stream.flush().await?;
                self.tn3270e_bound = true;
                self.maybe_send_screen().await?;
            },
            0x04 => self.handle_bind_image(data).await?,
            _ => {}
        }
        Ok(())
    }

    async fn handle_data(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        println!("[tn3270][DEBUG] handle_data: {:02X?}", data);
        let converted = self.codec.to_host(&data[2..]);
        self.screen_buffer.splice(..converted.len(), converted.iter().cloned());
        Ok(())
    }

    async fn handle_device_type(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        println!("[tn3270][DEBUG] handle_device_type: {:?}", data);
        self.terminal_type = String::from_utf8_lossy(data).to_string();
        let response = [0xFF, 0xFA, 0x28, 0x00, 0x02, 0xFF, 0xF0];
        self.stream.write_all(&response).await?;
        self.tn3270e_bound = true;
        self.maybe_send_screen().await?;
        Ok(())
    }

    async fn handle_bind_image(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        println!("[tn3270][DEBUG] handle_bind_image: {:02X?}", data);
        self.tn3270e.bind_image = data.to_vec();
        self.tn3270e_bound = true;
        self.maybe_send_screen().await?;
        Ok(())
    }

    async fn send_screen(&mut self) -> Result<(), Box<dyn Error>> {
        println!("[tn3270][DEBUG] send_screen: preparando buffer de pantalla");
        let converted = self.codec.from_host(&self.screen_buffer);
        let mut response = BytesMut::new();
        if self.telnet_state.tn3270e_enabled {
            response.put_u8(0x00); // Data type: 3270 data
            response.put_u16(self.tn3270e.sequence_number);
            self.tn3270e.sequence_number = self.tn3270e.sequence_number.wrapping_add(1);
        }
        response.put_u8(0xF5); // Write command
        response.put_u8(0xC0); // WCC
        response.put_slice(&converted);
        if self.telnet_state.tn3270e_enabled {
            response.put_u8(0xFF); // IAC
            response.put_u8(0xEF); // EOR (corregido: 0xEF)
        }
        // Traza hex completa
        println!("[tn3270][DEBUG] Enviando buffer 3270 ({} bytes): {}", response.len(), response.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" "));
        self.stream.write_all(&response).await?;
        Ok(())
    }
}

async fn handle_client(stream: TcpStream) -> Result<(), Box<dyn Error>> {
    println!("[tn3270][DEBUG] handle_client: nueva conexión");
    let mut session = Session::new(stream).await;
    let mut buf = BytesMut::with_capacity(1024);
    session.stream.write_all(&[
        0xFF, 0xFD, 0x18, // DO TERMINAL-TYPE
        0xFF, 0xFD, 0x00, // DO BINARY
        0xFF, 0xFD, 0x19, // DO EOR
        0xFF, 0xFD, 0x28  // DO TN3270E
    ]).await?;
    println!("[tn3270][DEBUG] handshake inicial enviado");
    session.stream.write_all(&[0xFF, 0xF1]).await?;
    println!("[tn3270][DEBUG] Enviado IAC NOP tras handshake");

    let mut sent_proactive = false;
    let mut timeout = tokio::time::sleep(std::time::Duration::from_secs(1));
    tokio::pin!(timeout);
    loop {
        tokio::select! {
            _ = &mut timeout, if !sent_proactive => {
                if session.telnet_state.tn3270e_enabled && session.telnet_state.tn3270e_negotiated && session.terminal_type.starts_with("IBM-3278") && !session.tn3270e_bound {
                    println!("[tn3270][DEBUG] Timeout: enviando REQUEST DEVICE-TYPE proactivo (workaround c3270/x3270)");
                    let mut req = vec![0xFF, 0xFA, 0x28, 0x02];
                    req.extend_from_slice(b"IBM-3278-2");
                    req.push(0xFF);
                    req.push(0xF0);
                    let _ = session.stream.write_all(&req).await;
                }
                sent_proactive = true;
            }
            n = session.stream.read_buf(&mut buf) => {
                let n = n?;
                if n == 0 { println!("[tn3270][DEBUG] Cliente desconectado"); break Ok(()); }
                println!("[tn3270][DEBUG] Recibido {} bytes: {:02X?}", n, &buf[..n]);
                session.handle_negotiation(&buf[..n]).await?;
                buf.clear();
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("0.0.0.0:2323").await?;
    println!("TN3270E server listening on port 2323...");

    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream).await {
                eprintln!("Client error: {}", e);
            }
        });
    }
}
