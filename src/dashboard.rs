use crate::win::*;
use std::ptr::null_mut;

pub const WM_DASHBOARD_SMART_CAPTURE: UINT = WM_APP + 20;
pub const WM_DASHBOARD_TOGGLE_RECORDING: UINT = WM_APP + 21;
pub const WM_DASHBOARD_OPEN_RECORDINGS: UINT = WM_APP + 22;
pub const WM_DASHBOARD_OPEN_SETTINGS: UINT = WM_APP + 23;

const CMD_SMART_CAPTURE: usize = 2001;
const CMD_RECORD: usize = 2002;
const CMD_OPEN_RECORDINGS: usize = 2003;
const CMD_SETTINGS: usize = 2004;
const STATUS_LABEL: INT = 2101;
const RECORD_BUTTON: INT = CMD_RECORD as INT;

pub fn create() -> Result<HWND, String> {
    let class_name = wide_null("ParkerMainWindow");
    let title = wide_null("Parker");
    let instance = unsafe { GetModuleHandleW(null_mut()) };
    let icon = load_icon(32, 32);
    let class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: icon,
        hCursor: unsafe { LoadCursorW(null_mut(), IDC_ARROW as *const u16) },
        hbrBackground: unsafe { GetStockObject(WHITE_BRUSH) as HBRUSH },
        lpszMenuName: null_mut(),
        lpszClassName: class_name.as_ptr(),
    };

    if unsafe { RegisterClassW(&class) } == 0 {
        return Err("Could not register Parker's window class.".to_string());
    }

    let width = 480;
    let height = 330;
    let x = (unsafe { GetSystemMetrics(SM_CXSCREEN) } - width) / 2;
    let y = (unsafe { GetSystemMetrics(SM_CYSCREEN) } - height) / 2;
    let window = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            x,
            y,
            width,
            height,
            null_mut(),
            null_mut(),
            instance,
            null_mut(),
        )
    };
    if window.is_null() {
        return Err("Could not create Parker's main window.".to_string());
    }

    unsafe {
        SendMessageW(window, WM_SETICON, ICON_BIG, load_icon(32, 32) as LPARAM);
        SendMessageW(window, WM_SETICON, ICON_SMALL, load_icon(16, 16) as LPARAM);
    }
    create_controls(window)?;
    Ok(window)
}

pub fn show(window: HWND) {
    unsafe {
        ShowWindow(window, SW_SHOWNORMAL);
        UpdateWindow(window);
        SetForegroundWindow(window);
    }
}

pub fn set_recording(window: HWND, recording: bool) {
    set_text(
        unsafe { GetDlgItem(window, RECORD_BUTTON) },
        if recording {
            "Stop recording"
        } else {
            "Record region"
        },
    );
    set_status(
        window,
        if recording {
            "Recording region. Stop here or press Ctrl+Shift+F9."
        } else {
            "Ready. Choose an action or use a keyboard shortcut."
        },
    );
}

pub fn set_processing(window: HWND) {
    set_text(
        unsafe { GetDlgItem(window, RECORD_BUTTON) },
        "Optimizing...",
    );
    set_status(
        window,
        "Optimizing recording. Parker will notify you when it is ready.",
    );
}

pub fn set_status(window: HWND, status: &str) {
    set_text(unsafe { GetDlgItem(window, STATUS_LABEL) }, status);
}

fn create_controls(window: HWND) -> Result<(), String> {
    create_control(window, "STATIC", "Parker", 28, 22, 420, 34, 0, SS_LEFT)?;
    create_control(
        window,
        "STATIC",
        "Capture text and QR codes, or record any screen region.",
        28,
        58,
        420,
        36,
        0,
        SS_LEFT,
    )?;
    create_control(
        window,
        "BUTTON",
        "Smart capture",
        28,
        112,
        198,
        48,
        CMD_SMART_CAPTURE,
        BS_PUSHBUTTON | WS_TABSTOP,
    )?;
    create_control(
        window,
        "BUTTON",
        "Record region",
        246,
        112,
        198,
        48,
        CMD_RECORD,
        BS_PUSHBUTTON | WS_TABSTOP,
    )?;
    create_control(
        window,
        "BUTTON",
        "Open recordings",
        28,
        176,
        198,
        42,
        CMD_OPEN_RECORDINGS,
        BS_PUSHBUTTON | WS_TABSTOP,
    )?;
    create_control(
        window,
        "BUTTON",
        "Settings",
        246,
        176,
        198,
        42,
        CMD_SETTINGS,
        BS_PUSHBUTTON | WS_TABSTOP,
    )?;
    create_control(
        window,
        "STATIC",
        "Ready. Choose an action or use a keyboard shortcut.",
        28,
        246,
        416,
        30,
        STATUS_LABEL as usize,
        SS_LEFT,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn create_control(
    parent: HWND,
    class: &str,
    text: &str,
    x: INT,
    y: INT,
    width: INT,
    height: INT,
    id: usize,
    control_style: DWORD,
) -> Result<HWND, String> {
    let class = wide_null(class);
    let text = wide_null(text);
    let control = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            text.as_ptr(),
            WS_CHILD | WS_VISIBLE | control_style,
            x,
            y,
            width,
            height,
            parent,
            id as HMENU,
            GetModuleHandleW(null_mut()),
            null_mut(),
        )
    };
    if control.is_null() {
        return Err("Could not create Parker's dashboard controls.".to_string());
    }
    unsafe {
        SendMessageW(
            control,
            WM_SETFONT,
            GetStockObject(DEFAULT_GUI_FONT) as WPARAM,
            TRUE as LPARAM,
        );
    }
    Ok(control)
}

fn set_text(window: HWND, text: &str) {
    if window.is_null() {
        return;
    }
    let text = wide_null(text);
    unsafe {
        SetWindowTextW(window, text.as_ptr());
    }
}

fn load_icon(width: INT, height: INT) -> HICON {
    unsafe {
        LoadImageW(
            GetModuleHandleW(null_mut()),
            101usize as *const u16,
            IMAGE_ICON,
            width,
            height,
            LR_DEFAULTCOLOR,
        ) as HICON
    }
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_COMMAND => {
            let command = wparam & 0xffff;
            let action = match command {
                CMD_SMART_CAPTURE => WM_DASHBOARD_SMART_CAPTURE,
                CMD_RECORD => WM_DASHBOARD_TOGGLE_RECORDING,
                CMD_OPEN_RECORDINGS => WM_DASHBOARD_OPEN_RECORDINGS,
                CMD_SETTINGS => WM_DASHBOARD_OPEN_SETTINGS,
                _ => return DefWindowProcW(window, message, wparam, lparam),
            };
            PostMessageW(window, action, 0, 0);
            0
        }
        WM_CLOSE => {
            ShowWindow(window, SW_HIDE);
            0
        }
        _ => DefWindowProcW(window, message, wparam, lparam),
    }
}
