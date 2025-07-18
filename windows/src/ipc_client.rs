use anyhow::{anyhow, Result};
use notecognito_core::{Config, IpcMessage, IpcMessageType, Notecard};
use serde_json;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

const IPC_HOST: &str = "127.0.0.1";
const IPC_PORT: u16 = 7855;
const MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1MB

pub struct IpcClient {
    stream: Option<Arc<Mutex<TcpStream>>>,
}

impl IpcClient {
    pub fn new() -> Self {
        IpcClient { stream: None }
    }

    pub async fn connect(&mut self) -> Result<()> {
        let addr = format!("{}:{}", IPC_HOST, IPC_PORT);
        let stream = TcpStream::connect(&addr).await?;
        self.stream = Some(Arc::new(Mutex::new(stream)));
        tracing::info!("Connected to IPC server at {}", addr);
        Ok(())
    }

    pub async fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    pub async fn get_configuration(&mut self) -> Result<Config> {
        let message = IpcMessage::new(IpcMessageType::GetConfiguration);
        let response = self.send_message(message).await?;

        match response.message_type {
            IpcMessageType::ConfigurationResponse { config } => Ok(config),
            IpcMessageType::Error { message } => Err(anyhow!("Server error: {}", message)),
            _ => Err(anyhow!("Unexpected response type")),
        }
    }

    pub async fn update_notecard(&mut self, notecard: Notecard) -> Result<()> {
        let message = IpcMessage::new(IpcMessageType::UpdateNotecard { notecard });
        let response = self.send_message(message).await?;

        match response.message_type {
            IpcMessageType::Success { .. } => Ok(()),
            IpcMessageType::Error { message } => Err(anyhow!("Server error: {}", message)),
            _ => Err(anyhow!("Unexpected response type")),
        }
    }

    pub async fn save_configuration(&mut self, config: Config) -> Result<()> {
        let message = IpcMessage::new(IpcMessageType::SaveConfiguration { config });
        let response = self.send_message(message).await?;

        match response.message_type {
            IpcMessageType::Success { .. } => Ok(()),
            IpcMessageType::Error { message } => Err(anyhow!("Server error: {}", message)),
            _ => Err(anyhow!("Unexpected response type")),
        }
    }

    async fn send_message(&mut self, message: IpcMessage) -> Result<IpcMessage> {
        let stream = self.stream.as_ref()
            .ok_or_else(|| anyhow!("Not connected to IPC server"))?;

        let mut stream = stream.lock().await;

        // Serialize message
        let json = serde_json::to_vec(&message)?;
        let len = json.len() as u32;

        // Send length prefix
        stream.write_all(&len.to_le_bytes()).await?;

        // Send message
        stream.write_all(&json).await?;
        stream.flush().await?;

        // Read response length
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await?;
        let message_len = u32::from_le_bytes(len_bytes) as usize;

        if message_len > MAX_MESSAGE_SIZE {
            return Err(anyhow!("Response too large"));
        }

        // Read response
        let mut buffer = vec![0; message_len];
        stream.read_exact(&mut buffer).await?;

        // Parse response
        let response: IpcMessage = serde_json::from_slice(&buffer)?;
        Ok(response)
    }

    pub async fn disconnect(&mut self) {
        self.stream = None;
    }
}