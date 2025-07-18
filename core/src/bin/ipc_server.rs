use notecognito_core::{ConfigManager, IpcServer};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    tracing::info!("Starting Notecognito IPC Server");

    // Create configuration manager
    let config_manager = ConfigManager::new()?;
    let config_manager = Arc::new(Mutex::new(config_manager));

    // Create and start IPC server
    let ipc_server = IpcServer::new(config_manager);

    tracing::info!("IPC Server initialized, starting to listen for connections...");

    // Run the server
    ipc_server.start().await?;

    Ok(())
}