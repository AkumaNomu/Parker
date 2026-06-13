#![windows_subsystem = "windows"]

#[cfg(not(target_os = "windows"))]
compile_error!("Parker only supports Windows.");

mod clipboard;
mod ocr;
mod qr;
mod recorder;
mod screenshot;
mod selector;
mod settings;
mod toast;
mod tray;
mod win;

use ocr::OcrKind;
use recorder::{Recorder, RecordingResult};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::ptr::null_mut;
use std::thread;
use std::time::Duration;
use tray::TrayAction;
use win::*;

const HOTKEY_OCR: i32 = 1;
const HOTKEY_RECORD: i32 = 2;
const HOTKEY_FOLDER: i32 = 3;
const HOTKEY_QUIT: i32 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppAction {
    SmartCapture,
    ToggleRecording,
    OpenRecordings,
    OpenSettings,
    Exit,
}

fn main() {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 as HANDLE);
        let app_id = wide_null("Parker.Capture");
        SetCurrentProcessExplicitAppUserModelID(app_id.as_ptr());
    }

    let instance_guard = match create_single_instance_guard() {
        Ok(handle) => handle,
        Err(error) => {
            show_error(&error);
            return;
        }
    };

    let initialization = match settings::initialize() {
        Ok(value) => value,
        Err(error) => {
            show_error(&error);
            return;
        }
    };

    let mut recorder = match Recorder::new() {
        Ok(value) => value,
        Err(error) => {
            show_error(&error);
            return;
        }
    };

    let app_window = match create_app_window() {
        Ok(window) => window,
        Err(error) => {
            show_error(&error);
            return;
        }
    };

    if let Err(error) = register_hotkeys(app_window) {
        unsafe {
            DestroyWindow(app_window);
        }
        show_error(&error);
        return;
    }

    if let Err(error) = tray::add(app_window) {
        unregister_hotkeys(app_window);
        unsafe {
            DestroyWindow(app_window);
        }
        show_error(&error);
        return;
    }

    let taskbar_created = unsafe {
        let name = wide_null("TaskbarCreated");
        RegisterWindowMessageW(name.as_ptr())
    };

    if initialization.first_run {
        toast::show(format!(
            "Parker initialized. Settings are stored in {}.",
            initialization.data_directory.display()
        ));
    } else {
        signal_ready();
    }

    let mut message = MSG::default();
    'messages: loop {
        let result = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
        if result <= 0 {
            break;
        }

        if taskbar_created != 0 && message.message == taskbar_created {
            let _ = tray::add(app_window);
            tray::set_recording(app_window, recorder.is_recording());
            continue;
        }

        let action = if message.message == WM_HOTKEY {
            match message.wParam as i32 {
                HOTKEY_OCR => Some(AppAction::SmartCapture),
                HOTKEY_RECORD => Some(AppAction::ToggleRecording),
                HOTKEY_FOLDER => Some(AppAction::OpenRecordings),
                HOTKEY_QUIT => Some(AppAction::Exit),
                _ => None,
            }
        } else if message.message == tray::WM_TRAY_CALLBACK {
            let recording = recorder.is_recording();
            tray::handle_callback(app_window, message.lParam, recording).map(map_tray_action)
        } else {
            None
        };

        if let Some(action) = action {
            match action {
                AppAction::SmartCapture => {
                    if recorder.is_recording() {
                        show_error("Stop the active recording before starting a smart capture.");
                    } else {
                        run_smart_capture(app_window);
                    }
                }
                AppAction::ToggleRecording => {
                    if recorder.is_recording() {
                        finish_recording(&mut recorder, app_window);
                        tray::set_recording(app_window, false);
                    } else if start_region_recording(&mut recorder) {
                        tray::set_recording(app_window, true);
                    }
                }
                AppAction::OpenRecordings => {
                    open_folder(recorder.output_directory());
                    toast::show("Opened Parker recordings.");
                }
                AppAction::OpenSettings => match settings::open(&initialization.settings_path) {
                    Ok(()) => toast::show("Opened Parker settings. Restart Parker after editing."),
                    Err(error) => show_error(&error),
                },
                AppAction::Exit => {
                    if recorder.is_recording() {
                        finish_recording(&mut recorder, app_window);
                    }
                    break 'messages;
                }
            }
            continue;
        }

        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    tray::remove(app_window);
    unregister_hotkeys(app_window);
    unsafe {
        DestroyWindow(app_window);
        CloseHandle(instance_guard);
    }
}

