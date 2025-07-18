use thiserror::Error;

#[derive(Error, Debug)]
pub enum NotecognitoError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IPC error: {0}")]
    Ipc(String),

    #[error("Invalid notecard ID: {0}")]
    InvalidNotecardId(u8),

    #[error("Platform-specific error: {0}")]
    Platform(String),

    #[error("Connection lost")]
    ConnectionLost,

    #[error("Invalid message format")]
    InvalidMessage,

    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

pub type Result<T> = std::result::Result<T, NotecognitoError>;