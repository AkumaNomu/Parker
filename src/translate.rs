use std::env;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const SCRIPT_LANGUAGES: &[(&str, &[&str])] = &[
    (
        "Latin",
        &[
            "eng", "deu", "fra", "spa", "ita", "por", "nld", "pol", "tur", "swe", "dan", "nor",
            "fin", "ron", "cat", "ces", "hun", "vie", "ind", "msa",
        ],
    ),
    ("Cyrillic", &["rus", "ukr", "bul", "srp", "mkd", "bel"]),
    ("Greek", &["ell"]),
    ("Han", &["chi_sim", "chi_tra", "jpn"]),
    ("Hangul", &["kor"]),
    ("Arabic", &["ara", "fas", "urd"]),
    ("Hebrew", &["heb"]),
    ("Devanagari", &["hin", "mar", "nep"]),
    ("Thai", &["tha"]),
    ("Tamil", &["tam"]),
    ("Telugu", &["tel"]),
    ("Kannada", &["kan"]),
    ("Malayalam", &["mal"]),
    ("Bengali", &["ben"]),
    ("Gujarati", &["guj"]),
    ("Gurmukhi", &["pan"]),
    ("Sinhala", &["sin"]),
    ("Armenian", &["hye"]),
    ("Georgian", &["kat"]),
    ("Khmer", &["khm"]),
    ("Myanmar", &["mya"]),
    ("Oriya", &["ori"]),
    ("Ethiopic", &["amh"]),
];

fn iso_language(language: &str) -> &str {
    match language {
        "afr" => "af",
        "ara" => "ar",
        "aze" => "az",
        "bel" => "be",
        "bul" => "bg",
        "ben" => "bn",
        "cat" => "ca",
        "ces" => "cs",
        "dan" => "da",
        "deu" => "de",
        "ell" => "el",
        "eng" => "en",
        "est" => "et",
        "fas" => "fa",
        "fin" => "fi",
        "fra" => "fr",
        "guj" => "gu",
        "heb" => "he",
        "hin" => "hi",
        "hrv" => "hr",
        "hun" => "hu",
        "hye" => "hy",
        "ind" => "id",
        "isl" => "is",
        "ita" => "it",
        "jpn" => "ja",
        "kat" => "ka",
        "khm" => "km",
        "kan" => "kn",
        "kor" => "ko",
        "lit" => "lt",
        "lav" => "lv",
        "mkd" => "mk",
        "mal" => "ml",
        "mar" => "mr",
        "msa" => "ms",
        "nld" => "nl",
        "nor" => "no",
        "pol" => "pl",
        "por" => "pt",
        "ron" => "ro",
        "rus" => "ru",
        "sin" => "si",
        "slk" => "sk",
        "slv" => "sl",
        "sqi" => "sq",
        "srp" => "sr",
        "swe" => "sv",
        "tam" => "ta",
        "tel" => "te",
        "tha" => "th",
        "tur" => "tr",
        "ukr" => "uk",
        "urd" => "ur",
        "vie" => "vi",
        "zho" => "zh",
        "chi_sim" => "zh",
        "chi_tra" => "zh",
        _ => "auto",
    }
}

pub fn lang_auto_enabled() -> bool {
    !matches!(
        env::var("PARKER_OCR_LANG_AUTO").as_deref(),
        Ok("0" | "false" | "FALSE" | "no" | "NO" | "off" | "OFF")
    )
}

pub fn translate_backend() -> Option<String> {
    env::var("PARKER_TRANSLATE_BACKEND")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty() && value != "none")
}

pub fn translate_target() -> String {
    env::var("PARKER_TRANSLATE_TARGET")
        .unwrap_or_else(|_| "en".to_string())
        .trim()
        .to_ascii_lowercase()
}

pub fn preview(text: &str, max_chars: usize) -> String {
    let single_line: String = text
        .chars()
        .map(|character| if character == '\n' { ' ' } else { character })
        .collect();
    let mut trimmed = single_line.trim().to_string();
    if trimmed.chars().count() > max_chars {
        trimmed = trimmed.chars().take(max_chars).collect();
        trimmed.push('…');
    }
    trimmed
}

pub fn translate_output_mode() -> String {
    env::var("PARKER_TRANSLATE_OUTPUT")
        .unwrap_or_else(|_| "original".to_string())
        .trim()
        .to_ascii_lowercase()
}

pub fn detect_language(
    tesseract_path: &str,
    image: &Path,
    configured: &str,
) -> Result<String, String> {
    if !lang_auto_enabled() {
        return Ok(configured.to_string());
    }

    let installed = installed_languages(tesseract_path)?;
    let script = match detect_script(tesseract_path, image) {
        Some(script) => script,
        None => return Ok(configured.to_string()),
    };

    let mut candidates: Vec<&str> = SCRIPT_LANGUAGES
        .iter()
        .find(|(name, _)| *name == script)
        .map(|(_, languages)| languages.to_vec())
        .unwrap_or_default();
    candidates.retain(|language| installed.iter().any(|installed| installed == language));
    candidates.dedup();

    if candidates.is_empty() {
        return Ok(configured.to_string());
    }
    if candidates.len() == 1 {
        return Ok(candidates[0].to_string());
    }

    let mut best = candidates[0].to_string();
    let mut best_confidence = None;
    for language in candidates.iter().take(8) {
        let confidence = mean_confidence(tesseract_path, image, language);
        if confidence > best_confidence {
            best_confidence = confidence;
            best = (*language).to_string();
        }
    }
    Ok(best)
}

