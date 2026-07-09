use crate::settings;
use inquire::{Select, Text};
use std::env;
use std::fs;

const HOTKEY_KEYS: &[&str] = &[
    "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M",
    "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z",
];
const CRF_RANGE: &[u8] = &[
    18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
];
const PRESETS: &[&str] = &[
    "ultrafast", "superfast", "veryfast", "faster", "fast",
    "medium", "slow", "slower", "veryslow",
];

pub fn run_config_ui() -> Result<(), String> {
    let data_dir = settings::data_directory();
    let settings_path = data_dir.join("settings.env");
    let mut content = String::new();
    if settings_path.exists() {
        content = fs::read_to_string(&settings_path)
            .map_err(|e| format!("Could not read settings: {e}"))?;
    }

    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

    loop {
        let choices = vec![
            "Configure hotkeys",
            "Configure video quality (CRF + preset)",
            "View current settings",
            "Exit",
        ];
        let choice = Select::new("Parker configuration:", choices)
            .prompt()
            .map_err(|e| format!("Menu error: {e}"))?;

        match choice {
            "Configure hotkeys" => configure_hotkeys(&mut lines, &settings_path)?,
            "Configure video quality (CRF + preset)" => {
                configure_video(&mut lines, &settings_path)?
            }
            "View current settings" => view_settings(&lines)?,
            "Exit" => break,
            _ => break,
        }
    }
    Ok(())
}

fn configure_hotkeys(
    lines: &mut Vec<String>,
    settings_path: &Path,
) -> Result<(), String> {
    let hotkeys = [
        ("PARKER_HOTKEY_OCR", "Smart capture (OCR/QR)", "F8"),
        ("PARKER_HOTKEY_RECORD", "Region recording", "F9"),
        ("PARKER_HOTKEY_CLIP", "Clip recording", "F7"),
        ("PARKER_HOTKEY_SCROLL", "Scroll capture", "F11"),
        ("PARKER_HOTKEY_FOLDER", "Open recordings", "F10"),
        ("PARKER_HOTKEY_QUIT", "Exit Parker", "F12"),
        ("PARKER_HOTKEY_WEB", "Extract webpage", "F6"),
    ];

    for &(key, label, default) in &hotkeys {
        let current = read_setting(lines, key).unwrap_or(default);
        let prompt = format!(
            "{label} (Ctrl+Shift+{current}): pick a new key or skip to keep"
        );
        let choices: Vec<&str> = std::iter::once("(keep current)")
            .chain(HOTKEY_KEYS.iter().copied())
            .collect();
        let picked = Select::new(&prompt, choices)
            .prompt()
            .map_err(|e| format!("Selection error: {e}"))?;
        if picked != "(keep current)" {
            replace_or_append(lines, key, picked);
        }
    }

    write_settings(settings_path, lines)?;
    Ok(())
}

fn configure_video(
    lines: &mut Vec<String>,
    settings_path: &Path,
) -> Result<(), String> {
    let current_crf = read_setting(lines, "PARKER_POST_CRF")
        .and_then(|v| v.parse::<u8>().ok())
        .unwrap_or(24);
    let crf_choices: Vec<String> = CRF_RANGE
        .iter()
        .map(|c| format!("{} {}", c, if c == &current_crf { "(current)" } else { "" }))
        .collect();
    let crf_picked = Select::new(
        "Select CRF (lower = better quality, larger file):",
        crf_choices,
    )
    .prompt()
    .map_err(|e| format!("CRF selection error: {e}"))?;
    let crf: u8 = crf_picked.split_whitespace().next().unwrap_or("24").parse().unwrap_or(24);
    replace_or_append(lines, "PARKER_POST_CRF", &crf.to_string());

    let current_preset = read_setting(lines, "PARKER_POST_PRESET")
        .unwrap_or("medium");
    let preset_choices: Vec<String> = PRESETS
        .iter()
        .map(|p| format!("{p}{}", if p == &current_preset { " (current)" } else { "" }))
        .collect();
    let preset_picked = Select::new(
        "Select x264 preset (quality vs speed):",
        preset_choices,
    )
    .prompt()
    .map_err(|e| format!("Preset selection error: {e}"))?;
    let preset = preset_picked.split_whitespace().next().unwrap_or("medium");
    replace_or_append(lines, "PARKER_POST_PRESET", preset);

    write_settings(settings_path, lines)?;
    Ok(())
}

fn view_settings(lines: &[String]) -> Result<(), String> {
    println!("\n--- Current settings ---");
    if lines.is_empty() {
        println!("(no settings file)");
    } else {
        for line in lines {
            println!("{line}");
        }
    }
    println!("------------------------\n");
    Text::new("Press Enter to continue")
        .prompt()
        .map_err(|e| format!("Read error: {e}"))?;
    Ok(())
}

fn read_setting<'a>(lines: &'a [String], key: &str) -> Option<&'a str> {
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with(key) {
            if let Some((_, value)) = trimmed.split_once('=') {
                return Some(value.trim());
            }
        }
    }
    None
}

fn replace_or_append(lines: &mut Vec<String>, key: &str, value: &str) {
    let mut replaced = false;
    for line in lines.iter_mut() {
        if line.trim_start().starts_with(key) {
            *line = format!("{key}={value}");
            replaced = true;
            break;
        }
    }
    if !replaced {
        lines.push(format!("{key}={value}"));
    }
}

fn write_settings(path: &Path, lines: &[String]) -> Result<(), String> {
    let new_content = lines.join("\n");
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, &new_content)
        .and_then(|_| fs::rename(&temporary, path))
        .map_err(|e| format!("Could not write settings: {e}"))?;
    // Re-read environment for the current process
    load_settings_env(path)?;
    Ok(())
}

fn load_settings_env(path: &Path) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Could not read settings: {e}"))?;
    for line in content.lines() {
        let trimmed = line.trim().trim_start_matches('\u{feff}');
        if trimmed.is_empty() || trimmed.starts_with('#') { continue; }
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            if env::var_os(key).is_none() && key.starts_with("PARKER_") {
                env::set_var(key, value.trim());
            }
        }
    }
    Ok(())
}

use std::path::Path;
