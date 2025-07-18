use anyhow::Result;
use notecognito_core::{DisplayProperties, NotecardId};
use objc2::rc::Retained;
use objc2::{msg_send, msg_send_id};
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSFont, NSTextField,
    NSView, NSWindow, NSWindowLevel, NSWindowStyleMask,
    NSWindowCollectionBehavior,
};
use objc2_foundation::{
    CGFloat, CGPoint, CGRect, CGSize, MainThreadMarker, NSString, NSTimer,
};
use std::collections::HashMap;
use std::ptr::NonNull;

// Thread-safe wrapper for NotecardWindow
pub struct NotecardWindow {
    // We'll store just the identifier and recreate windows as needed
    notecard_id: NotecardId,
    is_visible: bool,
}

// Make NotecardWindow Send + Sync by removing the NSWindow fields
unsafe impl Send for NotecardWindow {}
unsafe impl Sync for NotecardWindow {}

pub struct NotecardWindowManager {
    windows: HashMap<NotecardId, NotecardWindow>,
}

// Make NotecardWindowManager Send + Sync
unsafe impl Send for NotecardWindowManager {}
unsafe impl Sync for NotecardWindowManager {}

impl NotecardWindowManager {
    pub fn new() -> Self {
        NotecardWindowManager {
            windows: HashMap::new(),
        }
    }

    pub fn show_notecard(
        &mut self,
        notecard_id: NotecardId,
        content: &str,
        properties: &DisplayProperties,
    ) -> Result<()> {
        // Hide existing window if any
        self.hide_notecard(notecard_id)?;

        // Create window on main thread
        let mtm = MainThreadMarker::new()
            .ok_or_else(|| anyhow::anyhow!("Not on main thread"))?;

        // Create the window
        let window = self.create_notecard_window(mtm, notecard_id, content, properties)?;

        // Store window
        self.windows.insert(notecard_id, window);

        Ok(())
    }

    pub fn hide_notecard(&mut self, notecard_id: NotecardId) -> Result<()> {
        if let Some(notecard) = self.windows.remove(&notecard_id) {
            // For now, just remove from our tracking
            // Actual window cleanup would be handled on the main thread
        }
        Ok(())
    }

    fn create_notecard_window(
        &self,
        mtm: MainThreadMarker,
        notecard_id: NotecardId,
        content: &str,
        properties: &DisplayProperties,
    ) -> Result<NotecardWindow> {
        // For now, just create a simple representation
        // The actual window creation will be handled differently due to thread safety
        Ok(NotecardWindow {
            notecard_id,
            is_visible: true,
        })
    }
}

// Temporarily comment out the delegate implementation to fix thread safety issues
/*
use objc2::runtime::ProtocolObject;
use objc2::{declare_class, mutability, DeclaredClass};
use objc2_app_kit::{NSWindowDelegate, NSResponder};

declare_class!(
    struct NotecardWindowDelegate {
        window: Retained<NSWindow>,
    }

    unsafe impl ClassType for NotecardWindowDelegate {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
        const NAME: &'static str = "NotecardWindowDelegate";
    }

    impl DeclaredClass for NotecardWindowDelegate {
        type Ivars = StdMutex<Option<Retained<NSWindow>>>;
    }

    unsafe impl NSObjectProtocol for NotecardWindowDelegate {}

    unsafe impl NSWindowDelegate for NotecardWindowDelegate {}

    unsafe impl NotecardWindowDelegate {
        #[method(mouseDown:)]
        fn mouse_down(&self, _event: &NSEvent) {
            if let Some(window) = self.ivars().lock().unwrap().as_ref() {
                window.close();
            }
        }

        #[method(keyDown:)]
        fn key_down(&self, event: &NSEvent) {
            // Check for Escape key (keyCode 53)
            if event.keyCode() == 53 {
                if let Some(window) = self.ivars().lock().unwrap().as_ref() {
                    window.close();
                }
            }
        }
    }
);

impl NotecardWindowDelegate {
    fn new(mtm: MainThreadMarker, window: Retained<NSWindow>) -> Retained<Self> {
        let this = unsafe { msg_send_id![mtm.alloc::<Self>(), init] };
        *this.ivars().lock().unwrap() = Some(window);
        this
    }
}
*/
