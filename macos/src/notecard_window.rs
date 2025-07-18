use anyhow::Result;
use notecognito_core::{DisplayProperties, NotecardId};
use std::sync::Arc;
use tokio::sync::Mutex;

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

        // In a real implementation, we would close the window here
        Ok(())
    }

    fn create_window_on_main_thread(
        &self,
        notecard_id: NotecardId,
        content: &str,
        properties: &DisplayProperties,
    ) -> Result<()> {
        use dispatch::Queue;
        use objc2_app_kit::{
            NSBackingStoreType, NSColor, NSFont, NSTextField, NSWindow,
            NSWindowStyleMask,
        };
        use objc2_foundation::{CGFloat, CGPoint, CGRect, CGSize, MainThreadMarker, NSString};

        let content = content.to_string();
        let opacity = properties.opacity;
        let font_size = properties.font_size;

        // Dispatch to main queue
        Queue::main().async_barrier(move || {
            unsafe {
                // Try to get main thread marker
                let mtm = match MainThreadMarker::new() {
                    Some(m) => m,
                    None => {
                        tracing::error!("Not on main thread for window creation");
                        return;
                    }
                };

                // Create window frame
                let frame = CGRect::new(
                    CGPoint::new(200.0, 200.0),
                    CGSize::new(400.0, 200.0),
                );

                // Create window
                let window = NSWindow::initWithContentRect_styleMask_backing_defer(
                    mtm.alloc::<NSWindow>(),
                    frame,
                    NSWindowStyleMask::Borderless,
                    NSBackingStoreType::NSBackingStoreBuffered,
                    false,
                );

                // Configure window
                window.setLevel(NSWindowLevel::Floating);
                window.setOpaque(false);
                window.setBackgroundColor(Some(&NSColor::clearColor()));
                window.setAlphaValue(opacity as CGFloat / 100.0);
                window.setHasShadow(true);

                // Create background view
                let content_view = window.contentView().unwrap();
                let bg_color = NSColor::colorWithWhite_alpha(0.1, 0.9);
                content_view.setWantsLayer(true);

                if let Some(layer) = content_view.layer() {
                    use objc2::msg_send;
                    let _: () = msg_send![&layer, setBackgroundColor: bg_color.CGColor()];
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
                if let Some(font) = NSFont::systemFontOfSize(font_size as CGFloat) {
                    text_field.setFont(Some(&font));
                }

                // Set frame for text field with padding
                let text_frame = CGRect::new(
                    CGPoint::new(20.0, 20.0),
                    CGSize::new(360.0, 160.0),
                );
                text_field.setFrame(text_frame);

                // Add text field to window
                content_view.addSubview(&text_field);

                // Show window
                window.makeKeyAndOrderFront(None);

                // Auto-hide after a delay if configured
                if properties.auto_hide_duration > 0 {
                    let duration = properties.auto_hide_duration as f64;
                    Queue::main().after(duration, move || {
                        window.close();
                    });
                }

                tracing::info!("Created window for notecard {}", notecard_id.value());
            }
        });

        Ok(())
    }
}