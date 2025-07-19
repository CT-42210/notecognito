use anyhow::{anyhow, Result};
use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes};
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventType, EventField,
};
use notecognito_core::{HotkeyModifier, NotecardId};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use once_cell::sync::Lazy;

// Global state for the event tap callback
static HOTKEY_STATE: Lazy<Arc<Mutex<HotkeyState>>> = Lazy::new(|| {
    Arc::new(Mutex::new(HotkeyState {
        hotkeys: HashMap::new(),
        callback: None,
    }))
});

struct HotkeyState {
    hotkeys: HashMap<NotecardId, Vec<HotkeyModifier>>,
    callback: Option<Arc<dyn Fn(NotecardId) + Send + Sync>>,
}

pub struct HotkeyManager {
    monitoring: Arc<Mutex<bool>>,
    event_tap_thread: Option<thread::JoinHandle<()>>,
}

unsafe impl Send for HotkeyManager {}
unsafe impl Sync for HotkeyManager {}

impl HotkeyManager {
    pub fn new() -> Self {
        HotkeyManager {
            monitoring: Arc::new(Mutex::new(false)),
            event_tap_thread: None,
        }
    }

    pub fn register_hotkey(
        &mut self,
        notecard_id: NotecardId,
        modifiers: &[HotkeyModifier],
    ) -> Result<()> {
        let mut state = HOTKEY_STATE.lock().unwrap();
        state.hotkeys.insert(notecard_id, modifiers.to_vec());

        tracing::info!(
            "Registered hotkey for notecard {} with modifiers: {:?}",
            notecard_id.value(),
            modifiers
        );

        Ok(())
    }

    pub fn unregister_hotkey(&mut self, notecard_id: NotecardId) -> Result<()> {
        let mut state = HOTKEY_STATE.lock().unwrap();
        state.hotkeys.remove(&notecard_id);
        Ok(())
    }

    pub fn unregister_all(&mut self) -> Result<()> {
        let mut state = HOTKEY_STATE.lock().unwrap();
        state.hotkeys.clear();
        Ok(())
    }

    pub fn start_monitoring<F>(&mut self, callback: F) -> Result<()>
    where
        F: Fn(NotecardId) + Send + Sync + 'static,
    {
        // Check if already monitoring
        {
            let monitoring = self.monitoring.lock().unwrap();
            if *monitoring {
                return Ok(());
            }
        }

        // Check accessibility permissions first
        if !Self::check_accessibility_permissions() {
            return Err(anyhow!("Accessibility permissions not granted. Please grant permissions in System Preferences > Security & Privacy > Privacy > Accessibility"));
        }

        // Store callback in global state
        {
            let mut state = HOTKEY_STATE.lock().unwrap();
            state.callback = Some(Arc::new(callback));
        }

        let monitoring = Arc::clone(&self.monitoring);

        // Start event tap in a separate thread
        let handle = thread::spawn(move || {
            if let Err(e) = Self::run_event_tap(monitoring) {
                tracing::error!("Event tap error: {}", e);
            }
        });

        self.event_tap_thread = Some(handle);

        Ok(())
    }

    fn check_accessibility_permissions() -> bool {
        use core_foundation::base::{Boolean, TCFType};
        use core_foundation::dictionary::CFDictionary;
        use core_foundation::string::CFString;

        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXIsProcessTrustedWithOptions(options: core_foundation::dictionary::CFDictionaryRef) -> Boolean;
        }

        unsafe {
            // Create options dictionary to prompt if needed
            let key = CFString::from_static_string("AXTrustedCheckOptionPrompt");
            let value = core_foundation::boolean::CFBoolean::true_value();

            let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);

            AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) != 0
        }
    }

    fn run_event_tap(monitoring: Arc<Mutex<bool>>) -> Result<()> {
        tracing::debug!("Creating event tap...");

        // Create event tap with proper error handling
        let tap_result = CGEventTap::new(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            vec![CGEventType::KeyDown],
            |_proxy, event_type, event| {
                // Only process KeyDown events
                match event_type {
                    CGEventType::KeyDown => {
                        // Check if this matches any registered hotkey
                        if let Some(notecard_id) = Self::check_hotkey(&event) {
                            tracing::debug!("Hotkey matched for notecard {}", notecard_id.value());

                            // Call the callback
                            if let Ok(state) = HOTKEY_STATE.lock() {
                                if let Some(ref cb) = state.callback {
                                    cb(notecard_id);
                                }
                            }

                            // Consume the event (prevent it from propagating)
                            return None;
                        }
                    }
                    _ => {}
                }

                // Let the event pass through
                Some(event.clone())
            },
        );

        let tap = match tap_result {
            Ok(tap) => tap,
            Err(e) => {
                tracing::error!("Failed to create event tap: {:?}", e);
                return Err(anyhow!("Failed to create event tap. Make sure accessibility permissions are granted."));
            }
        };

        // Enable the tap
        tap.enable();

        // Update monitoring status
        {
            let mut mon = monitoring.lock().unwrap();
            *mon = true;
        }

        tracing::info!("Event tap created and enabled, starting run loop");

        // Run the current thread's run loop
        let tap_source = tap.mach_port.create_runloop_source(0)
            .map_err(|_| anyhow!("Failed to create run loop source"))?;

        let run_loop = CFRunLoop::get_current();
        unsafe {
            run_loop.add_source(&tap_source, kCFRunLoopCommonModes);
        }

        // Run the loop
        CFRunLoop::run_current();

        // Update monitoring status when done
        {
            let mut mon = monitoring.lock().unwrap();
            *mon = false;
        }

        tracing::info!("Event tap run loop ended");

        Ok(())
    }

    fn check_hotkey(event: &CGEvent) -> Option<NotecardId> {
        let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
        let flags = event.get_flags();

        // Map keycodes 18-26 to numbers 1-9
        let number = match keycode {
            18 => 1, // 1
            19 => 2, // 2
            20 => 3, // 3
            21 => 4, // 4
            23 => 5, // 5
            22 => 6, // 6
            26 => 7, // 7
            28 => 8, // 8
            25 => 9, // 9
            _ => return None,
        };

        // Try to create notecard ID
        let notecard_id = match NotecardId::new(number) {
            Ok(id) => id,
            Err(_) => return None,
        };

        // Check if this notecard has registered hotkeys
        let state = match HOTKEY_STATE.lock() {
            Ok(state) => state,
            Err(_) => return None,
        };

        if let Some(required_modifiers) = state.hotkeys.get(&notecard_id) {
            // Check if all required modifiers are pressed
            if Self::check_modifiers(&flags, required_modifiers) {
                tracing::debug!("Hotkey match found for notecard {}", notecard_id.value());
                return Some(notecard_id);
            }
        }

        None
    }

    fn check_modifiers(flags: &CGEventFlags, required: &[HotkeyModifier]) -> bool {
        for modifier in required {
            let pressed = match modifier {
                HotkeyModifier::Control => flags.contains(CGEventFlags::CGEventFlagControl),
                HotkeyModifier::Alt => flags.contains(CGEventFlags::CGEventFlagAlternate),
                HotkeyModifier::Shift => flags.contains(CGEventFlags::CGEventFlagShift),
                #[cfg(target_os = "macos")]
                HotkeyModifier::Command => flags.contains(CGEventFlags::CGEventFlagCommand),
            };

            if !pressed {
                return false;
            }
        }

        true
    }

    pub fn stop_monitoring(&mut self) {
        let mut monitoring = self.monitoring.lock().unwrap();
        *monitoring = false;

        // Stop the run loop
        CFRunLoop::get_current().stop();

        // Wait for thread to finish
        if let Some(handle) = self.event_tap_thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        self.stop_monitoring();
    }
}