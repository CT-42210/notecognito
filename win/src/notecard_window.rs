use anyhow::Result;
use notecognito_core::{DisplayProperties, NotecardId};
use std::collections::HashMap;
use std::ffi::c_void;
use std::mem;
use windows::Win32::{
    Foundation::*,
    Graphics::Dwm::*,
    Graphics::Gdi::*,
    System::LibraryLoader::*,
    UI::WindowsAndMessaging::*,
};

const NOTECARD_CLASS_NAME: &str = "NotecognitoNotecard";
const WM_NOTECARD_CLOSE: u32 = WM_USER + 100;

pub struct NotecardWindow {
    hwnd: HWND,
    notecard_id: NotecardId,
}

pub struct NotecardWindowManager {
    windows: HashMap<NotecardId, NotecardWindow>,
    class_registered: bool,
}

impl NotecardWindowManager {
    pub fn new() -> Self {
        NotecardWindowManager {
            windows: HashMap::new(),
            class_registered: false,
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

        // Register window class if needed
        if !self.class_registered {
            self.register_window_class()?;
        }

        // Create window
        let hwnd = self.create_notecard_window(notecard_id, content, properties)?;

        // Store window handle
        self.windows.insert(notecard_id, NotecardWindow { hwnd, notecard_id });

        // Show window
        unsafe {
            ShowWindow(hwnd, SW_SHOWNA);
            UpdateWindow(hwnd)?;
        }

        // Set auto-hide timer if configured
        if properties.auto_hide_duration > 0 {
            unsafe {
                SetTimer(
                    hwnd,
                    1,
                    properties.auto_hide_duration * 1000,
                    None,
                )?;
            }
        }

        Ok(())
    }

    pub fn hide_notecard(&mut self, notecard_id: NotecardId) -> Result<()> {
        if let Some(window) = self.windows.remove(&notecard_id) {
            unsafe {
                DestroyWindow(window.hwnd)?;
            }
        }
        Ok(())
    }

    fn register_window_class(&mut self) -> Result<()> {
        unsafe {
            let instance = GetModuleHandleW(None)?;

            let wc = WNDCLASSEXW {
                cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(notecard_window_proc),
                cbClsExtra: 0,
                cbWndExtra: mem::size_of::<*mut NotecardWindowData>() as i32,
                hInstance: instance,
                hIcon: HICON::default(),
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                hbrBackground: HBRUSH::default(),
                lpszMenuName: PCWSTR::null(),
                lpszClassName: w!(NOTECARD_CLASS_NAME),
                hIconSm: HICON::default(),
            };

            if RegisterClassExW(&wc) == 0 {
                return Err(anyhow::anyhow!("Failed to register window class"));
            }
        }

        self.class_registered = true;
        Ok(())
    }

    fn create_notecard_window(
        &self,
        notecard_id: NotecardId,
        content: &str,
        properties: &DisplayProperties,
    ) -> Result<HWND> {
        unsafe {
            let instance = GetModuleHandleW(None)?;

            // Create window data
            let window_data = Box::new(NotecardWindowData {
                notecard_id,
                content: content.to_string(),
                properties: properties.clone(),
                font: HFONT::default(),
            });

            // Create the window
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                w!(NOTECARD_CLASS_NAME),
                w!("Notecognito"),
                WS_POPUP,
                properties.position.0,
                properties.position.1,
                properties.size.0 as i32,
                properties.size.1 as i32,
                None,
                None,
                instance,
                Some(Box::into_raw(window_data) as *const c_void),
            )?;

            if hwnd.0 == 0 {
                return Err(anyhow::anyhow!("Failed to create window"));
            }

            // Set window transparency
            let alpha = ((properties.opacity as u32 * 255) / 100) as u8;
            SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA)?;

            // Enable blur behind for Windows 10/11
            let _ = enable_blur_behind(hwnd);

            Ok(hwnd)
        }
    }
}

struct NotecardWindowData {
    notecard_id: NotecardId,
    content: String,
    properties: DisplayProperties,
    font: HFONT,
}