fn create_single_instance_guard() -> Result<HANDLE, String> {
    let name = wide_null("Local\\ParkerCaptureSingleInstance");
    let handle = unsafe { CreateMutexW(null_mut(), FALSE, name.as_ptr()) };
    if handle.is_null() {
        return Err("Could not create Parker's single-instance guard.".to_string());
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe {
            CloseHandle(handle);
        }
        Err("Parker is already running. Use its notification-area icon or hotkeys.".to_string())
    } else {
        Ok(handle)
    }
}

fn map_tray_action(action: TrayAction) -> AppAction {
    match action {
        TrayAction::SmartCapture => AppAction::SmartCapture,
        TrayAction::ToggleRecording => AppAction::ToggleRecording,
        TrayAction::OpenRecordings => AppAction::OpenRecordings,
        TrayAction::OpenSettings => AppAction::OpenSettings,
        TrayAction::Exit => AppAction::Exit,
    }
}

fn run_smart_capture(clipboard_owner: HWND) {
    signal_selection_started("Select a region for QR detection or smart OCR.");
    let selected = match selector::select_region(
        "Select a QR code, table, code block, or text. Esc/right-click cancels.",
    ) {
        Ok(Some(rect)) => rect,
        Ok(None) => {
            signal_cancelled();
            return;
        }
        Err(error) => {
            show_error(&error);
            return;
        }
    };

    thread::sleep(Duration::from_millis(100));
    toast::show("Analyzing capture…");
    let capture = match ocr::create_capture_path() {
        Ok(capture) => capture,
        Err(error) => {
            show_error(&error);
            return;
        }
    };

    let result = screenshot::capture_region_to_bmp(selected, &capture.path)
        .and_then(|_| process_smart_capture(&capture.path, clipboard_owner));

    if capture.temporary {
        let _ = fs::remove_file(&capture.path);
    }

    if let Err(error) = result {
        show_error(&error);
    }
}

fn process_smart_capture(path: &Path, clipboard_owner: HWND) -> Result<(), String> {
    let payloads = qr::detect(path)?;
    if !payloads.is_empty() {
        let clipboard_text = payloads.join("\r\n");
        clipboard::copy_text(&clipboard_text, clipboard_owner)?;

        if let Some(url) = qr::first_web_url(&payloads).filter(|_| qr_auto_open_enabled()) {
            qr::open_web_url(url)?;
            if payloads.len() == 1 {
                signal_qr_opened();
            } else {
                toast::show(format!(
                    "Opened the first QR link and copied {} decoded values.",
                    payloads.len()
                ));
            }
        } else if payloads.len() == 1 {
            toast::show("QR content copied to the clipboard.");
        } else {
            toast::show(format!(
                "Copied {} decoded QR values to the clipboard.",
                payloads.len()
            ));
        }
        return Ok(());
    }

    let recognized = ocr::recognize_smart(path)?;
    clipboard::copy_text(&recognized.text, clipboard_owner)?;
    match recognized.kind {
        OcrKind::Text => signal_text_copied(),
        OcrKind::Code => signal_code_copied(),
        OcrKind::Table => signal_table_copied(),
    }
    Ok(())
}

fn start_region_recording(recorder: &mut Recorder) -> bool {
    signal_selection_started("Select the region to record.");
    let selected = match selector::select_region(
        "Drag over the region to record. The mouse cursor will never appear in the video.",
    ) {
        Ok(Some(rect)) => rect,
        Ok(None) => {
            signal_cancelled();
            return false;
        }
        Err(error) => {
            show_error(&error);
            return false;
        }
    };

    thread::sleep(Duration::from_millis(100));
    match recorder.start(selected) {
        Ok(_) => {
            signal_recording_started();
            true
        }
        Err(error) => {
            show_error(&error);
            false
        }
    }
}

fn finish_recording(recorder: &mut Recorder, clipboard_owner: HWND) {
    toast::show("Stopping and optimizing the recording…");
    match recorder.stop() {
        Ok(result) => match clipboard::copy_file(&result.path, clipboard_owner) {
            Ok(()) => signal_file_copied(&result),
            Err(error) => show_error(&format!(
                "The recording was saved to {}, but it could not be copied as a file: {error}",
                result.path.display()
            )),
        },
        Err(error) => show_error(&error),
    }
}

fn qr_auto_open_enabled() -> bool {
    std::env::var("PARKER_QR_AUTO_OPEN")
        .map(|value| !matches!(value.as_str(), "0" | "false" | "FALSE" | "no" | "NO"))
        .unwrap_or(true)
}

