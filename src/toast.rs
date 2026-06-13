use crate::win::*;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr::null_mut;
use std::sync::{Mutex, OnceLock};
use std::thread;

const TOAST_WIDTH: i32 = 410;
const TOAST_HEIGHT: i32 = 86;
const TOAST_MARGIN: i32 = 18;
const TOAST_DURATION_MS: u32 = 2600;

static CLASS_REGISTRATION: OnceLock<Result<(), String>> = OnceLock::new();
static MESSAGES: OnceLock<Mutex<HashMap<usize, String>>> = OnceLock::new();

fn messages() -> &'static Mutex<HashMap<usize, String>> {
    MESSAGES.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn initialize() -> Result<(), String> {
    CLASS_REGISTRATION
        .get_or_init(|| {
            let class_name = wide_null("ParkerToastWindow");
            let instance = unsafe { GetModuleHandleW(null_mut()) };
            let class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(toast_window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: null_mut(),
                hCursor: unsafe { LoadCursorW(null_mut(), IDC_ARROW as *const u16) },
                hbrBackground: null_mut(),
                lpszMenuName: null_mut(),
                lpszClassName: class_name.as_ptr(),
            };

            if unsafe { RegisterClassW(&class) } == 0 {
                Err("Could not register the Parker toast window class.".to_string())
            } else {
                Ok(())
            }
        })
        .clone()
}

pub fn show(message: impl Into<String>) {
    if initialize().is_err() {
        return;
    }

    let message = message.into();
    let _ = thread::spawn(move || unsafe {
        let mut work_area = RECT::default();
        if SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            &mut work_area as *mut RECT as *mut c_void,
            0,
        ) == 0
        {
            work_area = RECT {
                left: 0,
                top: 0,
                right: GetSystemMetrics(SM_CXSCREEN),
                bottom: GetSystemMetrics(SM_CYSCREEN),
            };
        }

        let x = work_area.right - TOAST_WIDTH - TOAST_MARGIN;
        let y = work_area.bottom - TOAST_HEIGHT - TOAST_MARGIN;
        let class_name = wide_null("ParkerToastWindow");
        let title = wide_null("Parker");
        let instance = GetModuleHandleW(null_mut());
        let window = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP,
            x,
            y,
            TOAST_WIDTH,
            TOAST_HEIGHT,
            null_mut(),
            null_mut(),
            instance,
            null_mut(),
        );

        if window.is_null() {
            return;
        }

        SetWindowDisplayAffinity(window, WDA_EXCLUDEFROMCAPTURE);

        if let Ok(mut map) = messages().lock() {
            map.insert(window as usize, message);
        }

        ShowWindow(window, SW_SHOWNOACTIVATE);
        UpdateWindow(window);
        SetTimer(window, 1, TOAST_DURATION_MS, None);

        let mut event = MSG::default();
        while GetMessageW(&mut event, null_mut(), 0, 0) > 0 {
            TranslateMessage(&event);
            DispatchMessageW(&event);
        }
    });
}

unsafe extern "system" fn toast_window_proc(
    window: HWND,
    message: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_PAINT => {
            paint_toast(window);
            0
        }
        WM_TIMER | WM_LBUTTONDOWN => {
            KillTimer(window, 1);
            DestroyWindow(window);
            0
        }
        WM_DESTROY => {
            if let Ok(mut map) = messages().lock() {
                map.remove(&(window as usize));
            }
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(window, message, wparam, lparam),
    }
}

unsafe fn paint_toast(window: HWND) {
    let mut paint = PAINTSTRUCT::default();
    let device = BeginPaint(window, &mut paint);
    if device.is_null() {
        return;
    }

    let mut client = RECT::default();
    GetClientRect(window, &mut client);
    let background = CreateSolidBrush(rgb(28, 29, 33));
    FillRect(device, &client, background);
    DeleteObject(background as HGDIOBJ);

    SetBkMode(device, TRANSPARENT);
    SetTextColor(device, rgb(245, 245, 247));

    let text = messages()
        .lock()
        .ok()
        .and_then(|map| map.get(&(window as usize)).cloned())
        .unwrap_or_else(|| "Parker".to_string());
    let text = wide_null(&text);
    let mut text_rect = RECT {
        left: 20,
        top: 10,
        right: client.right - 20,
        bottom: client.bottom - 10,
    };
    DrawTextW(
        device,
        text.as_ptr(),
        -1,
        &mut text_rect,
        DT_LEFT | DT_VCENTER | DT_WORDBREAK | DT_NOPREFIX,
    );

    EndPaint(window, &paint);
}
