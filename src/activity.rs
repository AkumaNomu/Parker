use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

static ACTIVITY_PATH: OnceLock<PathBuf> = OnceLock::new();

fn log_dir() -> PathBuf {
    crate::settings::data_directory().join("activity")
}

fn activity_path() -> PathBuf {
    ACTIVITY_PATH
        .get_or_init(|| log_dir().join("activity.jsonl"))
        .clone()
}

fn clipboard_path() -> PathBuf {
    log_dir().join("clipboard.json")
}

fn ensure_dir() -> Result<(), String> {
    let dir = log_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create activity dir: {e}"))
}

fn timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn desktop_context() -> String {
    crate::virtual_desktop::desktop_label()
}

#[derive(Serialize)]
struct ActivityEntry {
    ts: u128,
    kind: String,
    detail: String,
    desktop: String,
}

#[derive(Deserialize, Serialize)]
struct ClipboardEntry {
    ts: u128,
    text: String,
    desktop: String,
}

pub fn log_capture(kind: &str, detail: &str) {
    if !enabled() {
        return;
    }
    let entry = ActivityEntry {
        ts: timestamp(),
        kind: kind.to_string(),
        detail: detail.to_string(),
        desktop: desktop_context(),
    };
    if let Ok(line) = serde_json::to_string(&entry) {
        let _ = ensure_dir();
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(activity_path())
            .and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "{line}")
            });
    }
}

pub fn log_clipboard(text: &str) {
    if !enabled() {
        return;
    }
    let entry = ClipboardEntry {
        ts: timestamp(),
        text: text.to_string(),
        desktop: desktop_context(),
    };
    let _ = ensure_dir();
    let path = clipboard_path();
    let mut history: Vec<ClipboardEntry> = fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    history.push(entry);
    let max = std::env::var("PARKER_CLIPBOARD_HISTORY")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(50);
    if history.len() > max {
        history.drain(0..history.len() - max);
    }
    let _ = fs::write(&path, serde_json::to_string(&history).unwrap_or_default());
}

pub fn get_clipboard_history() -> Vec<String> {
    fs::read_to_string(clipboard_path())
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<ClipboardEntry>>(&s).ok())
        .unwrap_or_default()
        .into_iter()
        .rev()
        .take(20)
        .map(|e| e.text)
        .collect()
}

fn enabled() -> bool {
    std::env::var("PARKER_ACTIVITY_LOG")
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes"))
        .unwrap_or(true)
}
