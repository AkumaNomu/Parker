#![windows_subsystem = "windows"]

#[cfg(not(target_os = "windows"))]
compile_error!("Parker only supports Windows.");

mod activity;
mod clipboard;
mod config_ui;
mod input_capture;
mod input_controller;
mod ocr;
mod qr;
mod recorder;
mod screen_controller;
mod recording_indicator;
mod scheduler;
mod screenshot;
mod scroll_capture;
mod selector;
mod settings;
mod signals;
mod site_retriever;
mod toast;
mod tray;
mod updater;
mod virtual_desktop;
mod win;

use ocr::OcrKind;
use recorder::{Recorder, RecordingResult};
use recording_indicator::RecordingIndicator;
use scroll_capture::{ScrollCapture, ScrollCaptureResult};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::Receiver;

static SCHEDULED_ACTION: AtomicU8 = AtomicU8::new(0);
use std::thread;
use std::time::{Duration, Instant};
use tray::TrayAction;
use win::*;

const HOTKEY_OCR: i32 = 1;
const HOTKEY_RECORD: i32 = 2;
const HOTKEY_FOLDER: i32 = 3;
const HOTKEY_QUIT: i32 = 4;
const HOTKEY_CLIP: i32 = 5;
const HOTKEY_SCROLL: i32 = 6;
const HOTKEY_EXTRACT_WEB: i32 = 7;
const APP_TIMER_ID: usize = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppAction {
    SmartCapture,
    ToggleRecording,
    ToggleClipRecording,
    ToggleScrollCapture,
    StopRecording,
    OpenRecordings,
    CopyLastPath,
    OpenSettings,
    ClipboardHistory,
    ActivityLog,
    ExtractWebpage,
    TypeClipboard,
    ClickHere,
    FindTextOnScreen,
    SaveScreenshot,
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

    if args.get(1).map(|s| s.as_str()) == Some("web") {
        let url = args.get(2).expect("Usage: parker web <url> [output_dir]");
        let output_dir = args.get(3).map(Path::new).unwrap_or_else(|| Path::new(".")).join("parker-web");
        extract_webpage(url, &output_dir);
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
    let mut scroll_capture = match ScrollCapture::new(recorder.output_directory().to_path_buf()) {
        Ok(value) => value,
        Err(error) => {
            show_error(&error);
            return;
        }
    };
    let mut recording_indicator: Option<RecordingIndicator> = None;
    let mut finalization: Option<Receiver<Result<RecordingResult, String>>> = None;
    let mut scroll_finalization: Option<Receiver<Result<ScrollCaptureResult, String>>> = None;
    let mut last_recording_finished: Option<Instant> = None;
    let mut last_saved_path: Option<PathBuf> = None;
    let mut exit_after_finalization = false;

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

    let _ = virtual_desktop::initialize();
    unsafe {
        AddClipboardFormatListener(app_window);
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
    unsafe {
        SetTimer(app_window, APP_TIMER_ID, 500, None);
    }

    if initialization.first_run {
        toast::show(format!(
            "Parker initialized. Settings are stored in {}.",
            initialization.data_directory.display()
        ));
    } else {
        signals::ready();
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
            } else if scroll_finalization.is_some() {
                tray::set_scroll_processing(app_window);
            } else if scroll_capture.is_capturing() {
                tray::set_scroll_capture(app_window, true);
            } else {
                tray::set_recording(app_window, recorder.is_recording(), false);
            }
            continue;
        }

        if message.message == recorder::WM_RECORDING_FINALIZED {
            if let Some(receiver) = finalization.take() {
                last_saved_path = complete_recording(receiver, app_window);
            }
            tray::set_recording(app_window, false, false);
            last_recording_finished = Some(Instant::now());
            if exit_after_finalization {
                break 'messages;
            }
            continue;
        }

        if message.message == scroll_capture::WM_SCROLL_CAPTURE_FINALIZED {
            if let Some(receiver) = scroll_finalization.take() {
                last_saved_path = complete_scroll_capture(receiver, app_window);
            }
            tray::set_scroll_capture(app_window, false);
            continue;
        }

        if message.message == WM_TIMER && message.wParam == APP_TIMER_ID {
            if recording_indicator.is_some() && !recorder.is_recording() {
                recording_indicator.take();
                tray::set_recording(app_window, false, false);
                if let Some(error) = recorder.take_runtime_error() {
                    show_error(&error);
                }
            }
            let busy = recorder.is_recording()
                || finalization.is_some()
                || scroll_capture.is_capturing()
                || scroll_finalization.is_some();
            if let Some(action) = scheduler::check_schedule(busy) {
                let code = match map_tray_action(action) {
                    AppAction::SmartCapture => 1,
                    AppAction::ToggleRecording => 2,
                    AppAction::ToggleClipRecording => 3,
                    AppAction::ToggleScrollCapture => 4,
                    _ => 0,
                };
                if code != 0 {
                    SCHEDULED_ACTION.store(code, Ordering::Relaxed);
                }
            }
            continue;
        }

        let scheduled = match SCHEDULED_ACTION.swap(0, Ordering::Relaxed) {
            1 => Some(AppAction::SmartCapture),
            2 => Some(AppAction::ToggleRecording),
            3 => Some(AppAction::ToggleClipRecording),
            4 => Some(AppAction::ToggleScrollCapture),
            _ => None,
        };

        let action = scheduled.or_else(|| if message.message == WM_HOTKEY {
            match message.wParam as i32 {
                HOTKEY_OCR => Some(AppAction::SmartCapture),
                HOTKEY_RECORD => Some(AppAction::ToggleRecording),
                HOTKEY_CLIP => Some(AppAction::ToggleClipRecording),
                HOTKEY_SCROLL => Some(AppAction::ToggleScrollCapture),
                HOTKEY_FOLDER => Some(AppAction::OpenRecordings),
                HOTKEY_QUIT => Some(AppAction::Exit),
                HOTKEY_EXTRACT_WEB => Some(AppAction::ExtractWebpage),
                _ => None,
            }
        } else if message.message == recording_indicator::WM_RECORDING_INDICATOR_STOP {
            Some(AppAction::StopRecording)
        } else if message.message == tray::WM_TRAY_CALLBACK {
            let recording = recorder.is_recording();
            tray::handle_callback(
                app_window,
                message.lParam,
                recording,
                scroll_capture.is_capturing(),
                finalization.is_some(),
                scroll_finalization.is_some(),
                last_saved_path.is_some(),
            )
            .map(map_tray_action)
        } else if message.message == WM_CLIPBOARDUPDATE {
            if std::env::var("PARKER_ACTIVITY_LOG").map(|v| v == "0").unwrap_or(false) {
                None
            } else {
                let text = clipboard::read_text(app_window).unwrap_or_default();
                if !text.is_empty() && text.len() <= 2048 {
                    activity::log_clipboard(&text);
                }
                None
            }
        } else {
            None
        });

        if let Some(action) = action {
            match action {
                AppAction::SmartCapture => {
                    if recorder.is_recording()
                        || finalization.is_some()
                        || scroll_capture.is_capturing()
                        || scroll_finalization.is_some()
                    {
                        toast::show("Wait for the active capture to finish processing.");
                    } else {
                        activity::log_capture("smart_capture", "started");
                        run_smart_capture(app_window);
                    }
                }
                AppAction::ToggleRecording => {
                    if finalization.is_some()
                        || scroll_finalization.is_some()
                        || scroll_capture.is_capturing()
                    {
                        toast::show("Parker is still optimizing the previous recording.");
                    } else if recorder.is_recording() {
                        recording_indicator.take();
                        finalization = begin_finish_recording(&mut recorder, app_window);
                        if finalization.is_some() {
                            tray::set_processing(app_window);
                        }
                    } else if last_recording_finished
                        .is_some_and(|finished| finished.elapsed() < Duration::from_secs(1))
                    {
                        // Ignore a hotkey queued while FFmpeg was finishing.
                    } else {
                        activity::log_capture("recording", "started");
                        recording_indicator.take();
                        if let Some(selected) = start_region_recording(&mut recorder) {
                            match RecordingIndicator::show(app_window, selected) {
                                Ok(indicator) => recording_indicator = Some(indicator),
                                Err(error) => toast::show(format!(
                                    "Recording active without on-screen control: {error} Use Ctrl+Shift+F9 to stop."
                                )),
                            }
                            tray::set_recording(app_window, true, false);
                        }
                    }
                }
                AppAction::ToggleClipRecording => {
                    if finalization.is_some()
                        || scroll_finalization.is_some()
                        || scroll_capture.is_capturing()
                    {
                        toast::show("Parker is still optimizing the previous capture.");
                    } else if recorder.is_recording() {
                        recording_indicator.take();
                        finalization = begin_finish_recording(&mut recorder, app_window);
                        if finalization.is_some() {
                            tray::set_processing(app_window);
                        }
                    } else if last_recording_finished
                        .is_some_and(|finished| finished.elapsed() < Duration::from_secs(1))
                    {
                        // Ignore a hotkey queued while FFmpeg was finishing.
                    } else {
                        activity::log_capture("clip_recording", "started");
                        recording_indicator.take();
                        if let Some(selected) = start_clip_recording(&mut recorder) {
                            match RecordingIndicator::show(app_window, selected) {
                                Ok(indicator) => recording_indicator = Some(indicator),
                                Err(error) => toast::show(format!(
                                    "Clip recording active without on-screen control: {error} Use Ctrl+Shift+F7 to stop."
                                )),
                            }
                            tray::set_recording(app_window, true, true);
                        }
                    }
                }
                AppAction::ToggleScrollCapture => {
                    if finalization.is_some() || scroll_finalization.is_some() {
                        toast::show("Parker is still optimizing the previous capture.");
                    } else if recorder.is_recording() {
                        toast::show("Stop recording before starting scroll capture.");
                    } else if scroll_capture.is_capturing() {
                        scroll_finalization =
                            begin_finish_scroll_capture(&mut scroll_capture, app_window);
                        if scroll_finalization.is_some() {
                            tray::set_scroll_processing(app_window);
                        }
                    } else {
                        activity::log_capture("scroll_capture", "started");
                        match start_scroll_capture(&mut scroll_capture) {
                            Ok(()) => tray::set_scroll_capture(app_window, true),
                            Err(error) => show_error(&error),
                        }
                    }
                }
                AppAction::StopRecording => {
                    if recorder.is_recording() && finalization.is_none() {
                        recording_indicator.take();
                        finalization = begin_finish_recording(&mut recorder, app_window);
                        if finalization.is_some() {
                            tray::set_processing(app_window);
                        }
                    } else if scroll_capture.is_capturing() && scroll_finalization.is_none() {
                        scroll_finalization =
                            begin_finish_scroll_capture(&mut scroll_capture, app_window);
                        if scroll_finalization.is_some() {
                            tray::set_scroll_processing(app_window);
                        }
                    }
                }
                AppAction::OpenRecordings => {
                    open_folder(recorder.output_directory());
                    toast::show("Opened Parker recordings.");
                }
                AppAction::CopyLastPath => {
                    if let Some(path) = last_saved_path.as_deref() {
                        match clipboard::copy_text(&path.display().to_string(), app_window) {
                            Ok(()) => {
                                toast::show(format!("Copied last file path: {}", path.display()));
                                signals::success();
                            }
                            Err(error) => show_error(&format!(
                                "The last saved file is {}, but its path could not be copied: {error}",
                                path.display()
                            )),
                        }
                    } else {
                        toast::show("No saved capture yet.");
                    }
                }
                AppAction::OpenSettings => match settings::open(&initialization.settings_path) {
                    Ok(()) => toast::show("Opened Parker settings. Restart Parker after editing."),
                    Err(error) => show_error(&error),
                },
                AppAction::ClipboardHistory => {
                    let history = activity::get_clipboard_history();
                    if history.is_empty() {
                        toast::show("No clipboard history yet.");
                    } else {
                        let path = crate::settings::data_directory()
                            .join("activity")
                            .join("clipboard.json");
                        shell_open(&path);
                        let preview: String = history
                            .iter()
                            .take(3)
                            .map(|s| format!("• {}", truncate(s, 60)))
                            .collect::<Vec<_>>()
                            .join("\n");
                        toast::show(format!(
                            "Clipboard history: {} items\n{}{}",
                            history.len(),
                            preview,
                            if history.len() > 3 {
                                format!("\n… and {} more", history.len() - 3)
                            } else {
                                String::new()
                            }
                        ));
                    }
                }
                AppAction::ActivityLog => {
                    let path = crate::settings::data_directory()
                        .join("activity")
                        .join("activity.jsonl");
                    let _ = std::fs::create_dir_all(path.parent().unwrap());
                    shell_open(&path);
                    toast::show("Opened activity log.");
                }
                AppAction::TypeClipboard => {
                    input_controller::type_from_clipboard();
                    toast::show("Typed clipboard contents.");
                }
                AppAction::ClickHere => {
                    let (x, y) = input_controller::get_cursor_pos();
                    input_controller::click_left();
                    toast::show(format!("Clicked at ({x}, {y})."));
                }
                AppAction::FindTextOnScreen => {
                    let text = clipboard::read_text(app_window).unwrap_or_default();
                    if text.is_empty() {
                        toast::show("Copy text to clipboard first, then use this action.");
                    } else {
                        match screen_controller::find_text_on_screen(&text) {
                            Ok(points) if points.is_empty() => {
                                toast::show(format!("Text '{text}' not found on screen."));
                            }
                            Ok(points) => {
                                let p = &points[0];
                                input_controller::click_left_at(Some((p.x, p.y)));
                                toast::show(format!("Found and clicked '{text}' at ({}, {}).", p.x, p.y));
                            }
                            Err(e) => show_error(&e),
                        }
                    }
                }
                AppAction::SaveScreenshot => {
                    let dir = recorder.output_directory();
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis();
                    let path = dir.join(format!("screenshot_{ts}.bmp"));
                    match screen_controller::capture_primary() {
                        Ok(img) => {
                            if let Err(e) = img.save_bmp(&path) {
                                show_error(&e);
                            } else {
                                activity::log_capture("screenshot", "saved");
                                toast::show(format!("Screenshot saved: {}", path.display()));
                            }
                        }
                        Err(e) => show_error(&e),
                    }
                }
                AppAction::ExtractWebpage => {
                    #[cfg(feature = "site_retriever")]
                    {
                        use site_retriever::SiteRetriever;
                        
                        // For now, use clipboard to get URL or prompt
                        // This could be enhanced to show an input dialog
                        let output_dir = recorder.output_directory().join("parker-web");
                        if let Err(e) = std::fs::create_dir_all(&output_dir) {
                            show_error(&format!("Failed to create output directory: {}", e));
                        } else {
                            // Try to get URL from clipboard
                            if let Ok(url) = clipboard::get_text(app_window) {
                                if url.starts_with("http://") || url.starts_with("https://") {
                                    toast::show(format!("Extracting: {}", url));
                                    std::thread::spawn(move || {
                                        let retriever = match SiteRetriever::new(output_dir, true) {
                                            Ok(r) => r,
                                            Err(e) => {
                                                toast::show(format!("Error: {}", e));
                                                return;
                                            }
                                        };
                                        if let Err(e) = extract_webpage_thread(&retriever, &url) {
                                            toast::show(format!("Extraction failed: {}", e));
                                        } else {
                                            toast::show("Webpage extracted successfully!");
                                        }
                                    });
                                } else {
                                    toast::show("Clipboard doesn't contain a valid URL. Copy a URL first, then press Ctrl+Shift+F6.");
                                }
                            } else {
                                toast::show("Clipboard doesn't contain a valid URL. Copy a URL first, then press Ctrl+Shift+F6.");
                            }
                        }
                    }
                    #[cfg(not(feature = "site_retriever"))]
                    {
                        toast::show("Extract webpage feature not available. Rebuild with --features site_retriever");
                    }
                },
                AppAction::Exit => {
                    if recorder.is_recording() {
                        recording_indicator.take();
                        finalization = begin_finish_recording(&mut recorder, app_window);
                        if finalization.is_some() {
                            tray::set_processing(app_window);
                        }
                        exit_after_finalization = finalization.is_some();
                        if exit_after_finalization {
                            toast::show("Parker will exit after the recording is saved.");
                        } else {
                            break 'messages;
                        }
                    } else if scroll_capture.is_capturing() {
                        scroll_finalization =
                            begin_finish_scroll_capture(&mut scroll_capture, app_window);
                        if scroll_finalization.is_some() {
                            tray::set_scroll_processing(app_window);
                        }
                        exit_after_finalization = scroll_finalization.is_some();
                        if exit_after_finalization {
                            toast::show("Parker will exit after the scroll capture is saved.");
                        } else {
                            break 'messages;
                        }
                    } else if finalization.is_some() {
                        exit_after_finalization = true;
                        toast::show("Parker will exit after the recording is saved.");
                    } else if scroll_finalization.is_some() {
                        exit_after_finalization = true;
                        toast::show("Parker will exit after the scroll capture is saved.");
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
        TrayAction::SmartCapture => AppAction::SmartCapture,
        TrayAction::ToggleRecording => AppAction::ToggleRecording,
        TrayAction::ToggleClipRecording => AppAction::ToggleClipRecording,
        TrayAction::ToggleScrollCapture => AppAction::ToggleScrollCapture,
        TrayAction::OpenRecordings => AppAction::OpenRecordings,
        TrayAction::CopyLastPath => AppAction::CopyLastPath,
        TrayAction::OpenSettings => AppAction::OpenSettings,
        TrayAction::ClipboardHistory => AppAction::ClipboardHistory,
        TrayAction::ActivityLog => AppAction::ActivityLog,
        TrayAction::TypeClipboard => AppAction::TypeClipboard,
        TrayAction::ClickHere => AppAction::ClickHere,
        TrayAction::FindTextOnScreen => AppAction::FindTextOnScreen,
        TrayAction::SaveScreenshot => AppAction::SaveScreenshot,
        TrayAction::ExtractWebpage => AppAction::ExtractWebpage,
        TrayAction::Exit => AppAction::Exit,
    }
}

fn run_smart_capture(clipboard_owner: HWND) {
    signals::selection_started();
    let selected = match selector::select_region(
        "Select a QR code, table, code block, or text. Esc/right-click cancels.",
    ) {
        Ok(Some(rect)) => rect,
        Ok(None) => {
            signals::cancelled();
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
                signals::qr_opened();
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
        OcrKind::Text => signals::text_copied(),
        OcrKind::Code => signals::code_copied(),
        OcrKind::Table => signals::table_copied(),
    }
    Ok(())
}

fn start_region_recording(recorder: &mut Recorder) -> Option<selector::ScreenRect> {
    signals::selection_started();
    let selected = match selector::select_region(
        "Drag over the region to record. The mouse cursor will never appear in the video.",
    ) {
        Ok(Some(rect)) => rect,
        Ok(None) => {
            signals::cancelled();
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
            signals::recording_started();
            Some(selected)
        }
        Err(error) => {
            show_error(&error);
            None
        }
    }
}

fn start_clip_recording(recorder: &mut Recorder) -> Option<selector::ScreenRect> {
    signals::selection_started();
    let selected = match selector::select_region(
        "Drag over the clip region. Parker keeps the last 30-60 seconds.",
    ) {
        Ok(Some(rect)) => rect,
        Ok(None) => {
            signals::cancelled();
            return None;
        }
        Err(error) => {
            show_error(&error);
            return None;
        }
    };

    thread::sleep(Duration::from_millis(100));
    let clip_seconds = std::env::var("PARKER_RING_SECONDS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value >= 5)
        .unwrap_or(45);

    match recorder.start_clip(selected, clip_seconds) {
        Ok(_) => {
            signals::clip_recording_started(clip_seconds);
            Some(selected)
        }
        Err(error) => {
            show_error(&error);
            None
        }
    }
}

fn start_scroll_capture(scroll_capture: &mut ScrollCapture) -> Result<(), String> {
    signals::selection_started();
    let selected = match selector::select_region(
        "Drag over the scrolling region. Scroll the page, then press Ctrl+Shift+F11 again.",
    ) {
        Ok(Some(rect)) => rect,
        Ok(None) => {
            signals::cancelled();
            return Ok(());
        }
        Err(error) => return Err(error),
    };

    thread::sleep(Duration::from_millis(100));
    scroll_capture.start(selected)?;
    signals::scroll_capture_started();
    Ok(())
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

fn begin_finish_scroll_capture(
    scroll_capture: &mut ScrollCapture,
    app_window: HWND,
) -> Option<Receiver<Result<ScrollCaptureResult, String>>> {
    toast::show("Stopping and stitching the scroll capture…");
    match scroll_capture.stop_in_background(app_window) {
        Ok(receiver) => Some(receiver),
        Err(error) => {
            show_error(&error);
            None
        }
    }
}

fn complete_recording(
    receiver: Receiver<Result<RecordingResult, String>>,
    clipboard_owner: HWND,
) -> Option<PathBuf> {
    match receiver.recv() {
        Ok(Ok(result)) => {
            let saved_path = result.path.clone();
            match clipboard::copy_file(&result.path, clipboard_owner) {
                Ok(()) => {
                    signals::file_copied(&result);
                    return Some(saved_path);
                }
                Err(error) => show_error(&format!(
                    "The recording was saved to {}, but it could not be copied as a file: {error}",
                    result.path.display()
                )),
            }
            return Some(saved_path);
        }
        Ok(Err(error)) => show_error(&error),
        Err(_) => show_error("The recording finalizer ended without returning a result."),
    }
    None
}

fn complete_scroll_capture(
    receiver: Receiver<Result<ScrollCaptureResult, String>>,
    clipboard_owner: HWND,
) -> Option<PathBuf> {
    match receiver.recv() {
        Ok(Ok(result)) => {
            let saved_path = result.path.clone();
            match clipboard::copy_file(&result.path, clipboard_owner) {
                Ok(()) => {
                    signals::scroll_capture_saved(&result);
                    return Some(saved_path);
                }
                Err(error) => show_error(&format!(
                "The scroll capture was saved to {}, but it could not be copied as a file: {error}",
                result.path.display()
            )),
            }
            return Some(saved_path);
        }
        Ok(Err(error)) => show_error(&error),
        Err(_) => show_error("The scroll capture finalizer ended without returning a result."),
    }
    None
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
"F8" => VK_F8 as UINT,
        "F9" => VK_F9 as UINT,
        "F10" => VK_F10 as UINT,
        "F12" => VK_F12 as UINT,
        s if s.len() == 1 => {
            let Some(c) = s.chars().next() else {
                return (default_key, default_name.to_string());
            };
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
    let (ocr_key, ocr_name) = parse_hotkey("PARKER_HOTKEY_OCR", VK_F8 as UINT, "Ctrl+Shift+F8");
    let (rec_key, rec_name) = parse_hotkey("PARKER_HOTKEY_RECORD", VK_F9 as UINT, "Ctrl+Shift+F9");
    let (clip_key, clip_name) = parse_hotkey("PARKER_HOTKEY_CLIP", 0x76, "Ctrl+Shift+F7");
    let (scroll_key, scroll_name) = parse_hotkey("PARKER_HOTKEY_SCROLL", 0x7A, "Ctrl+Shift+F11");
    let (fol_key, fol_name) = parse_hotkey("PARKER_HOTKEY_FOLDER", VK_F10 as UINT, "Ctrl+Shift+F10");
    let (quit_key, quit_name) = parse_hotkey("PARKER_HOTKEY_QUIT", VK_F12 as UINT, "Ctrl+Shift+F12");
    let (web_key, web_name) = parse_hotkey("PARKER_HOTKEY_WEB", 0x75, "Ctrl+Shift+F6");

    let bindings: [(i32, u32, String); 7] = [
        (HOTKEY_OCR, ocr_key, ocr_name),
        (HOTKEY_RECORD, rec_key, rec_name),
        (HOTKEY_CLIP, clip_key, clip_name),
        (HOTKEY_SCROLL, scroll_key, scroll_name),
        (HOTKEY_FOLDER, fol_key, fol_name),
        (HOTKEY_QUIT, quit_key, quit_name),
        (HOTKEY_EXTRACT_WEB, web_key, web_name),
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
    for id in [
        HOTKEY_OCR,
        HOTKEY_RECORD,
        HOTKEY_CLIP,
        HOTKEY_SCROLL,
        HOTKEY_FOLDER,
        HOTKEY_QUIT,
        HOTKEY_EXTRACT_WEB,
    ] {
        unsafe {
            UnregisterHotKey(window, id);
        }
    }
}

fn open_folder(path: &Path) {
    let _ = Command::new("explorer.exe").arg(path).spawn();
}

fn shell_open(path: &std::path::Path) {
    let op = crate::win::wide_null("open");
    let ws = crate::win::wide_null(&path.display().to_string());
    unsafe {
        crate::win::ShellExecuteW(
            std::ptr::null_mut(),
            op.as_ptr(),
            ws.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            5,
        );
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

fn show_error(message: &str) {
    signals::error();
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

#[cfg(feature = "site_retriever")]
fn extract_webpage(url: &str, output_dir: &Path) {
    use site_retriever::SiteRetriever;
    
    println!("Extracting webpage: {}", url);
    println!("Output directory: {}", output_dir.display());

    let retriever = match SiteRetriever::new(output_dir.to_path_buf(), true) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };

    let page = match retriever.extract(url) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to extract page: {}", e);
            return;
        }
    };

    println!("Page title: {}", page.title);
    println!("Components found: {}", page.components.len());
    println!("CSS assets: {}", page.assets.css.len());
    println!("JS assets: {}", page.assets.js.len());
    println!("Images: {}", page.assets.images.len());
    println!("Fonts: {}", page.assets.fonts.len());

    println!("Downloading assets...");
    let mut page_mut = page;
    if let Err(e) = retriever.download_assets(&mut page_mut.assets) {
        eprintln!("Warning: Some assets failed to download: {}", e);
    }

    println!("Saving extracted data...");
    if let Err(e) = retriever.save_page(&page_mut) {
        eprintln!("Failed to save: {}", e);
        return;
    }

    println!("Done! Files saved to: {}", output_dir.display());
}

#[cfg(not(feature = "site_retriever"))]
fn extract_webpage(_url: &str, _output_dir: &Path) {
    eprintln!("Error: site_retriever feature not enabled. Rebuild with --features site_retriever");
}

#[cfg(feature = "site_retriever")]
fn extract_webpage_thread(retriever: &SiteRetriever, url: &str) -> Result<(), String> {
    let page = retriever.extract(url)?;
    
    println!("Page title: {}", page.title);
    println!("Components found: {}", page.components.len());
    println!("CSS assets: {}", page.assets.css.len());
    println!("JS assets: {}", page.assets.js.len());
    println!("Images: {}", page.assets.images.len());
    println!("Fonts: {}", page.assets.fonts.len());

    println!("Downloading assets...");
    let mut page_mut = page;
    if let Err(e) = retriever.download_assets(&mut page_mut.assets) {
        eprintln!("Warning: Some assets failed to download: {}", e);
    }

    println!("Saving extracted data...");
    retriever.save_page(&page_mut)?;

    println!("Done! Files saved to: {}", retriever.output_dir.display());
    Ok(())
}
