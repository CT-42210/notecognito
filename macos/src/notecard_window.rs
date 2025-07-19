use anyhow::Result;
use notecognito_core::{DisplayProperties, NotecardId};
use std::sync::Arc;
use tokio::sync::Mutex;
use objc2::msg_send;
use dispatch::Queue;
use std::collections::HashMap;
use std::sync::Mutex as StdMutex;

// Store only window IDs that can be used to find windows later
static ACTIVE_WINDOW_IDS: once_cell::sync::Lazy<StdMutex<HashMap<u8, i64>>> =
    once_cell::sync::Lazy::new(|| StdMutex::new(HashMap::new()));

// Simple window info structure
#[derive(Clone)]
pub struct NotecardWindowInfo {
    notecard_id: NotecardId,
    content: String,
    properties: DisplayProperties,
}

pub struct NotecardWindowManager {
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
        let window_info = NotecardWindowInfo {
            notecard_id,
            content: content.to_string(),
            properties: properties.clone(),
        };

        let mut pending = self.pending_windows.lock().await;
        pending.push(window_info);

        self.create_window_on_main_thread(notecard_id, content, properties)?;
        Ok(())
    }

    pub async fn hide_notecard(&mut self, notecard_id: NotecardId) -> Result<()> {
        let mut pending = self.pending_windows.lock().await;
        pending.retain(|w| w.notecard_id != notecard_id);

        let notecard_id_value = notecard_id.value();
        Queue::main().exec_async(move || {
            let mut window_ids = ACTIVE_WINDOW_IDS.lock().unwrap();
            if let Some(window_number) = window_ids.remove(&notecard_id_value) {
                // Find and close the window using NSApp
                unsafe {
                    use objc2_app_kit::NSApplication;
                    use objc2_foundation::MainThreadMarker;

                    if let Some(mtm) = MainThreadMarker::new() {
                        let app = NSApplication::sharedApplication(mtm);
                        let windows = app.windows();

                        for i in 0..windows.count() {
                            let window = windows.objectAtIndex(i);
                            let window_num: i64 = msg_send![&window, windowNumber];
                            if window_num == window_number {
                                let _: () = msg_send![&window, close];
                                break;
                            }
                        }
                    }
                }
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
        let notecard_id_value = notecard_id.value();

        Queue::main().exec_async(move || {
            unsafe {
                let mtm = match MainThreadMarker::new() {
                    Some(m) => m,
                    None => {
                        tracing::error!("Not on main thread for window creation");
                        return;
                    }
                };

                let frame = CGRect::new(
                    CGPoint::new(position.0 as CGFloat, position.1 as CGFloat),
                    CGSize::new(size.0 as CGFloat, size.1 as CGFloat),
                );

                let window = NSWindow::initWithContentRect_styleMask_backing_defer(
                    mtm.alloc::<NSWindow>(),
                    frame,
                    NSWindowStyleMask::Borderless, // Remove NonactivatingPanel
                    NSBackingStoreType::NSBackingStoreBuffered,
                    false,
                );

                let _: () = msg_send![&window, setLevel: 3i64];
                window.setOpaque(false);
                window.setBackgroundColor(Some(&NSColor::clearColor()));
                window.setAlphaValue(opacity as CGFloat / 100.0);
                window.setHasShadow(true);
                window.setIgnoresMouseEvents(false);

                let content_view = window.contentView().unwrap();
                let bg_color = NSColor::colorWithWhite_alpha(0.1, 0.9);
                content_view.setWantsLayer(true);

                if let Some(layer) = content_view.layer() {
                    let _: () = msg_send![&layer, setCornerRadius: 10.0f64];
                }
                // Set background color directly on the content view
                let _: () = msg_send![&content_view, setBackgroundColor: &*bg_color];

                let text_field = NSTextField::new(mtm);
                text_field.setStringValue(&NSString::from_str(&content));
                text_field.setEditable(false);
                text_field.setBordered(false);
                text_field.setDrawsBackground(false);
                text_field.setTextColor(Some(&NSColor::whiteColor()));

                let font = NSFont::systemFontOfSize(font_size as CGFloat);
                text_field.setFont(Some(&font));

                let text_frame = CGRect::new(
                    CGPoint::new(20.0, 20.0),
                    CGSize::new(size.0 as CGFloat - 40.0, size.1 as CGFloat - 40.0),
                );
                text_field.setFrame(text_frame);

                content_view.addSubview(&text_field);

                // Store window number instead of window object
                let window_number: i64 = msg_send![&window, windowNumber];
                {
                    let mut window_ids = ACTIVE_WINDOW_IDS.lock().unwrap();
                    window_ids.insert(notecard_id_value, window_number);
                }

                window.makeKeyAndOrderFront(None);

                tracing::info!("Notecard {} window displayed", notecard_id_value);
            }
        });

        Ok(())
    }
}