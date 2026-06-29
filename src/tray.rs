use crate::win::*;
use std::mem::size_of;
use std::ptr::null_mut;

pub const WM_TRAY_CALLBACK: UINT = WM_APP + 1;
const TRAY_ICON_ID: UINT = 1;

const CMD_SMART_CAPTURE: UINT = 1001;
const CMD_RECORD: UINT = 1002;
const CMD_CLIP: UINT = 1003;
const CMD_SCROLL: UINT = 1004;
const CMD_OPEN_RECORDINGS: UINT = 1005;
const CMD_COPY_LAST_PATH: UINT = 1006;
const CMD_OPEN_SETTINGS: UINT = 1007;
const CMD_EXIT: UINT = 1008;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrayAction {
    SmartCapture,
    ToggleRecording,
    ToggleClipRecording,
    ToggleScrollCapture,
    OpenRecordings,
    CopyLastPath,
    OpenSettings,
    Exit,
}

pub fn add(window: HWND) -> Result<(), String> {
    let mut data = base_data(window);
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
    data.uCallbackMessage = WM_TRAY_CALLBACK;
    data.hIcon = load_app_icon();
    copy_wide(&mut data.szTip, "Parker — ready");

    if unsafe { Shell_NotifyIconW(NIM_ADD, &mut data) } == 0 {
        return Err("Could not add Parker to the Windows notification area.".to_string());
    }

    data.uVersion = NOTIFYICON_VERSION_4;
    unsafe {
        Shell_NotifyIconW(NIM_SETVERSION, &mut data);
    }
    Ok(())
}

pub fn remove(window: HWND) {
    let mut data = base_data(window);
    unsafe {
        Shell_NotifyIconW(NIM_DELETE, &mut data);
    }
}

pub fn set_recording(window: HWND, recording: bool, clip: bool) {
    set_status(
        window,
        if recording {
            if clip {
                "Parker — clip recording (Ctrl+Shift+F7/F9 to stop)"
            } else {
                "Parker — recording region (Ctrl+Shift+F7/F9 to stop)"
            }
        } else {
            "Parker — ready"
        },
    );
}

pub fn set_processing(window: HWND) {
    set_status(window, "Parker — optimizing recording");
}

pub fn set_scroll_capture(window: HWND, capturing: bool) {
    set_status(
        window,
        if capturing {
            "Parker — scroll capture (Ctrl+Shift+F11 to stop)"
        } else {
            "Parker — ready"
        },
    );
}

pub fn set_scroll_processing(window: HWND) {
    set_status(window, "Parker — stitching scroll capture");
}

fn set_status(window: HWND, tooltip: &str) {
    let mut data = base_data(window);
    data.uFlags = NIF_TIP | NIF_ICON;
    data.hIcon = load_app_icon();
    copy_wide(&mut data.szTip, tooltip);
    unsafe {
        Shell_NotifyIconW(NIM_MODIFY, &mut data);
    }
}

pub fn handle_callback(
    window: HWND,
    lparam: LPARAM,
    recording: bool,
    scroll_capture: bool,
    processing: bool,
    scroll_processing: bool,
    has_last_path: bool,
) -> Option<TrayAction> {
    let notification = (lparam as usize & 0xffff) as UINT;
    match notification {
        WM_LBUTTONDBLCLK => Some(TrayAction::OpenRecordings),
        WM_RBUTTONUP | WM_CONTEXTMENU => show_menu(
            window,
            recording,
            scroll_capture,
            processing,
            scroll_processing,
            has_last_path,
        ),
        _ => None,
    }
}

