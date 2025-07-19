use anyhow::Result;
use notecognito_core::{DisplayProperties, NotecardId};
use std::sync::Arc;
use tokio::sync::Mutex;
use objc2::msg_send; // Add this import for msg_send macro
use dispatch::Queue;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex as StdMutex;
use objc2_app_kit::NSWindow;
use objc2::rc::Retained;

// Global window storage
static ACTIVE_WINDOWS: Lazy<StdMutex<HashMap<u8, Retained<NSWindow>>>> = Lazy::new(|| {
    StdMutex::new(HashMap::new())
});

// Simple window info structure
#[derive(Clone)]
pub struct NotecardWindowInfo {
    notecard_id: NotecardId,
    content: String,
    properties: DisplayProperties,
}

pub struct NotecardWindowManager {
    // Store window info for display on the main thread
    pending_windows: Arc<Mutex<Vec<NotecardWindowInfo>>>,
}

unsafe impl Send for NotecardWindowManager {}
unsafe impl Sync for NotecardWindowManager {}

impl NotecardWindowManager {
    pub fn new() -> Self {
        NotecardWindowManager {
            pending_windows: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn show_notecard(
        &mut self,
        notecard_id: NotecardId,
        content: &str,
        properties: &DisplayProperties,
    ) -> Result<()> {
        // For now, just store the window info
        // The actual window creation needs to happen on the main thread
        let window_info = NotecardWindowInfo {
            notecard_id,
            content: content.to_string(),
            properties: properties.clone(),
        };

        // Add to pending windows
        let mut pending = self.pending_windows.lock().await;
        pending.push(window_info);

        // In a real implementation, we would signal the main thread to create the window
        // For now, let's use dispatch to create a basic window
        self.create_window_on_main_thread(notecard_id, content, properties)?;

        Ok(())
    }

    pub async fn hide_notecard(&mut self, notecard_id: NotecardId) -> Result<()> {
        // Remove from pending if exists
        let mut pending = self.pending_windows.lock().await;
        pending.retain(|w| w.notecard_id != notecard_id);

        // Close the window on the main thread
        let notecard_id_value = notecard_id.value();
        Queue::main().exec_async(move || {
            let mut windows = ACTIVE_WINDOWS.lock().unwrap();
            if let Some(window) = windows.remove(&notecard_id_value) {
                window.close();
                tracing::info!("Notecard {} window closed", notecard_id_value);
            }
        });

        Ok(())
    }

    fn create_window_on_main_thread(
        &self,
        notecard_id: NotecardId,
        content: &str,
        properties: &DisplayProperties,
    ) -> Result<()> {
        use objc2_app_kit::{
            NSBackingStoreType, NSColor, NSFont, NSTextField, NSWindow,
            NSWindowStyleMask,
        };
        use objc2_foundation::{CGFloat, CGPoint, CGRect, CGSize, MainThreadMarker, NSString};

        let content = content.to_string();
        let opacity = properties.opacity;
        let font_size = properties.font_size;
        let position = properties.position;
        let size = properties.size;

        // Dispatch to main queue
        Queue::main().exec_async(move || {
            unsafe {
                // Try to get main thread marker
                let mtm = match MainThreadMarker::new() {
                    Some(m) => m,
                    None => {
                        tracing::error!("Not on main thread for window creation");
                        return;
                    }
                };

                // Create window frame with user-specified position and size
                let frame = CGRect::new(
                    CGPoint::new(position.0 as CGFloat, position.1 as CGFloat),
                    CGSize::new(size.0 as CGFloat, size.1 as CGFloat),
                );

                // Create window
                let window = NSWindow::initWithContentRect_styleMask_backing_defer(
                    mtm.alloc::<NSWindow>(),
                    frame,
                    NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
                    NSBackingStoreType::NSBackingStoreBuffered,
                    false,
                );

                // Configure window
                // Set floating window level
                let _: () = msg_send![&window, setLevel: 3i64]; // NSFloatingWindowLevel
                window.setOpaque(false);
                window.setBackgroundColor(Some(&NSColor::clearColor()));
                window.setAlphaValue(opacity as CGFloat / 100.0);
                window.setHasShadow(true);
                window.setIgnoresMouseEvents(false); // Allow clicks to dismiss

                // Create background view
                let content_view = window.contentView().unwrap();
                let bg_color = NSColor::colorWithWhite_alpha(0.1, 0.9);
                content_view.setWantsLayer(true);

                if let Some(layer) = content_view.layer() {
                    let cg_color: *const std::ffi::c_void = msg_send![&bg_color, CGColor];
                    let _: () = msg_send![&layer, setBackgroundColor: cg_color];
                    let _: () = msg_send![&layer, setCornerRadius: 10.0f64];
                }

                // Create text field
                let text_field = NSTextField::new(mtm);
                text_field.setStringValue(&NSString::from_str(&content));
                text_field.setEditable(false);
                text_field.setBordered(false);
                text_field.setDrawsBackground(false);
                text_field.setTextColor(Some(&NSColor::whiteColor()));

                // Set font
                let font = NSFont::systemFontOfSize(font_size as CGFloat);
                text_field.setFont(Some(&font));

                // Set frame for text field with padding
                let text_frame = CGRect::new(
                    CGPoint::new(20.0, 20.0),
                    CGSize::new(size.0 as CGFloat - 40.0, size.1 as CGFloat - 40.0),
                );
                text_field.setFrame(text_frame);

                // Add text field to window
                content_view.addSubview(&text_field);

                // Store window for later access
                {
                    let mut windows = ACTIVE_WINDOWS.lock().unwrap();
                    windows.insert(notecard_id.value(), window.clone());
                }

                // Show window
                window.makeKeyAndOrderFront(None);

                tracing::info!("Notecard {} window displayed", notecard_id.value());
            }
        });

        Ok(())
    }
}