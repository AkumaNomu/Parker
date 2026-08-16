#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
mod linux;

#[cfg(any(target_os = "windows", target_os = "linux"))]
mod translate;

#[cfg(target_os = "windows")]
mod clipboard;
#[cfg(target_os = "windows")]
mod config_ui;
#[cfg(target_os = "windows")]
mod dashboard;
#[cfg(target_os = "windows")]
mod ocr;
#[cfg(target_os = "windows")]
mod qr;
#[cfg(target_os = "windows")]
mod recorder;
#[cfg(target_os = "windows")]
mod recording_indicator;
#[cfg(target_os = "windows")]
mod screenshot;
#[cfg(target_os = "windows")]
mod selector;
#[cfg(target_os = "windows")]
mod settings;
#[cfg(target_os = "windows")]
mod toast;
#[cfg(target_os = "windows")]
mod tray;
#[cfg(target_os = "windows")]
mod updater;
#[cfg(target_os = "windows")]
mod win;
#[cfg(target_os = "windows")]
mod windows_app;

#[cfg(target_os = "windows")]
fn main() {
    windows_app::run();
}

#[cfg(target_os = "linux")]
fn main() {
    linux::run();
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn main() {
    eprintln!("Parker supports Windows and Linux.");
}
