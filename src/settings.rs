use crate::win::CREATE_NO_WINDOW;
use std::env;
use std::fs;
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_SETTINGS: &str = r#"# Parker settings
# Lines use KEY=VALUE. Restart Parker after editing.

# Smart capture
PARKER_OCR_LANG=eng
PARKER_OCR_PSM=6
PARKER_OCR_MODE=auto
PARKER_QR_AUTO_OPEN=1
PARKER_KEEP_OCR_CAPTURE=0

# Recording
PARKER_RECORD_FPS=30
PARKER_COMPRESSION=balanced
PARKER_VIDEO_ENCODER=auto
# PARKER_MAX_WIDTH=1920
# PARKER_MAX_HEIGHT=1080

# Advanced video overrides (uncomment to override the compression profile)
# PARKER_POST_CRF=24
# PARKER_POST_PRESET=medium
"#;

#[derive(Clone, Debug)]
pub struct Initialization {
    pub first_run: bool,
    pub data_directory: PathBuf,
    pub settings_path: PathBuf,
}

pub fn initialize() -> Result<Initialization, String> {
    let data_directory = data_directory();
    fs::create_dir_all(&data_directory).map_err(|error| {
        format!(
            "Could not create Parker's data directory {}: {error}",
            data_directory.display()
        )
    })?;
    fs::create_dir_all(data_directory.join("logs"))
        .map_err(|error| format!("Could not create Parker's log directory: {error}"))?;

    let settings_path = data_directory.join("settings.env");
    let first_run = !settings_path.exists();
    if first_run {
        write_atomic(&settings_path, DEFAULT_SETTINGS)?;
    }

    load_environment(&settings_path)?;

    Ok(Initialization {
        first_run,
        data_directory,
        settings_path,
    })
}

pub fn open(path: &Path) -> Result<(), String> {
    Command::new("notepad.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open {}: {error}", path.display()))
}

fn load_environment(path: &Path) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))?;

    for (index, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            return Err(format!(
                "Invalid Parker settings line {}. Expected KEY=VALUE.",
                index + 1
            ));
        };
        let key = key.trim();
        let value = expand_variables(value.trim());
        if key.is_empty() || !key.starts_with("PARKER_") {
            return Err(format!(
                "Invalid Parker setting name on line {}.",
                index + 1
            ));
        }
        if env::var_os(key).is_none() {
            env::set_var(key, value);
        }
    }

    Ok(())
}

fn expand_variables(value: &str) -> String {
    let mut output = value.to_string();
    for variable in ["LOCALAPPDATA", "USERPROFILE", "TEMP"] {
        if let Ok(replacement) = env::var(variable) {
            output = output.replace(&format!("%{variable}%"), &replacement);
        }
    }
    output
}

fn data_directory() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join("Parker")
}

fn write_atomic(path: &Path, content: &str) -> Result<(), String> {
    let temporary = path.with_extension("tmp");
    let mut file = fs::File::create(&temporary)
        .map_err(|error| format!("Could not create {}: {error}", temporary.display()))?;
    file.write_all(content.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("Could not write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("Could not finalize {}: {error}", path.display()))
}
