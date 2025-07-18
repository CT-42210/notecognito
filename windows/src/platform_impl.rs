use anyhow::Result;
use notecognito_core::{
    DisplayProperties, HotkeyModifier, NotecardId, PlatformInterface,
};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::hotkey::HotkeyManager;
use crate::notecard_window::NotecardWindowManager;

pub struct WindowsPlatform {
    hotkey_manager: Arc<Mutex<HotkeyManager>>,
    window_manager: Arc<Mutex<NotecardWindowManager>>,
    initialized: bool,
}

impl WindowsPlatform {
    pub fn new(
        hotkey_manager: Arc<Mutex<HotkeyManager>>,
        window_manager: Arc<Mutex<NotecardWindowManager>>,
    ) -> Self {
        WindowsPlatform {
            hotkey_manager,
            window_manager,
            initialized: false,
        }
    }
}

impl PlatformInterface for WindowsPlatform {
    fn register_hotkey(
        &mut self,
        id: NotecardId,
        modifiers: &[HotkeyModifier],
    ) -> notecognito_core::Result<()> {
        // Use tokio runtime to run async code
        let hotkey_manager = Arc::clone(&self.hotkey_manager);
        let modifiers = modifiers.to_vec();

        let result = tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async move {
                let mut manager = hotkey_manager.lock().await;
                manager.register_hotkey(id, &modifiers)
            })
        });

        result.map_err(|e| notecognito_core::NotecognitoError::Platform(e.to_string()))
    }

    fn unregister_hotkey(&mut self, id: NotecardId) -> notecognito_core::Result<()> {
        let hotkey_manager = Arc::clone(&self.hotkey_manager);

        let result = tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async move {
                let mut manager = hotkey_manager.lock().await;
                manager.unregister_hotkey(id)
            })
        });

        result.map_err(|e| notecognito_core::NotecognitoError::Platform(e.to_string()))
    }

    fn show_notecard(
        &mut self,
        id: NotecardId,
        content: &str,
        properties: &DisplayProperties,
    ) -> notecognito_core::Result<()> {
        let window_manager = Arc::clone(&self.window_manager);
        let content = content.to_string();
        let properties = properties.clone();

        let result = tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async move {
                let mut manager = window_manager.lock().await;
                manager.show_notecard(id, &content, &properties)
            })
        });

        result.map_err(|e| notecognito_core::NotecognitoError::Platform(e.to_string()))
    }

    fn hide_notecard(&mut self, id: NotecardId) -> notecognito_core::Result<()> {
        let window_manager = Arc::clone(&self.window_manager);

        let result = tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async move {
                let mut manager = window_manager.lock().await;
                manager.hide_notecard(id)
            })
        });

        result.map_err(|e| notecognito_core::NotecognitoError::Platform(e.to_string()))
    }

    fn set_launch_on_startup(&mut self, enabled: bool) -> notecognito_core::Result<()> {
        use windows::Win32::System::Registry::*;
        use windows::Win32::Foundation::*;

        unsafe {
            let key_path = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
            let mut hkey = HKEY::default();

            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                key_path,
                0,
                KEY_SET_VALUE,
                &mut hkey,
            ).map_err(|e| notecognito_core::NotecognitoError::Platform(e.to_string()))?;

            let result = if enabled {
                let exe_path = std::env::current_exe()
                    .map_err(|e| notecognito_core::NotecognitoError::Platform(e.to_string()))?;
                let exe_path = exe_path.to_string_lossy();
                let value = format!("\"{}\"", exe_path);

                RegSetValueExW(
                    hkey,
                    w!("Notecognito"),
                    0,
                    REG_SZ,
                    Some(value.as_bytes()),
                ).map_err(|e| notecognito_core::NotecognitoError::Platform(e.to_string()))
            } else {
                match RegDeleteValueW(hkey, w!("Notecognito")) {
                    Ok(_) => Ok(()),
                    Err(e) if e.code() == ERROR_FILE_NOT_FOUND.to_hresult() => Ok(()),
                    Err(e) => Err(notecognito_core::NotecognitoError::Platform(e.to_string())),
                }
            };

            RegCloseKey(hkey)
                .map_err(|e| notecognito_core::NotecognitoError::Platform(e.to_string()))?;

            result
        }
    }

    fn initialize(&mut self) -> notecognito_core::Result<()> {
        if self.initialized {
            return Ok(());
        }

        self.initialized = true;
        Ok(())
    }

    fn cleanup(&mut self) -> notecognito_core::Result<()> {
        let hotkey_manager = Arc::clone(&self.hotkey_manager);

        let result = tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async move {
                let mut manager = hotkey_manager.lock().await;
                manager.unregister_all()
            })
        });

        result.map_err(|e| notecognito_core::NotecognitoError::Platform(e.to_string()))
    }

    fn check_permissions(&self) -> notecognito_core::Result<bool> {
        // Windows doesn't require special permissions for hotkeys or overlays
        Ok(true)
    }

    fn request_permissions(&self) -> notecognito_core::Result<()> {
        // No permissions needed on Windows
        Ok(())
    }
}