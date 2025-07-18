use anyhow::{anyhow, Result};
use notecognito_core::{HotkeyModifier, NotecardId};
use std::collections::HashMap;
use std::sync::Arc;

pub struct HotkeyManager {
    registered_hotkeys: HashMap<NotecardId, Vec<HotkeyModifier>>,
    callback: Option<Arc<dyn Fn(NotecardId) + Send + Sync>>,
}

// Make HotkeyManager Send + Sync by removing non-thread-safe types
unsafe impl Send for HotkeyManager {}
unsafe impl Sync for HotkeyManager {}

impl HotkeyManager {
    pub fn new() -> Self {
        HotkeyManager {
            registered_hotkeys: HashMap::new(),
            callback: None,
        }
    }

    pub fn register_hotkey(
        &mut self,
        notecard_id: NotecardId,
        modifiers: &[HotkeyModifier],
    ) -> Result<()> {
        // Store the hotkey
        self.registered_hotkeys
            .insert(notecard_id, modifiers.to_vec());

        tracing::info!(
            "Registered hotkey for notecard {} with modifiers: {:?}",
            notecard_id.value(),
            modifiers
        );

        Ok(())
    }

    pub fn unregister_hotkey(&mut self, notecard_id: NotecardId) -> Result<()> {
        self.registered_hotkeys.remove(&notecard_id);
        Ok(())
    }

    pub fn unregister_all(&mut self) -> Result<()> {
        self.registered_hotkeys.clear();
        Ok(())
    }

    pub fn start_monitoring<F>(&mut self, callback: F) -> Result<()>
    where
        F: Fn(NotecardId) + Send + Sync + 'static,
    {
        self.callback = Some(Arc::new(callback));

        // Don't spawn a thread since we're not using any async operations
        // The callback will be called directly when a hotkey is triggered

        Ok(())
    }

    pub fn stop_monitoring(&mut self) {
        self.callback = None;
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        self.stop_monitoring();
    }
}