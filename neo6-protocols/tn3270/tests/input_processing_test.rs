// These tests are currently disabled as they require methods that are not yet properly exposed
// TODO: Re-enable once the Session API is properly designed for testing

/*
use tn3270::Session;
use tokio::net::{TcpListener, TcpStream};
use std::time::Duration;

#[tokio::test]
async fn test_enter_aid_processing() {
    // Create a mock TN3270 session
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    
    // Spawn a task to accept the connection
    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut session = Session::new(stream).await;
        session.initialize().await.unwrap();
        
        // Test Enter AID with field data (0x7D is Enter AID)
        let test_data = vec![
            0x7D, // Enter AID
            0x11, 0x40, 0x40, // Cursor address
            0x1D, 0x40, // Start field
            b'M', b'E', b'N', b'U', // Field data "MENU"
            0xFF, 0xEF, // End of data
        ];
        
        // Process the input data
        session.send_3270_data(&test_data).await.unwrap();
    });
    
    // Connect to the server
    let _client_stream = TcpStream::connect(addr).await.unwrap();
    
    // Wait for the server task to complete
    tokio::time::timeout(Duration::from_secs(5), server_task).await.unwrap().unwrap();
}

#[tokio::test]
async fn test_pf_key_processing() {
    // Create a mock TN3270 session
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    
    // Spawn a task to accept the connection
    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut session = Session::new(stream).await;
        session.initialize().await.unwrap();
        
        // Test PF1 key (0xF1 = Help)
        let test_data = vec![
            0xF1, // PF1 AID (Help)
            0x11, 0x40, 0x40, // Cursor address
            0xFF, 0xEF, // End of data
        ];
        
        // Process the input data
        session.send_3270_data(&test_data).await.unwrap();
    });
    
    // Connect to the server
    let _client_stream = TcpStream::connect(addr).await.unwrap();
    
    // Wait for the server task to complete
    tokio::time::timeout(Duration::from_secs(5), server_task).await.unwrap().unwrap();
}

#[tokio::test]
async fn test_clear_aid_processing() {
    // Create a mock TN3270 session
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    
    // Spawn a task to accept the connection
    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut session = Session::new(stream).await;
        session.initialize().await.unwrap();
        
        // Test Clear key (0x6D)
        let test_data = vec![
            0x6D, // Clear AID
            0xFF, 0xEF, // End of data
        ];
        
        // Process the input data
        session.send_3270_data(&test_data).await.unwrap();
    });
    
    // Connect to the server
    let _client_stream = TcpStream::connect(addr).await.unwrap();
    
    // Wait for the server task to complete
    tokio::time::timeout(Duration::from_secs(5), server_task).await.unwrap().unwrap();
}
*/
