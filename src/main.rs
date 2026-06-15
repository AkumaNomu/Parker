#![windows_subsystem = "windows"]

#[cfg(not(target_os = "windows"))]
compile_error!("Parker only supports Windows.");

mod clipboard;
mod config_ui;
mod dashboard;
mod ocr;
mod qr;
mod recorder;
mod recording_indicator;
mod screenshot;
mod selector;
mod settings;
mod toast;
mod tray;
mod updater;
mod win;

use ocr::OcrKind;
use recorder::{Recorder, RecordingResult};
use recording_indicator::RecordingIndicator;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::ptr::null_mut;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};
use tray::TrayAction;
use win::*;

const HOTKEY_OCR: i32 = 1;
const HOTKEY_RECORD: i32 = 2;
const HOTKEY_FOLDER: i32 = 3;
const HOTKEY_QUIT: i32 = 4;
const APP_TIMER_ID: usize = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppAction {
    SmartCapture,
    ToggleRecording,
    StopRecording,
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

    // Check for self‑update flag
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--self-update") {
        if let Err(err) = updater::check_self_update() {
            show_error(&err);
        }
    }
    // If user runs "config" subcommand, launch terminal UI and exit
    if args.get(1).map(|s| s.as_str()) == Some("config") {
        if let Err(err) = config_ui::run_config_ui() {
            show_error(&err);
        }
        return;
    }

    if args.get(1).map(|s| s.as_str()) == Some("batch") {
        let dir = args.get(2).map(Path::new).unwrap_or_else(|| Path::new("."));
        batch_process(dir);
        return;
    }

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
    let mut recording_indicator: Option<RecordingIndicator> = None;
    let mut finalization: Option<Receiver<Result<RecordingResult, String>>> = None;
    let mut last_recording_finished: Option<Instant> = None;
    let mut exit_after_finalization = false;

    let app_window = match dashboard::create() {
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
    dashboard::show(app_window);

    let taskbar_created = unsafe {
        let name = wide_null("TaskbarCreated");
        RegisterWindowMessageW(name.as_ptr())
    };
    unsafe {
        SetTimer(app_window, APP_TIMER_ID, 500, None);
    }

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
            if finalization.is_some() {
                tray::set_processing(app_window);
                dashboard::set_processing(app_window);
            } else {
                tray::set_recording(app_window, recorder.is_recording());
                dashboard::set_recording(app_window, recorder.is_recording());
            }
            continue;
        }

        if message.message == recorder::WM_RECORDING_FINALIZED {
            if let Some(receiver) = finalization.take() {
                complete_recording(receiver, app_window);
            }
            tray::set_recording(app_window, false);
            dashboard::set_recording(app_window, false);
            last_recording_finished = Some(Instant::now());
            if exit_after_finalization {
                break 'messages;
            }
            continue;
        }

        if message.message == WM_TIMER && message.wParam == APP_TIMER_ID {
            if recording_indicator.is_some() && !recorder.is_recording() {
                recording_indicator.take();
                tray::set_recording(app_window, false);
                dashboard::set_recording(app_window, false);
                if let Some(error) = recorder.take_runtime_error() {
                    show_error(&error);
                }
            }
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
        } else if message.message == recording_indicator::WM_RECORDING_INDICATOR_STOP {
            Some(AppAction::StopRecording)
        } else if message.message == dashboard::WM_DASHBOARD_SMART_CAPTURE {
            Some(AppAction::SmartCapture)
        } else if message.message == dashboard::WM_DASHBOARD_TOGGLE_RECORDING {
            Some(AppAction::ToggleRecording)
        } else if message.message == dashboard::WM_DASHBOARD_OPEN_RECORDINGS {
            Some(AppAction::OpenRecordings)
        } else if message.message == dashboard::WM_DASHBOARD_OPEN_SETTINGS {
            Some(AppAction::OpenSettings)
        } else if message.message == tray::WM_TRAY_CALLBACK {
            let recording = recorder.is_recording();
            match tray::handle_callback(
                app_window,
                message.lParam,
                recording,
                finalization.is_some(),
            ) {
                Some(TrayAction::OpenParker) => {
                    dashboard::show(app_window);
                    None
                }
                Some(action) => Some(map_tray_action(action)),
                None => None,
            }
        } else {
            None
        };

        if let Some(action) = action {
            match action {
                AppAction::SmartCapture => {
                    if recorder.is_recording() || finalization.is_some() {
                        toast::show("Wait for the active recording to finish processing.");
                    } else {
                        run_smart_capture(app_window);
                    }
                }
                AppAction::ToggleRecording => {
                    if finalization.is_some() {
                        toast::show("Parker is still optimizing the previous recording.");
                    } else if recorder.is_recording() {
                        recording_indicator.take();
                        finalization = begin_finish_recording(&mut recorder, app_window);
                        if finalization.is_some() {
                            tray::set_processing(app_window);
                            dashboard::set_processing(app_window);
                        }
                    } else if last_recording_finished
                        .is_some_and(|finished| finished.elapsed() < Duration::from_secs(1))
                    {
                        // Ignore a hotkey queued while FFmpeg was finishing.
                    } else {
                        recording_indicator.take();
                        if let Some(selected) = start_region_recording(&mut recorder) {
                            match RecordingIndicator::show(app_window, selected) {
                                Ok(indicator) => recording_indicator = Some(indicator),
                                Err(error) => toast::show(format!(
                                    "Recording active without on-screen control: {error} Use Ctrl+Shift+F9 to stop."
                                )),
                            }
                            tray::set_recording(app_window, true);
                            dashboard::set_recording(app_window, true);
                        }
                    }
                }
                AppAction::StopRecording => {
                    if recorder.is_recording() && finalization.is_none() {
                        recording_indicator.take();
                        finalization = begin_finish_recording(&mut recorder, app_window);
                        if finalization.is_some() {
                            tray::set_processing(app_window);
                            dashboard::set_processing(app_window);
                        }
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
                        recording_indicator.take();
                        finalization = begin_finish_recording(&mut recorder, app_window);
                        if finalization.is_some() {
                            tray::set_processing(app_window);
                            dashboard::set_processing(app_window);
                        }
                        exit_after_finalization = finalization.is_some();
                        if exit_after_finalization {
                            toast::show("Parker will exit after the recording is saved.");
                        } else {
                            break 'messages;
                        }
                    } else if finalization.is_some() {
                        exit_after_finalization = true;
                        toast::show("Parker will exit after the recording is saved.");
                    } else {
                        break 'messages;
                    }
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
        KillTimer(app_window, APP_TIMER_ID);
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
        TrayAction::OpenParker => unreachable!("OpenParker is handled before action mapping"),
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

fn start_region_recording(recorder: &mut Recorder) -> Option<selector::ScreenRect> {
    signal_selection_started("Select the region to record.");
    let selected = match selector::select_region(
        "Drag over the region to record. The mouse cursor will never appear in the video.",
    ) {
        Ok(Some(rect)) => rect,
        Ok(None) => {
            signal_cancelled();
            return None;
        }
        Err(error) => {
            show_error(&error);
            return None;
        }
    };

    thread::sleep(Duration::from_millis(100));
    match recorder.start(selected) {
        Ok(_) => {
            signal_recording_started();
            Some(selected)
        }
        Err(error) => {
            show_error(&error);
            None
        }
    }
}

fn begin_finish_recording(
    recorder: &mut Recorder,
    app_window: HWND,
) -> Option<Receiver<Result<RecordingResult, String>>> {
    toast::show("Stopping and optimizing the recording…");
    match recorder.stop_in_background(app_window) {
        Ok(receiver) => Some(receiver),
        Err(error) => {
            show_error(&error);
            None
        }
    }
}

fn complete_recording(receiver: Receiver<Result<RecordingResult, String>>, clipboard_owner: HWND) {
    match receiver.recv() {
        Ok(Ok(result)) => match clipboard::copy_file(&result.path, clipboard_owner) {
            Ok(()) => signal_file_copied(&result),
            Err(error) => show_error(&format!(
                "The recording was saved to {}, but it could not be copied as a file: {error}",
                result.path.display()
            )),
        },
        Ok(Err(error)) => show_error(&error),
        Err(_) => show_error("The recording finalizer ended without returning a result."),
    }
}

fn qr_auto_open_enabled() -> bool {
    std::env::var("PARKER_QR_AUTO_OPEN")
        .map(|value| !matches!(value.as_str(), "0" | "false" | "FALSE" | "no" | "NO"))
        .unwrap_or(true)
}

fn parse_hotkey(env_var: &str, default_key: UINT, default_name: &str) -> (UINT, String) {
    let s = std::env::var(env_var)
        .unwrap_or_default()
        .to_ascii_uppercase();
    let key = match s.as_str() {
        "F1" => 0x70,
        "F2" => 0x71,
        "F3" => 0x72,
        "F4" => 0x73,
        "F5" => 0x74,
        "F6" => 0x75,
        "F7" => 0x76,
        "F8" => VK_F8,
        "F9" => VK_F9,
        "F10" => VK_F10,
        "F11" => 0x7A,
        "F12" => VK_F12,
        s if s.len() == 1 => {
            let c = s.chars().next().unwrap();
            if c.is_ascii_alphanumeric() {
                c as UINT
            } else {
                default_key
            }
        }
        _ => default_key,
    };

    let name = if key == default_key {
        default_name.to_string()
    } else {
        format!("Ctrl+Shift+{}", s)
    };
    (key, name)
}
fn register_hotkeys(window: HWND) -> Result<(), String> {
    let modifiers = MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT;
    let (ocr_key, ocr_name) = parse_hotkey("PARKER_HOTKEY_OCR", VK_F8, "Ctrl+Shift+F8");
    let (rec_key, rec_name) = parse_hotkey("PARKER_HOTKEY_RECORD", VK_F9, "Ctrl+Shift+F9");
    let (fol_key, fol_name) = parse_hotkey("PARKER_HOTKEY_FOLDER", VK_F10, "Ctrl+Shift+F10");
    let (quit_key, quit_name) = parse_hotkey("PARKER_HOTKEY_QUIT", VK_F12, "Ctrl+Shift+F12");

    let bindings: [(i32, u32, String); 4] = [
        (HOTKEY_OCR, ocr_key, ocr_name),
        (HOTKEY_RECORD, rec_key, rec_name),
        (HOTKEY_FOLDER, fol_key, fol_name),
        (HOTKEY_QUIT, quit_key, quit_name),
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
    toast::show(
        "Recording started. Drag the timer anywhere; click its stop button or press Ctrl+Shift+F9 to finish.",
    );
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

fn batch_process(dir: &Path) {
    if !dir.is_dir() {
        println!("Error: {} is not a directory.", dir.display());
        return;
    }

    let Ok(entries) = fs::read_dir(dir) else {
        println!("Error: Could not read directory.");
        return;
    };

    // Need to initialize settings for the output directory and compression config to work
    let _ = settings::initialize();

    let recorder = match Recorder::new() {
        Ok(r) => r,
        Err(e) => {
            println!("Error initializing recorder: {}", e);
            return;
        }
    };

    // Hack to get FFmpeg path since post_process needs it
    // Wait, wait, actually we can just find it
    let ffmpeg = match std::env::var_os("PARKER_FFMPEG")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            if let Ok(exe) = std::env::current_exe() {
                if let Some(parent) = exe.parent() {
                    let bundled = parent.join("ffmpeg.exe");
                    if bundled.is_file() {
                        return Some(bundled);
                    }
                }
            }
            let output = Command::new("where.exe").arg("ffmpeg.exe").output().ok()?;
            if output.status.success() {
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .map(|s| std::path::PathBuf::from(s.trim()))
            } else {
                None
            }
        }) {
        Some(f) => f,
        None => {
            println!("Error: FFmpeg not found. Cannot post-process.");
            return;
        }
    };

    println!("Scanning {} for .capture.mkv files...", dir.display());
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.to_string_lossy().ends_with(".capture.mkv") {
            let final_path = path.with_extension("").with_extension("mp4");
            println!("Processing {}...", path.display());
            match recorder::post_process(&ffmpeg, &path, &final_path, recorder.output_directory()) {
                Ok(encoder) => println!(
                    "Success using {encoder}. Saved to {}.",
                    final_path.display()
                ),
                Err(e) => println!("Failed: {e}"),
            }
        }
    }
    println!("Batch processing complete.");
}
