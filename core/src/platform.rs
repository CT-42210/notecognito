use serde::{Deserialize, Serialize};
use crate::error::Result;
use crate::notecard::NotecardId;
use crate::config::DisplayProperties;

/// Hotkey modifier keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HotkeyModifier {
    Control,
    Alt,
    Shift,
    #[cfg(target_os = "macos")]
    Command,
    #[cfg(target_os = "win")]
    Windows,
}

impl HotkeyModifier {
    /// Gets the platform-specific name for the modifier
    pub fn display_name(&self) -> &'static str {
        match self {
            HotkeyModifier::Control => {
                #[cfg(target_os = "macos")]
                return "⌃ Control";
                #[cfg(not(target_os = "macos"))]
                return "Ctrl";
            }
            HotkeyModifier::Alt => {
                #[cfg(target_os = "macos")]
                return "⌥ Option";
                #[cfg(not(target_os = "macos"))]
                return "Alt";
            }
            HotkeyModifier::Shift => {
                #[cfg(target_os = "macos")]
                return "⇧ Shift";
                #[cfg(not(target_os = "macos"))]
                return "Shift";
            }
            #[cfg(target_os = "macos")]
            HotkeyModifier::Command => "⌘ Command",
            #[cfg(target_os = "win")]
            HotkeyModifier::Windows => "⊞ Win",
        }
    }
}

/// Platform-specific interface that must be implemented for each OS
pub trait PlatformInterface: Send + Sync {
    /// Registers a global hotkey for a notecard
    fn register_hotkey(&mut self, id: NotecardId, modifiers: &[HotkeyModifier]) -> Result<()>;

    /// Unregisters a global hotkey for a notecard
    fn unregister_hotkey(&mut self, id: NotecardId) -> Result<()>;

    /// Shows a notecard overlay window
    fn show_notecard(&mut self, id: NotecardId, content: &str, properties: &DisplayProperties) -> Result<()>;

    /// Hides a notecard overlay window
    fn hide_notecard(&mut self, id: NotecardId) -> Result<()>;

    /// Sets the app to launch on startup
    fn set_launch_on_startup(&mut self, enabled: bool) -> Result<()>;

    /// Initializes the platform-specific components
    fn initialize(&mut self) -> Result<()>;

    /// Cleans up platform-specific resources
    fn cleanup(&mut self) -> Result<()>;

    /// Checks if the required permissions are granted (e.g., accessibility on macOS)
    fn check_permissions(&self) -> Result<bool>;

    /// Requests the required permissions from the user
    fn request_permissions(&self) -> Result<()>;
}

/// Platform detection helper
pub fn current_platform() -> &'static str {
    #[cfg(target_os = "macos")]
    return "macos";
    #[cfg(target_os = "win")]
    return "win";
    #[cfg(target_os = "linux")]
    return "linux";
    #[cfg(not(any(target_os = "macos", target_os = "win", target_os = "linux")))]
    return "unknown";
}