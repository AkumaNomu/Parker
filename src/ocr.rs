pub use crate::smart::OcrKind;
use crate::smart::{self, OcrMode};
use crate::win::{BELOW_NORMAL_PRIORITY_CLASS, CREATE_NO_WINDOW};
use std::env;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct OcrCapturePath {
    pub path: PathBuf,
    pub temporary: bool,
}

pub struct OcrResult {
    pub kind: OcrKind,
    pub text: String,
    pub language: String,
}

pub fn create_capture_path() -> Result<OcrCapturePath, String> {
    let keep_capture = env_flag("PARKER_KEEP_OCR_CAPTURE");
    let directory = if keep_capture {
        if let Some(profile) = env::var_os("USERPROFILE") {
            PathBuf::from(profile).join("Pictures").join("Parker")
        } else {
            env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("Parker")
        }
    } else {
        env::temp_dir().join("Parker")
    };

    fs::create_dir_all(&directory).map_err(|error| {
        format!(
            "Could not create the OCR capture directory {}: {error}",
            directory.display()
        )
    })?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("The system clock is invalid: {error}"))?
        .as_millis();

    Ok(OcrCapturePath {
        path: directory.join(format!("parker-capture-{timestamp}.bmp")),
        temporary: !keep_capture,
    })
}

pub fn recognize_smart(path: &Path) -> Result<OcrResult, String> {
    let mode = configured_mode()?;
    let language = resolved_language(path)?;

    match mode {
        OcrMode::Text => {
            let plain = smart::clean_text(&run_tesseract(path, false, &language)?);
            if plain.is_empty() {
                return Err("No text was detected in the selected region.".to_string());
            }
            Ok(OcrResult {
                kind: OcrKind::Text,
                text: plain,
                language: language.clone(),
            })
        }
        OcrMode::Code => {
            let plain = smart::clean_text(&run_tesseract(path, false, &language)?);
            if plain.is_empty() {
                return Err("No code text was detected in the selected region.".to_string());
            }
            Ok(OcrResult {
                kind: OcrKind::Code,
                text: smart::normalize_code(&plain),
                language,
            })
        }
        OcrMode::Table => {
            let tsv = run_tesseract(path, true, &language)?;
            smart::extract_table(&tsv)
                .map(|text| OcrResult {
                    kind: OcrKind::Table,
                    text,
                    language: language.clone(),
                })
                .ok_or_else(|| {
                    "The selected region did not contain a consistently aligned table.".to_string()
                })
        }
        OcrMode::Auto => {
            // Automatic mode uses a single Tesseract process. TSV provides both
            // word geometry for table inference and enough layout data to
            // rebuild normal/code text without running OCR twice.
            let tsv = run_tesseract(path, true, &language)?;
            if let Some(table) = smart::extract_table(&tsv) {
                return Ok(OcrResult {
                    kind: OcrKind::Table,
                    text: table,
                    language: language.clone(),
                });
            }

            let plain = smart::clean_text(&smart::reconstruct_text_from_tsv(&tsv));
            if plain.is_empty() {
                return Err(
                    "No text, code, table, or QR code was detected in the selected region."
                        .to_string(),
                );
            }

            if smart::looks_like_code(&plain) {
                Ok(OcrResult {
                    kind: OcrKind::Code,
                    text: smart::normalize_code(&plain),
                    language,
                })
            } else {
                Ok(OcrResult {
                    kind: OcrKind::Text,
                    text: plain,
                    language,
                })
            }
        }
    }
}

fn resolved_language(path: &Path) -> Result<String, String> {
    let configured = env::var("PARKER_OCR_LANG").unwrap_or_else(|_| "eng".to_string());
    if !crate::translate::lang_auto_enabled() {
        return Ok(configured);
    }
    let tesseract = locate_tesseract().ok_or_else(|| {
        "Tesseract OCR was not found. Run install.ps1, install Tesseract with winget, or set PARKER_TESSERACT to tesseract.exe."
            .to_string()
    })?;
    crate::translate::detect_language(&tesseract.to_string_lossy(), path, &configured)
}

fn run_tesseract(path: &Path, tsv: bool, language: &str) -> Result<String, String> {
    let tesseract = locate_tesseract().ok_or_else(|| {
        "Tesseract OCR was not found. Run install.ps1, install Tesseract with winget, or set PARKER_TESSERACT to tesseract.exe."
            .to_string()
    })?;
    let psm = configured_psm()?;

    let mut command = Command::new(&tesseract);
    command
        .creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS)
        .arg(path)
        .arg("stdout")
        .arg("-l")
        .arg(language)
        .arg("--psm")
        .arg(psm.to_string())
        .arg("-c")
        .arg("preserve_interword_spaces=1");

    if tsv {
        command.arg("tsv");
    }

    let output = command.output().map_err(|error| {
        format!(
            "Could not start Tesseract at {}: {error}",
            tesseract.display()
        )
    })?;

    if !output.status.success() {
        let details = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if details.is_empty() {
            format!("Tesseract exited with {}.", output.status)
        } else {
            format!("Tesseract failed: {details}")
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn configured_mode() -> Result<OcrMode, String> {
    let value = env::var("PARKER_OCR_MODE").unwrap_or_else(|_| "auto".to_string());
    smart::parse_mode(&value)
}

fn configured_psm() -> Result<u8, String> {
    let value = env::var("PARKER_OCR_PSM").unwrap_or_else(|_| "6".to_string());
    let psm = value
        .parse::<u8>()
        .map_err(|_| "PARKER_OCR_PSM must be an integer between 0 and 13.".to_string())?;
    if psm > 13 {
        Err("PARKER_OCR_PSM must be an integer between 0 and 13.".to_string())
    } else {
        Ok(psm)
    }
}

fn env_flag(name: &str) -> bool {
    env::var(name)
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn locate_tesseract() -> Option<PathBuf> {
    if let Some(path) = env::var_os("PARKER_TESSERACT") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    if let Ok(executable) = env::current_exe() {
        if let Some(parent) = executable.parent() {
            let bundled = parent.join("tesseract.exe");
            if bundled.is_file() {
                return Some(bundled);
            }
        }
    }

    let mut candidates = Vec::new();
    for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(root) = env::var_os(variable) {
            candidates.push(
                PathBuf::from(root)
                    .join("Tesseract-OCR")
                    .join("tesseract.exe"),
            );
        }
    }
    if let Some(local) = env::var_os("LOCALAPPDATA") {
        let local = PathBuf::from(local);
        candidates.push(
            local
                .join("Programs")
                .join("Tesseract-OCR")
                .join("tesseract.exe"),
        );
        candidates.push(
            local
                .join("Microsoft")
                .join("WinGet")
                .join("Links")
                .join("tesseract.exe"),
        );
    }

    if let Some(found) = candidates.into_iter().find(|path| path.is_file()) {
        return Some(found);
    }

    let output = Command::new("where.exe")
        .creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS)
        .arg("tesseract.exe")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .find(|path| path.is_file())
}
