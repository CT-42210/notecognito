use anyhow::Result;
use notecognito_core::{
    DisplayProperties, HotkeyModifier, NotecardId, PlatformInterface,
};
use objc2_app_kit::{NSAlert, NSAlertStyle, NSModalResponse};
use objc2_foundation::{MainThreadMarker, NSString};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::hotkey::HotkeyManager;
use crate::notecard_window::NotecardWindowManager;

pub struct MacOSPlatform {
    hotkey_manager: Arc<Mutex<HotkeyManager>>,
    window_manager: Arc<Mutex<NotecardWindowManager>>,
    initialized: bool,
}

impl MacOSPlatform {
    pub fn new(
        hotkey_manager: Arc<Mutex<HotkeyManager>>,
        window_manager: Arc<Mutex<NotecardWindowManager>>,
    ) -> Self {
        MacOSPlatform {
            hotkey_manager,
            window_manager,
            initialized: false,
        }
    }
}

impl PlatformInterface for MacOSPlatform {
    fn register_hotkey(
        &mut self,
        id: NotecardId,
        modifiers: &[HotkeyModifier],
    ) -> notecognito_core::Result<()> {
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
        use core_foundation::array::CFArray;
        use core_foundation::base::{Boolean, CFType, TCFType};
        use core_foundation::string::CFString;
        use core_foundation::url::CFURL;
        use std::ptr;

        unsafe {
            // Dynamic loading of LaunchServices framework
            #[link(name = "CoreServices", kind = "framework")]
            extern "C" {
                fn LSSharedFileListCreate(
                    allocator: core_foundation::base::CFAllocatorRef,
                    list_type: core_foundation::base::CFStringRef,
                    list_options: core_foundation::base::CFTypeRef,
                ) -> core_foundation::base::CFTypeRef;

                fn LSSharedFileListInsertItemURL(
                    list: core_foundation::base::CFTypeRef,
                    insert_after_item: core_foundation::base::CFTypeRef,
                    name: core_foundation::string::CFStringRef,
                    icon_ref: core_foundation::base::CFTypeRef,
                    url: core_foundation::url::CFURLRef,
                    properties: core_foundation::dictionary::CFDictionaryRef,
                    items_to_add: core_foundation::array::CFArrayRef,
                ) -> core_foundation::base::CFTypeRef;

                fn LSSharedFileListItemRemove(
                    list: core_foundation::base::CFTypeRef,
                    item: core_foundation::base::CFTypeRef,
                ) -> core_foundation::base::OSStatus;

                fn LSSharedFileListCopySnapshot(
                    list: core_foundation::base::CFTypeRef,
                    seed: *mut u32,
                ) -> core_foundation::array::CFArrayRef;

                fn LSSharedFileListItemCopyResolvedURL(
                    item: core_foundation::base::CFTypeRef,
                    flags: u32,
                    error: *mut core_foundation::base::CFErrorRef,
                ) -> core_foundation::url::CFURLRef;
            }

            // Constants
            let k_ls_shared_file_list_session_login_items =
                CFString::from_static_string("com.apple.LSSharedFileList.SessionLoginItems");
            let k_ls_shared_file_list_item_last =
                core_foundation::base::kCFNull;

            // Create login items list
            let list = LSSharedFileListCreate(
                ptr::null(),
                k_ls_shared_file_list_session_login_items.as_concrete_TypeRef(),
                ptr::null(),
            );

            if list.is_null() {
                return Err(notecognito_core::NotecognitoError::Platform(
                    "Failed to access login items".to_string(),
                ));
            }

            // Get app URL
            let app_path = std::env::current_exe()
                .map_err(|e| notecognito_core::NotecognitoError::Platform(e.to_string()))?;

            // For .app bundles, get the bundle path
            let bundle_path = if app_path.to_string_lossy().contains(".app/Contents/MacOS/") {
                app_path
                    .parent() // MacOS
                    .and_then(|p| p.parent()) // Contents
                    .and_then(|p| p.parent()) // .app
                    .unwrap_or(&app_path)
            } else {
                &app_path
            };

            let app_url = CFURL::from_path(bundle_path, false)
                .ok_or_else(|| notecognito_core::NotecognitoError::Platform(
                    "Failed to create app URL".to_string(),
                ))?;

            if enabled {
                // Add to login items
                LSSharedFileListInsertItemURL(
                    list,
                    k_ls_shared_file_list_item_last,
                    CFString::from_static_string("Notecognito").as_concrete_TypeRef(),
                    ptr::null(),
                    app_url.as_concrete_TypeRef(),
                    ptr::null(),
                    ptr::null(),
                );
            } else {
                // Remove from login items
                let mut seed: u32 = 0;
                let items = LSSharedFileListCopySnapshot(list, &mut seed);

                if !items.is_null() {
                    let items_array = CFArray::<CFType>::wrap_under_create_rule(items);

                    for i in 0..items_array.len() {
                        let item = items_array.get(i).unwrap();
                        let item_url = LSSharedFileListItemCopyResolvedURL(
                            item.as_CFTypeRef(),
                            0,
                            ptr::null_mut(),
                        );

                        if !item_url.is_null() {
                            let item_url = CFURL::wrap_under_create_rule(item_url);
                            if item_url.to_path().unwrap() == bundle_path {
                                LSSharedFileListItemRemove(list, item.as_CFTypeRef());
                            }
                        }
                    }
                }
            }

            Ok(())
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
        use core_graphics::access::ScreenCaptureAccess;

        // Check if we have accessibility permissions (needed for global hotkeys)
        let has_permissions = ScreenCaptureAccess::preflight();

        if !has_permissions {
            // Show alert on main thread
            if let Some(mtm) = MainThreadMarker::new() {
                unsafe {
                    let alert = NSAlert::new(mtm);
                    alert.setMessageText(&NSString::from_str("Accessibility Permission Required"));
                    alert.setInformativeText(&NSString::from_str(
                        "Notecognito needs accessibility permissions to register global hotkeys.\n\n\
                        Please grant permission in System Preferences > Security & Privacy > \
                        Privacy > Accessibility, then restart the app."
                    ));
                    alert.setAlertStyle(NSAlertStyle::Warning);
                    alert.runModal();
                }
            }
        }

        Ok(has_permissions)
    }

    fn request_permissions(&self) -> notecognito_core::Result<()> {
        use core_graphics::access::ScreenCaptureAccess;

        // This will prompt the user for permissions if not already granted
        ScreenCaptureAccess::request();

        Ok(())
    }
}