use anyhow::Result;
use notecognito_core::{DisplayProperties, NotecardId};
use objc2::rc::Retained;
use objc2::{msg_send, msg_send_id, ClassType};
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSEvent, NSEventType, NSFont, NSScreen, NSTextField,
    NSTextFieldDelegate, NSView, NSWindow, NSWindowLevel, NSWindowStyleMask,
};
use objc2_foundation::{
    ns_string, CGFloat, CGPoint, CGRect, CGSize, MainThreadMarker, NSNotification, NSObject,
    NSObjectProtocol, NSString, NSTimer,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;

pub struct NotecardWindow {
    window: Retained<NSWindow>,
    text_field: Retained<NSTextField>,
    notecard_id: NotecardId,
    timer: Option<Retained<NSTimer>>,
}

pub struct NotecardWindowManager {
    windows: HashMap<NotecardId, NotecardWindow>,
}

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
        if let Some(mut notecard) = self.windows.remove(&notecard_id) {
            unsafe {
                // Cancel timer if exists
                if let Some(timer) = notecard.timer.take() {
                    timer.invalidate();
                }

                // Close window
                notecard.window.close();
            }
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
        unsafe {
            // Create window rect
            let rect = CGRect::new(
                CGPoint::new(properties.position.0 as f64, properties.position.1 as f64),
                CGSize::new(properties.size.0 as f64, properties.size.1 as f64),
            );

            // Create window
            let window = NSWindow::initWithContentRect_styleMask_backing_defer(
                mtm.alloc::<NSWindow>(),
                rect,
                NSWindowStyleMask::Borderless
                    | NSWindowStyleMask::NonactivatingPanel,
                NSBackingStoreType::NSBackingStoreBuffered,
                false,
            );

            // Configure window
            window.setLevel(NSWindowLevel::FloatingWindow);
            window.setOpaque(false);
            window.setBackgroundColor(Some(&NSColor::clearColor()));
            window.setHasShadow(true);
            window.setIgnoresMouseEvents(false);
            window.setCollectionBehavior(
                NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                    | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary
                    | NSWindowCollectionBehavior::NSWindowCollectionBehaviorIgnoresCycle,
            );

            // Set window opacity
            let alpha = properties.opacity as f32 / 100.0;
            window.setAlphaValue(alpha as CGFloat);

            // Create content view with dark background
            let content_view = NSView::initWithFrame(mtm.alloc::<NSView>(), rect);

            // Set dark background
            let background_color = NSColor::colorWithWhite_alpha(0.15, 1.0);
            msg_send![&content_view, setWantsLayer: true];
            let layer: Retained<NSObject> = msg_send_id![&content_view, layer];
            msg_send![&layer, setBackgroundColor: msg_send_id![&background_color, CGColor]];
            msg_send![&layer, setCornerRadius: 8.0f64];

            // Create text field
            let text_rect = CGRect::new(
                CGPoint::new(10.0, 10.0),
                CGSize::new(rect.size.width - 20.0, rect.size.height - 20.0),
            );
            let text_field = NSTextField::initWithFrame(mtm.alloc::<NSTextField>(), text_rect);

            // Configure text field
            text_field.setStringValue(&NSString::from_str(content));
            text_field.setBezeled(false);
            text_field.setDrawsBackground(false);
            text_field.setEditable(false);
            text_field.setSelectable(false);

            // Set font
            let font_name = match properties.font_family.as_str() {
                "System" => "Helvetica Neue",
                "SF Pro" => "SF Pro Display",
                name => name,
            };

            if let Some(font) = NSFont::fontWithName_size(
                &NSString::from_str(font_name),
                properties.font_size as CGFloat,
            ) {
                text_field.setFont(Some(&font));
            }

            // Set text color (white)
            text_field.setTextColor(Some(&NSColor::whiteColor()));

            // Enable word wrap
            msg_send![&text_field, setLineBreakMode: 0]; // NSLineBreakByWordWrapping
            let cell: Retained<NSObject> = msg_send_id![&text_field, cell];
            msg_send![&cell, setWraps: true];

            // Add text field to content view
            content_view.addSubview(&text_field);

            // Set content view
            window.setContentView(Some(&content_view));

            // Make window visible
            window.makeKeyAndOrderFront(None);

            // Create auto-hide timer if needed
            let timer = if properties.auto_hide_duration > 0 {
                let window_weak = window.clone();
                let duration = properties.auto_hide_duration as f64;

                Some(NSTimer::scheduledTimerWithTimeInterval_repeats_block(
                    duration,
                    false,
                    &block2::ConcreteBlock::new(move |_timer: &NSTimer| {
                        window_weak.close();
                    }).copy(),
                ))
            } else {
                None
            };

            // Set up click handler
            let delegate = NotecardWindowDelegate::new(mtm, window.clone());
            window.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
            std::mem::forget(delegate); // Keep delegate alive

            Ok(NotecardWindow {
                window,
                text_field,
                notecard_id,
                timer,
            })
        }
    }
}

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