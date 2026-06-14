use crate::win::*;
use std::mem::size_of;
use std::ptr::null_mut;

pub const WM_TRAY_CALLBACK: UINT = WM_APP + 1;
const TRAY_ICON_ID: UINT = 1;

const CMD_SMART_CAPTURE: UINT = 1001;
const CMD_RECORD: UINT = 1002;
const CMD_OPEN_RECORDINGS: UINT = 1003;
const CMD_OPEN_SETTINGS: UINT = 1004;
const CMD_EXIT: UINT = 1005;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrayAction {
    SmartCapture,
    ToggleRecording,
    OpenRecordings,
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

pub fn set_recording(window: HWND, recording: bool) {
    let mut data = base_data(window);
    data.uFlags = NIF_TIP | NIF_ICON;
    data.hIcon = load_app_icon();
    copy_wide(
        &mut data.szTip,
        if recording {
            "Parker — recording region (Ctrl+Shift+F9 to stop)"
        } else {
            "Parker — ready"
        },
    );
    unsafe {
        Shell_NotifyIconW(NIM_MODIFY, &mut data);
    }
}

pub fn handle_callback(window: HWND, lparam: LPARAM, recording: bool) -> Option<TrayAction> {
    let notification = (lparam as usize & 0xffff) as UINT;
    match notification {
        WM_LBUTTONDBLCLK => Some(TrayAction::OpenRecordings),
        WM_RBUTTONUP | WM_CONTEXTMENU => show_menu(window, recording),
        _ => None,
    }
}

fn show_menu(window: HWND, recording: bool) -> Option<TrayAction> {
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return None;
        }

        append(menu, CMD_SMART_CAPTURE, "Smart capture\tCtrl+Shift+F8");
        append(
            menu,
            CMD_RECORD,
            if recording {
                "Stop and optimize recording\tCtrl+Shift+F9"
            } else {
                "Record a region\tCtrl+Shift+F9"
            },
        );
        AppendMenuW(menu, MF_SEPARATOR, 0, null_mut());
        append(menu, CMD_OPEN_RECORDINGS, "Open recordings\tCtrl+Shift+F10");
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
            CMD_OPEN_RECORDINGS => Some(TrayAction::OpenRecordings),
            CMD_OPEN_SETTINGS => Some(TrayAction::OpenSettings),
            CMD_EXIT => Some(TrayAction::Exit),
            _ => None,
        }
    }
}

unsafe fn append(menu: HMENU, id: UINT, label: &str) {
    let label = wide_null(label);
    AppendMenuW(menu, MF_STRING, id as usize, label.as_ptr());
}

fn base_data(window: HWND) -> NOTIFYICONDATAW {
    let mut data = NOTIFYICONDATAW::default();
    data.cbSize = size_of::<NOTIFYICONDATAW>() as DWORD;
    data.hWnd = window;
    data.uID = TRAY_ICON_ID;
    data
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