fn installed_languages(tesseract_path: &str) -> Result<Vec<String>, String> {
    let output = tesseract_command(tesseract_path)
        .arg("--list-langs")
        .output()
        .map_err(|error| format!("Could not list Tesseract languages: {error}"))?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .skip(1)
        .map(str::trim)
        .filter(|line| !line.is_empty() && *line != "osd")
        .map(str::to_string)
        .collect())
}

fn detect_script(tesseract_path: &str, image: &Path) -> Option<String> {
    let output = tesseract_command(tesseract_path)
        .arg(image)
        .arg("stdout")
        .arg("-l")
        .arg("osd")
        .arg("--psm")
        .arg("0")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("Script:")
                .map(str::trim)
                .map(str::to_string)
        })
        .filter(|script| !script.is_empty())
}

fn mean_confidence(tesseract_path: &str, image: &Path, language: &str) -> Option<f64> {
    let output = tesseract_command(tesseract_path)
        .arg(image)
        .arg("stdout")
        .arg("-l")
        .arg(language)
        .arg("--psm")
        .arg("6")
        .arg("tsv")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut total = 0.0f64;
    let mut count = 0usize;
    for line in stdout.lines().skip(1) {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 12 || fields[0] != "5" {
            continue;
        }
        if let Ok(confidence) = fields[10].parse::<f64>() {
            if confidence >= 0.0 {
                total += confidence;
                count += 1;
            }
        }
    }
    if count == 0 {
        None
    } else {
        Some(total / count as f64)
    }
}

#[cfg(target_os = "windows")]
fn tesseract_command(path: &str) -> Command {
    use crate::win::{BELOW_NORMAL_PRIORITY_CLASS, CREATE_NO_WINDOW};
    use std::os::windows::process::CommandExt;
    let mut command = Command::new(path);
    command.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
    command
}

#[cfg(not(target_os = "windows"))]
fn tesseract_command(path: &str) -> Command {
    Command::new(path)
}

pub fn translate(text: &str, source_language: &str) -> Result<Option<String>, String> {
    let backend = match translate_backend() {
        Some(backend) => backend,
        None => return Ok(None),
    };
    let target = translate_target();
    let source = iso_language(source_language);

    let translated = match backend.as_str() {
        "argos" => translate_argos(text, source, &target)?,
        "libretranslate" => translate_libretranslate(text, source, &target)?,
        other => {
            return Err(format!(
                "PARKER_TRANSLATE_BACKEND must be none, argos, or libretranslate, not {other}."
            ))
        }
    };
    if translated.trim().is_empty() {
        return Err("The translation service returned an empty result.".to_string());
    }
    Ok(Some(translated.trim().to_string()))
}

fn translate_argos(text: &str, source: &str, target: &str) -> Result<String, String> {
    let output = Command::new("argos-translate")
        .args(["--from-lang", source, "--to-lang"])
        .arg(target)
        .arg(text)
        .output()
        .map_err(|error| {
            format!("Could not run argos-translate: {error}. Install it with: pip install argos-translate")
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "argos-translate failed.".to_string()
        } else {
            format!("argos-translate failed: {stderr}")
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn translate_libretranslate(text: &str, source: &str, target: &str) -> Result<String, String> {
    let endpoint = env::var("PARKER_TRANSLATE_ENDPOINT").map_err(|_| {
        "PARKER_TRANSLATE_ENDPOINT must be set, for example http://localhost:5000.".to_string()
    })?;
    let url = format!("{}/translate", endpoint.trim_end_matches('/'));
    let body = serde_json::json!({
        "q": text,
        "source": source,
        "target": target,
        "format": "text"
    })
    .to_string();

    let mut command = Command::new("curl");
    #[cfg(target_os = "windows")]
    {
        use crate::win::CREATE_NO_WINDOW;
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
        .args(["-sS", "-X", "POST", "-H", "Content-Type: application/json"])
        .arg("--data-binary")
        .arg("@-")
        .arg(&url)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().map_err(|error| {
        format!("Could not run curl: {error}. Install curl or use the argos backend.")
    })?;
    child
        .stdin
        .take()
        .ok_or("Could not open curl stdin.")?
        .write_all(body.as_bytes())
        .map_err(|error| format!("Could not write translation request to curl: {error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("Could not read curl output: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "LibreTranslate request failed.".to_string()
        } else {
            format!("LibreTranslate request failed: {stderr}")
        });
    }

    let value: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Could not read LibreTranslate response: {error}"))?;
    value
        .get("translatedText")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .ok_or_else(|| format!("LibreTranslate returned an unexpected response: {value}"))
}