fn create_app_window() -> Result<HWND, String> {
    let class_name = wide_null("ParkerMainWindow");
    let title = wide_null("Parker");
    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    let class = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(app_window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: unsafe { LoadIconW(instance, 101usize as *const u16) },
        hCursor: unsafe { LoadCursorW(null_mut(), IDC_ARROW as *const u16) },
        hbrBackground: null_mut(),
        lpszMenuName: null_mut(),
        lpszClassName: class_name.as_ptr(),
    };

    if unsafe { RegisterClassW(&class) } == 0 {
        return Err("Could not register Parker's background window class.".to_string());
    }

    let window = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP,
            0,
            0,
            1,
            1,
            null_mut(),
            null_mut(),
            instance,
            null_mut(),
        )
    };

    if window.is_null() {
        Err("Could not create Parker's background window.".to_string())
    } else {
        Ok(window)
    }
}

unsafe extern "system" fn app_window_proc(
    window: HWND,
    message: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcW(window, message, wparam, lparam)
}

fn register_hotkeys(window: HWND) -> Result<(), String> {
    let modifiers = MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT;
    let bindings = [
        (HOTKEY_OCR, VK_F8, "Ctrl+Shift+F8"),
        (HOTKEY_RECORD, VK_F9, "Ctrl+Shift+F9"),
        (HOTKEY_FOLDER, VK_F10, "Ctrl+Shift+F10"),
        (HOTKEY_QUIT, VK_F12, "Ctrl+Shift+F12"),
    ];

    for (id, key, label) in bindings {
        if unsafe { RegisterHotKey(window, id, modifiers, key) } == 0 {
            unregister_hotkeys(window);
            return Err(format!(
                "Could not register {label}. Another application may already use it."
            ));
        }
    }

    Ok(())
}

fn unregister_hotkeys(window: HWND) {
    for id in [HOTKEY_OCR, HOTKEY_RECORD, HOTKEY_FOLDER, HOTKEY_QUIT] {
        unsafe {
            UnregisterHotKey(window, id);
        }
    }
}

fn open_folder(path: &Path) {
    let _ = Command::new("explorer.exe").arg(path).spawn();
}

fn show_error(message: &str) {
    signal_error();
    toast::show(format!("Parker error: {message}"));
    let text = wide_null(message);
    let caption = wide_null("Parker");
    unsafe {
        MessageBoxW(
            null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_OK | MB_ICONERROR | MB_TOPMOST,
        );
    }
}

fn signal_ready() {
    toast::show("Parker is ready. Ctrl+Shift+F8 captures; Ctrl+Shift+F9 records a region.");
    unsafe {
        Beep(740, 70);
        Beep(990, 90);
    }
}

fn signal_selection_started(_message: &str) {
    unsafe {
        Beep(660, 65);
    }
}

fn signal_recording_started() {
    toast::show("Region recording started. The cursor is hidden. Press Ctrl+Shift+F9 to finish.");
    unsafe {
        Beep(880, 80);
        Beep(1175, 100);
    }
}

fn signal_file_copied(result: &RecordingResult) {
    let reduction = if result.source_bytes > result.final_bytes && result.source_bytes > 0 {
        format!(
            ", {}% smaller",
            100 - (result.final_bytes.saturating_mul(100) / result.source_bytes)
        )
    } else {
        String::new()
    };
    toast::show(format!(
        "Recording optimized with {} ({}{}) and copied as an MP4 file.",
        result.encoder,
        format_bytes(result.final_bytes),
        reduction
    ));
    unsafe {
        Beep(1175, 75);
        Beep(1568, 130);
    }
}

fn format_bytes(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / MB)
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn signal_text_copied() {
    toast::show("Text recognized and copied.");
    signal_success();
}

fn signal_code_copied() {
    toast::show("Code detected and copied with its line structure preserved.");
    signal_success();
}

fn signal_table_copied() {
    toast::show("Table detected and copied as tab-separated values.");
    signal_success();
}

fn signal_qr_opened() {
    toast::show("QR link opened and copied to the clipboard.");
    signal_success();
}

fn signal_success() {
    unsafe {
        Beep(1047, 70);
        Beep(1319, 70);
        Beep(1568, 110);
    }
}

fn signal_cancelled() {
    toast::show("Capture cancelled.");
    unsafe {
        Beep(440, 80);
    }
}

fn signal_error() {
    unsafe {
        Beep(260, 220);
    }
}
