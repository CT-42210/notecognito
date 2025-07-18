use anyhow::{anyhow, Result};
use core_foundation::base::TCFType;
use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes};
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventType, EventField,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use notecognito_core::{HotkeyModifier, NotecardId};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::thread;

pub struct HotkeyManager {
    registered_hotkeys: HashMap<(CGEventFlags, u16), NotecardId>,
    event_tap: Option<CFEventTap>,
    callback: Option<Arc<dyn Fn(NotecardId) + Send + Sync>>,
    monitor_thread: Option<thread::JoinHandle<()>>,
}

impl HotkeyManager {
    pub fn new() -> Self {
        HotkeyManager {
            registered_hotkeys: HashMap::new(),
            event_tap: None,
            callback: None,
            monitor_thread: None,
        }
    }

    pub fn register_hotkey(
        &mut self,
        notecard_id: NotecardId,
        modifiers: &[HotkeyModifier],
    ) -> Result<()> {
        // Convert modifiers to CGEventFlags
        let mut flags = CGEventFlags::empty();

        for modifier in modifiers {
            flags |= match modifier {
                HotkeyModifier::Control => CGEventFlags::CGEventFlagControl,
                HotkeyModifier::Alt => CGEventFlags::CGEventFlagAlternate,
                HotkeyModifier::Shift => CGEventFlags::CGEventFlagShift,
                HotkeyModifier::Command => CGEventFlags::CGEventFlagCommand,
            };
        }

        // Key code for numbers 1-9 (0x12 - 0x1A)
        let keycode = 0x11 + notecard_id.value() as u16;

        // Store the hotkey
        self.registered_hotkeys.insert((flags, keycode), notecard_id);

        tracing::info!(
            "Registered hotkey for notecard {} (keycode: {}, flags: {:?})",
            notecard_id.value(),
            keycode,
            flags
        );

        Ok(())
    }

    pub fn unregister_hotkey(&mut self, notecard_id: NotecardId) -> Result<()> {
        self.registered_hotkeys.retain(|_, &mut id| id != notecard_id);
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

        // Create shared state for the event tap callback
        let hotkeys = Arc::new(StdMutex::new(self.registered_hotkeys.clone()));
        let callback = Arc::clone(self.callback.as_ref().unwrap());

        // Create event tap
        let event_tap = CGEventTap::new(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::DefaultTap,
            vec![CGEventType::KeyDown],
            move |_proxy, event_type, event| {
                if event_type != CGEventType::KeyDown {
                    return None;
                }

                let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u16;
                let flags = event.get_flags();

                // Remove non-modifier flags
                let modifier_flags = flags
                    & (CGEventFlags::CGEventFlagControl
                        | CGEventFlags::CGEventFlagAlternate
                        | CGEventFlags::CGEventFlagShift
                        | CGEventFlags::CGEventFlagCommand);

                // Check if this matches a registered hotkey
                let hotkeys = hotkeys.lock().unwrap();
                if let Some(&notecard_id) = hotkeys.get(&(modifier_flags, keycode)) {
                    callback(notecard_id);
                    // Consume the event
                    return None;
                }

                // Pass through the event
                Some(event)
            },
        )
        .map_err(|_| anyhow!("Failed to create event tap"))?;

        event_tap.enable();
        self.event_tap = Some(event_tap);

        // Start monitoring thread
        let event_tap = self.event_tap.as_ref().unwrap().clone();
        let handle = thread::spawn(move || {
            unsafe {
                let run_loop = CFRunLoop::get_current();
                let loop_source = event_tap
                    .mach_port
                    .create_runloop_source(0)
                    .expect("Failed to create run loop source");
                run_loop.add_source(&loop_source, kCFRunLoopCommonModes);
                CFRunLoop::run_current();
            }
        });

        self.monitor_thread = Some(handle);

        Ok(())
    }

    pub fn stop_monitoring(&mut self) {
        if let Some(event_tap) = &self.event_tap {
            event_tap.enable(false);
        }
        self.event_tap = None;

        if let Some(handle) = self.monitor_thread.take() {
            // Stop the run loop
            // Note: In production, you'd want a more graceful shutdown
            let _ = handle.join();
        }
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        self.stop_monitoring();
    }
}