fn show_menu(
    window: HWND,
    recording: bool,
    scroll_capture: bool,
    processing: bool,
    scroll_processing: bool,
    has_last_path: bool,
) -> Option<TrayAction> {
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return None;
        }

        let record_busy = processing || scroll_processing || scroll_capture;
        let scroll_busy = processing || scroll_processing || recording;

        append(menu, CMD_SMART_CAPTURE, "Smart capture\tCtrl+Shift+F8");
        if record_busy {
            append_with_flags(
                menu,
                CMD_RECORD,
                "Optimizing recording…",
                MF_STRING | MF_GRAYED,
            );
            append_with_flags(menu, CMD_CLIP, "Optimizing clip…", MF_STRING | MF_GRAYED);
        } else if recording {
            append(menu, CMD_RECORD, "Stop recording\tCtrl+Shift+F9");
            append(menu, CMD_CLIP, "Stop clip recording\tCtrl+Shift+F7");
        } else {
            append(menu, CMD_RECORD, "Record a region\tCtrl+Shift+F9");
            append(menu, CMD_CLIP, "Record 30-60s clip\tCtrl+Shift+F7");
        }
        if scroll_busy {
            append_with_flags(
                menu,
                CMD_SCROLL,
                "Stitching scroll capture…",
                MF_STRING | MF_GRAYED,
            );
        } else {
            append(
                menu,
                CMD_SCROLL,
                if scroll_capture {
                    "Stop scroll capture\tCtrl+Shift+F11"
                } else {
                    "Scroll capture\tCtrl+Shift+F11"
                },
            );
        }
        AppendMenuW(menu, MF_SEPARATOR, 0, null_mut());
        append(menu, CMD_OPEN_RECORDINGS, "Open recordings\tCtrl+Shift+F10");
        if has_last_path {
            append(menu, CMD_COPY_LAST_PATH, "Copy last file path");
        } else {
            append_with_flags(
                menu,
                CMD_COPY_LAST_PATH,
                "Copy last file path",
                MF_STRING | MF_GRAYED,
            );
        }
        append(menu, CMD_OPEN_SETTINGS, "Settings");
        AppendMenuW(menu, MF_SEPARATOR, 0, null_mut());
        append(menu, CMD_EXIT, "Exit Parker\tCtrl+Shift+F12");

        let mut point = POINT::default();
        GetCursorPos(&mut point);
        SetForegroundWindow(window);
        let command = TrackPopupMenu(
            menu,
            TPM_RIGHTBUTTON | TPM_RETURNCMD | TPM_NONOTIFY,
            point.x,
            point.y,
            0,
            window,
            null_mut(),
        ) as UINT;
        DestroyMenu(menu);
        PostMessageW(window, WM_NULL, 0, 0);

        match command {
            CMD_SMART_CAPTURE => Some(TrayAction::SmartCapture),
            CMD_RECORD => Some(TrayAction::ToggleRecording),
            CMD_CLIP => Some(TrayAction::ToggleClipRecording),
            CMD_SCROLL => Some(TrayAction::ToggleScrollCapture),
            CMD_OPEN_RECORDINGS => Some(TrayAction::OpenRecordings),
            CMD_COPY_LAST_PATH => Some(TrayAction::CopyLastPath),
            CMD_OPEN_SETTINGS => Some(TrayAction::OpenSettings),
            CMD_EXIT => Some(TrayAction::Exit),
            _ => None,
        }
    }
}

unsafe fn append(menu: HMENU, id: UINT, label: &str) {
    append_with_flags(menu, id, label, MF_STRING);
}

unsafe fn append_with_flags(menu: HMENU, id: UINT, label: &str, flags: UINT) {
    let label = wide_null(label);
    AppendMenuW(menu, flags, id as usize, label.as_ptr());
}

fn base_data(window: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as DWORD,
        hWnd: window,
        uID: TRAY_ICON_ID,
        ..Default::default()
    }
}

fn load_app_icon() -> HICON {
    unsafe {
        let instance = GetModuleHandleW(null_mut());
        LoadIconW(instance, 101usize as *const u16)
    }
}

fn copy_wide<const N: usize>(destination: &mut [u16; N], text: &str) {
    destination.fill(0);
    for (slot, value) in destination
        .iter_mut()
        .take(N.saturating_sub(1))
        .zip(text.encode_utf16())
    {
        *slot = value;
    }
}
