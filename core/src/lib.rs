pub mod config;
pub mod notecard;
pub mod ipc;
pub mod platform;
pub mod error;

#[cfg(feature = "ffi")]
pub mod ffi;

pub use config::{Config, ConfigManager, DisplayProperties};
pub use notecard::{Notecard, NotecardId};
pub use ipc::{IpcServer, IpcMessage, IpcMessageType};
pub use platform::{PlatformInterface, HotkeyModifier};
pub use error::{NotecognitoError, Result};

// Re-export commonly used items
pub mod prelude {
    pub use crate::config::*;
    pub use crate::notecard::*;
    pub use crate::error::Result;
}