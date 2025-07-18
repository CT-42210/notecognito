use anyhow::{anyhow, Result};
use notecognito_core::{HotkeyModifier, NotecardId};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use windows::Win32::{
    Foundation::*,
    UI::Input::KeyboardAndMouse::*,
    UI::WindowsAndMessaging::*,
};

const HOTKEY_BASE_ID: i32 = 1000;

pub struct HotkeyManager {
    registered_hotkeys: HashMap<NotecardId, i32>,
    message_thread: Option<thread::JoinHandle<()>>,
}

impl HotkeyManager {
    pub fn new() -> Self {
        HotkeyManager {
            registered_hotkeys: HashMap::new(),
            message_thread: None,
        }
    }

    pub fn register_hotkey(
        &mut self,
        notecard_id: NotecardId,
        modifiers: &[HotkeyModifier],
    ) -> Result<()> {
        // Convert modifiers to Windows format
        let mut win_modifiers = HOT_KEY_MODIFIERS::default();

        for modifier in modifiers {
            win_modifiers |= match modifier {
                HotkeyModifier::Control => MOD_CONTROL,
                HotkeyModifier::Alt => MOD_ALT,
                HotkeyModifier::Shift => MOD_SHIFT,
                HotkeyModifier::Windows => MOD_WIN,
            };
        }

        // Virtual key code for numbers 1-9
        let vk_code = VIRTUAL_KEY((0x30 + notecard_id.value()) as u16);

        // Generate unique ID for this hotkey
        let hotkey_id = HOTKEY_BASE_ID + notecard_id.value() as i32;

        // Register the hotkey
        unsafe {
            if !RegisterHotKey(HWND::default(), hotkey_id, win_modifiers, vk_code).as_bool() {
                return Err(anyhow!("Failed to register hotkey for notecard {}", notecard_id.value()));
            }
        }

        self.registered_hotkeys.insert(notecard_id, hotkey_id);
        tracing::info!("Registered hotkey for notecard {}", notecard_id.value());

        Ok(())
    }

    pub fn unregister_hotkey(&mut self, notecard_id: NotecardId) -> Result<()> {
        if let Some(hotkey_id) = self.registered_hotkeys.remove(&notecard_id) {
            unsafe {
                UnregisterHotKey(HWND::default(), hotkey_id)?;
            }
        }
        Ok(())
    }

    pub fn unregister_all(&mut self) -> Result<()> {
        for (_, hotkey_id) in self.registered_hotkeys.drain() {
            unsafe {
                let _ = UnregisterHotKey(HWND::default(), hotkey_id);
            }
        }
        Ok(())
    }

    pub fn start_message_loop<F>(&mut self, callback: F) -> Result<()>
    where
        F: Fn(NotecardId) + Send + 'static,
    {
        let callback = Arc::new(callback);

        let handle = thread::spawn(move || {
            unsafe {
                let mut msg = MSG::default();

                loop {
                    let result = GetMessageW(&mut msg, HWND::default(), 0, 0);

                    if result.0 == -1 {
                        tracing::error!("GetMessage failed");
                        break;
                    }

                    if result.0 == 0 {
                        // WM_QUIT received
                        break;
                    }

                    if msg.message == WM_HOTKEY {
                        let hotkey_id = msg.wParam.0 as i32;
                        let notecard_number = (hotkey_id - HOTKEY_BASE_ID) as u8;

                        if let Ok(notecard_id) = NotecardId::new(notecard_number) {
                            callback(notecard_id);
                        }
                    }

                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        });

        self.message_thread = Some(handle);
        Ok(())
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        let _ = self.unregister_all();
    }
}