use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::error::{NotecognitoError, Result};
use crate::config::{Config, ConfigManager};
use crate::notecard::Notecard;

const IPC_PORT: u16 = 7855;
const MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1MB max message size

/// IPC message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IpcMessageType {
    GetConfiguration,
    UpdateNotecard { notecard: Notecard },
    SaveConfiguration { config: Config },
    ConfigurationResponse { config: Config },
    Success { message: String },
    Error { message: String },
}

/// IPC message wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcMessage {
    pub id: String,
    #[serde(flatten)]
    pub message_type: IpcMessageType,
}

impl IpcMessage {
    pub fn new(message_type: IpcMessageType) -> Self {
        use chrono::Utc;
        IpcMessage {
            id: format!("{}", Utc::now().timestamp_millis()),
            message_type,
        }
    }

    pub fn with_id(id: String, message_type: IpcMessageType) -> Self {
        IpcMessage { id, message_type }
    }
}

/// IPC server that handles communication with the configuration UI
pub struct IpcServer {
    config_manager: Arc<Mutex<ConfigManager>>,
}

impl IpcServer {
    /// Creates a new IPC server
    pub fn new(config_manager: Arc<Mutex<ConfigManager>>) -> Self {
        IpcServer { config_manager }
    }

    /// Starts the IPC server
    pub async fn start(&self) -> Result<()> {
        let addr = format!("127.0.0.1:{}", IPC_PORT);
        let listener = TcpListener::bind(&addr).await?;

        tracing::info!("IPC server listening on {}", addr);

        loop {
            let (stream, addr) = listener.accept().await?;
            tracing::debug!("New connection from {}", addr);

            let config_manager = Arc::clone(&self.config_manager);

            // Spawn a task to handle each connection
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, config_manager).await {
                    tracing::error!("Error handling connection: {}", e);
                }
            });
        }
    }
}

/// Handles a single client connection
async fn handle_connection(
    mut stream: TcpStream,
    config_manager: Arc<Mutex<ConfigManager>>,
) -> Result<()> {
    let mut buffer = vec![0; MAX_MESSAGE_SIZE];

    loop {
        // Read message length (4 bytes)
        let mut len_bytes = [0u8; 4];
        match stream.read_exact(&mut len_bytes).await {
            Ok(_) => {},
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                tracing::debug!("Client disconnected");
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        }

        let message_len = u32::from_le_bytes(len_bytes) as usize;

        if message_len > MAX_MESSAGE_SIZE {
            return Err(NotecognitoError::InvalidMessage);
        }

        // Read the message
        stream.read_exact(&mut buffer[..message_len]).await?;

        // Parse the message
        let message: IpcMessage = serde_json::from_slice(&buffer[..message_len])
            .map_err(|_| NotecognitoError::InvalidMessage)?;

        tracing::debug!("Received message: {:?}", message.message_type);

        // Process the message
        let response = process_message(message, &config_manager).await?;

        // Send the response
        send_message(&mut stream, &response).await?;
    }
}

/// Processes an incoming IPC message
async fn process_message(
    message: IpcMessage,
    config_manager: &Arc<Mutex<ConfigManager>>,
) -> Result<IpcMessage> {
    let response_type = match message.message_type {
        IpcMessageType::GetConfiguration => {
            let manager = config_manager.lock().await;
            IpcMessageType::ConfigurationResponse {
                config: manager.config().clone(),
            }
        }

        IpcMessageType::UpdateNotecard { notecard } => {
            let mut manager = config_manager.lock().await;
            match manager.update_notecard(notecard) {
                Ok(_) => {
                    manager.save()?;
                    IpcMessageType::Success {
                        message: "Notecard updated successfully".to_string(),
                    }
                }
                Err(e) => IpcMessageType::Error {
                    message: e.to_string(),
                },
            }
        }

        IpcMessageType::SaveConfiguration { config } => {
            let mut manager = config_manager.lock().await;
            *manager.config_mut() = config;
            match manager.save() {
                Ok(_) => IpcMessageType::Success {
                    message: "Configuration saved successfully".to_string(),
                },
                Err(e) => IpcMessageType::Error {
                    message: e.to_string(),
                },
            }
        }

        _ => IpcMessageType::Error {
            message: "Invalid message type".to_string(),
        },
    };

    Ok(IpcMessage::with_id(message.id, response_type))
}

/// Sends a message over the TCP stream
async fn send_message(stream: &mut TcpStream, message: &IpcMessage) -> Result<()> {
    let json = serde_json::to_vec(message)?;
    let len = json.len() as u32;

    // Write message length
    stream.write_all(&len.to_le_bytes()).await?;

    // Write message
    stream.write_all(&json).await?;
    stream.flush().await?;

    Ok(())
}

/// IPC client for testing and configuration UI
pub struct IpcClient {
    stream: TcpStream,
}

impl IpcClient {
    /// Connects to the IPC server
    pub async fn connect() -> Result<Self> {
        let addr = format!("127.0.0.1:{}", IPC_PORT);
        let stream = TcpStream::connect(&addr).await
            .map_err(|_| NotecognitoError::ConnectionLost)?;

        Ok(IpcClient { stream })
    }

    /// Sends a message and waits for a response
    pub async fn send_message(&mut self, message: IpcMessage) -> Result<IpcMessage> {
        // Send the message
        send_message(&mut self.stream, &message).await?;

        // Read response length
        let mut len_bytes = [0u8; 4];
        self.stream.read_exact(&mut len_bytes).await?;
        let message_len = u32::from_le_bytes(len_bytes) as usize;

        if message_len > MAX_MESSAGE_SIZE {
            return Err(NotecognitoError::InvalidMessage);
        }

        // Read the response
        let mut buffer = vec![0; message_len];
        self.stream.read_exact(&mut buffer).await?;

        // Parse the response
        let response: IpcMessage = serde_json::from_slice(&buffer)
            .map_err(|_| NotecognitoError::InvalidMessage)?;

        Ok(response)
    }
}