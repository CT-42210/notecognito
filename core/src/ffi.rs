use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;
use crate::{ConfigManager, NotecardId, Notecard};

/// Result type for FFI functions
#[repr(C)]
pub struct FfiResult {
    success: bool,
    error_message: *mut c_char,
}

impl FfiResult {
    fn success() -> Self {
        FfiResult {
            success: true,
            error_message: ptr::null_mut(),
        }
    }

    fn error(msg: &str) -> Self {
        let error_message = CString::new(msg).unwrap_or_else(|_| CString::new("Unknown error").unwrap());
        FfiResult {
            success: false,
            error_message: error_message.into_raw(),
        }
    }
}

/// Frees a string allocated by Rust
#[no_mangle]
pub extern "C" fn notecognito_free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(s);
    }
}

/// Creates a new configuration manager
#[no_mangle]
pub extern "C" fn notecognito_config_manager_new() -> *mut ConfigManager {
    match ConfigManager::new() {
        Ok(manager) => Box::into_raw(Box::new(manager)),
        Err(_) => ptr::null_mut(),
    }
}

/// Frees a configuration manager
#[no_mangle]
pub extern "C" fn notecognito_config_manager_free(manager: *mut ConfigManager) {
    if manager.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(manager);
    }
}

/// Updates a notecard
#[no_mangle]
pub extern "C" fn notecognito_update_notecard(
    manager: *mut ConfigManager,
    id: c_int,
    content: *const c_char,
) -> FfiResult {
    if manager.is_null() || content.is_null() {
        return FfiResult::error("Invalid parameters");
    }

    let manager = unsafe { &mut *manager };

    let content_str = unsafe {
        match CStr::from_ptr(content).to_str() {
            Ok(s) => s,
            Err(_) => return FfiResult::error("Invalid UTF-8 in content"),
        }
    };

    let notecard_id = match NotecardId::new(id as u8) {
        Ok(id) => id,
        Err(_) => return FfiResult::error("Invalid notecard ID (must be 1-9)"),
    };

    let notecard = Notecard::new(notecard_id, content_str.to_string());

    match manager.update_notecard(notecard) {
        Ok(_) => match manager.save() {
            Ok(_) => FfiResult::success(),
            Err(e) => FfiResult::error(&e.to_string()),
        },
        Err(e) => FfiResult::error(&e.to_string()),
    }
}

/// Gets notecard content
#[no_mangle]
pub extern "C" fn notecognito_get_notecard_content(
    manager: *mut ConfigManager,
    id: c_int,
) -> *mut c_char {
    if manager.is_null() {
        return ptr::null_mut();
    }

    let manager = unsafe { &*manager };

    let notecard_id = match NotecardId::new(id as u8) {
        Ok(id) => id,
        Err(_) => return ptr::null_mut(),
    };

    match manager.get_notecard(notecard_id) {
        Some(notecard) => {
            match CString::new(notecard.content.clone()) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => ptr::null_mut(),
            }
        }
        None => ptr::null_mut(),
    }
}

/// Gets the configuration as JSON
#[no_mangle]
pub extern "C" fn notecognito_get_config_json(manager: *mut ConfigManager) -> *mut c_char {
    if manager.is_null() {
        return ptr::null_mut();
    }

    let manager = unsafe { &*manager };

    match serde_json::to_string(manager.config()) {
        Ok(json) => match CString::new(json) {
            Ok(c_str) => c_str.into_raw(),
            Err(_) => ptr::null_mut(),
        },
        Err(_) => ptr::null_mut(),
    }
}

/// Sets the launch on startup flag
#[no_mangle]
pub extern "C" fn notecognito_set_launch_on_startup(
    manager: *mut ConfigManager,
    enabled: bool,
) -> FfiResult {
    if manager.is_null() {
        return FfiResult::error("Invalid manager");
    }

    let manager = unsafe { &mut *manager };
    manager.config_mut().launch_on_startup = enabled;

    match manager.save() {
        Ok(_) => FfiResult::success(),
        Err(e) => FfiResult::error(&e.to_string()),
    }
}