unsafe extern "system" fn notecard_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let create_struct = lparam.0 as *const CREATESTRUCTW;
            let window_data = (*create_struct).lpCreateParams as *mut NotecardWindowData;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, window_data as isize);

            // Create font
            if let Some(data) = window_data.as_mut() {
                let font_name = match data.properties.font_family.as_str() {
                    "System" => "Segoe UI",
                    name => name,
                };

                data.font = CreateFontW(
                    -(data.properties.font_size as i32),
                    0, 0, 0,
                    FW_NORMAL.0 as i32,
                    false.into(),
                    false.into(),
                    false.into(),
                    DEFAULT_CHARSET.0 as u32,
                    OUT_DEFAULT_PRECIS.0 as u32,
                    CLIP_DEFAULT_PRECIS.0 as u32,
                    CLEARTYPE_QUALITY.0 as u32,
                    DEFAULT_PITCH.0 as u32 | FF_DONTCARE.0 as u32,
                    &HSTRING::from(font_name),
                );
            }

            LRESULT(0)
        }

        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            if let Some(window_data) = get_window_data(hwnd) {
                // Set up drawing
                SetBkMode(hdc, TRANSPARENT);
                SetTextColor(hdc, COLORREF(0xFFFFFF)); // White text
                SelectObject(hdc, window_data.font);

                // Get client rect
                let mut rect = RECT::default();
                GetClientRect(hwnd, &mut rect)?;

                // Draw dark background
                let brush = CreateSolidBrush(COLORREF(0x202020));
                FillRect(hdc, &rect, brush);
                DeleteObject(brush);

                // Add padding
                rect.left += 10;
                rect.top += 10;
                rect.right -= 10;
                rect.bottom -= 10;

                // Draw text
                let text = HSTRING::from(&window_data.content);
                DrawTextW(
                    hdc,
                    &text,
                    &mut rect,
                    DT_LEFT | DT_TOP | DT_WORDBREAK | DT_EXPANDTABS,
                );
            }

            EndPaint(hwnd, &ps);
            LRESULT(0)
        }

        WM_TIMER => {
            // Auto-hide timer fired
            PostMessageW(hwnd, WM_NOTECARD_CLOSE, WPARAM(0), LPARAM(0))?;
            LRESULT(0)
        }

        WM_LBUTTONDOWN => {
            // Close on click
            PostMessageW(hwnd, WM_NOTECARD_CLOSE, WPARAM(0), LPARAM(0))?;
            LRESULT(0)
        }

        WM_KEYDOWN => {
            if wparam.0 == VK_ESCAPE.0 as usize {
                PostMessageW(hwnd, WM_NOTECARD_CLOSE, WPARAM(0), LPARAM(0))?;
            }
            LRESULT(0)
        }

        WM_NOTECARD_CLOSE => {
            DestroyWindow(hwnd)?;
            LRESULT(0)
        }

        WM_DESTROY => {
            // Clean up window data
            if let Some(window_data) = get_window_data_mut(hwnd) {
                if window_data.font.0 != 0 {
                    DeleteObject(window_data.font);
                }
                // Free the window data
                let _ = Box::from_raw(window_data);
            }
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn get_window_data(hwnd: HWND) -> Option<&'static NotecardWindowData> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const NotecardWindowData;
    ptr.as_ref()
}

unsafe fn get_window_data_mut(hwnd: HWND) -> Option<&'static mut NotecardWindowData> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut NotecardWindowData;
    ptr.as_mut()
}

fn enable_blur_behind(hwnd: HWND) -> Result<()> {
    unsafe {
        let policy = DWM_WINDOW_CORNER_PREFERENCE::DWMWCP_ROUND;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &policy as *const _ as *const c_void,
            mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        )?;

        let backdrop_type = DWM_SYSTEMBACKDROP_TYPE::DWMSBT_TRANSIENTWINDOW;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop_type as *const _ as *const c_void,
            mem::size_of::<DWM_SYSTEMBACKDROP_TYPE>() as u32,
        );
    }
    Ok(())
}