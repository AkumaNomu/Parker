use crate::settings;
use inquire::{Select, Text};
use std::fs;

/// Runs a minimal terminal UI that lets the user pick CRF and preset.
/// The values are written back to the Parker settings file.
pub fn run_config_ui() -> Result<(), String> {
    let crf_input = Text::new("Enter CRF (0-51, lower = better quality):")
        .prompt()
        .map_err(|e| format!("Failed to read CRF: {e}"))?;
    let crf: u8 = crf_input
        .parse()
        .map_err(|_| "Please enter an integer between 0 and 51".to_string())?;
    if crf > 51 {
        return Err("Value must be <= 51".to_string());
    }

    let presets = vec![
        "ultrafast",
        "superfast",
        "veryfast",
        "faster",
        "fast",
        "medium",
        "slow",
        "slower",
        "veryslow",
    ];
    let preset = Select::new("Select x264 preset (quality vs speed):", presets)
        .prompt()
        .map_err(|e| format!("Failed to select preset: {e}"))?;

    let data_dir = settings::data_directory();
    let settings_path = data_dir.join("settings.env");
    let mut content = String::new();
    if settings_path.exists() {
        content = fs::read_to_string(&settings_path)
            .map_err(|e| format!("Could not read settings: {e}"))?;
    }

    let mut lines: Vec<String> = content.lines().map(|line| line.to_string()).collect();
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

    replace_or_append(&mut lines, "PARKER_POST_CRF", &crf.to_string());
    replace_or_append(&mut lines, "PARKER_POST_PRESET", preset);
    let new_content = lines.join("\n");
    fs::write(&settings_path, new_content).map_err(|e| format!("Could not write settings: {e}"))?;
    Ok(())
}
