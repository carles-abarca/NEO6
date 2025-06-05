use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use bytes::{BytesMut, BufMut};
use ebcdic::Ebcdic;
use std::error::Error;
use std::collections::HashMap;

// Estados de negociación Telnet
#[derive(Debug, Default)]
struct TelnetState {
    will_echo: bool,
    binary_mode: bool,
    tn3270e_enabled: bool,
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
struct Session {
    stream: TcpStream,
    telnet_state: TelnetState,
    tn3270e: TN3270EState,
    codec: Codec,
    screen_buffer: Vec<u8>,
    terminal_type: String,
    logical_unit: String,
}

impl Session {
    async fn new(stream: TcpStream) -> Self {
        Session {
            stream,
            telnet_state: TelnetState::default(),
            tn3270e: TN3270EState::default(),
            codec: Codec::new(),
            screen_buffer: vec![0x00; 1920], // 80x24
            terminal_type: String::new(),
            logical_unit: "DEFAULT".to_string(),
        }
    }

    async fn handle_negotiation(&mut self, buf: &[u8]) -> Result<(), Box<dyn Error>> {
        let mut i = 0;
        while i < buf.len() {
            if buf[i] == 0xFF { // IAC
                if i + 2 >= buf.len() { break; }
                
                let cmd = buf[i+1];
                let opt = buf[i+2];
                
                match cmd {
                    0xFB => self.handle_will(opt).await?,
                    0xFD => self.handle_do(opt).await?,
                    0xFA => self.handle_subnegotiation(&buf[i..]).await?,
                    _ => {}
                }
                i += 3;
            } else {
                i += 1;
            }
        }
        Ok(())
    }

    async fn handle_will(&mut self, opt: u8) -> Result<(), Box<dyn Error>> {
        match opt {
            0x18 => { // TERMINAL-TYPE
                self.stream.write_all(&[0xFF, 0xFD, 0x18]).await?;
            },
            0x19 => { // EOR
                self.stream.write_all(&[0xFF, 0xFD, 0x19]).await?;
            },
            0x28 => { // TN3270E
                self.telnet_state.tn3270e_enabled = true;
                self.stream.write_all(&[0xFF, 0xFB, 0x28]).await?;
            },
            _ => {}
        }
        Ok(())
    }

    async fn handle_do(&mut self, opt: u8) -> Result<(), Box<dyn Error>> {
        match opt {
            0x00 => { // BINARY
                self.stream.write_all(&[0xFF, 0xFB, 0x00]).await?;
                self.telnet_state.binary_mode = true;
            },
            _ => {}
        }
        Ok(())
    }

    async fn handle_subnegotiation(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        if data.len() < 3 { return Ok(()); }
        
        let subopt = data[1];
        match subopt {
            0x28 => self.handle_tn3270e(&data[2..]).await?,
            0x18 => self.handle_terminal_type(&data[2..]).await?,
            _ => {}
        }
        Ok(())
    }

    async fn handle_tn3270e(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        if data.is_empty() { return Ok(()); }
        
        let data_type = data[0];
        match data_type {
            0x00 => self.handle_data(data).await?,
            0x02 => self.handle_device_type(data).await?,
            0x04 => self.handle_bind_image(data).await?,
            _ => {}
        }
        Ok(())
    }

    async fn handle_data(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        // Procesar datos 3270
        let converted = self.codec.to_host(&data[2..]);
        // Actualizar buffer de pantalla
        self.screen_buffer.splice(..converted.len(), converted.iter().cloned());
        Ok(())
    }

    async fn handle_device_type(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        self.terminal_type = String::from_utf8_lossy(data).to_string();
        let response = [0xFF, 0xFA, 0x28, 0x00, 0x02, 0xFF, 0xF0];
        self.stream.write_all(&response).await?;
        Ok(())
    }

    async fn handle_bind_image(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        self.tn3270e.bind_image = data.to_vec();
        Ok(())
    }

    async fn send_screen(&mut self) -> Result<(), Box<dyn Error>> {
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
            response.put_u8(0xE0); // EOR
        }
        
        self.stream.write_all(&response).await?;
        Ok(())
    }
}

async fn handle_client(stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let mut session = Session::new(stream).await;
    let mut buf = BytesMut::with_capacity(1024);

    // Negociación inicial
    session.stream.write_all(&[
        0xFF, 0xFD, 0x18, // DO TERMINAL-TYPE
        0xFF, 0xFD, 0x00, // DO BINARY
        0xFF, 0xFD, 0x19, // DO EOR
        0xFF, 0xFB, 0x28  // WILL TN3270E
    ]).await?;

    loop {
        let n = session.stream.read_buf(&mut buf).await?;
        if n == 0 { break; }

        session.handle_negotiation(&buf[..n]).await?;
        
        if session.telnet_state.binary_mode {
            session.send_screen().await?;
        }

        buf.clear();
    }
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

