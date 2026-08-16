use image::ImageReader;
use rqrr::PreparedImage;
use self_update::{backends::github::Update, Status};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn run() {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        None => Err(help()),
        Some("capture") => capture(),
        Some("record") => start_recording(),
        Some("stop") => stop_recording(),
        Some("toggle") => {
            if recording_state().is_some() {
                stop_recording()
            } else {
                start_recording()
            }
        }
        Some("open") => open_recordings(),
        Some("config") => open_settings(),
        Some("batch") => batch(args.get(1).map(Path::new).unwrap_or_else(|| Path::new("."))),
        Some("--self-update") => self_update(),
        Some("help") | Some("--help") | Some("-h") => Err(help()),
        Some(other) => Err(format!("Unknown command: {other}\n\n{}", help())),
    };
    if let Err(error) = result {
        eprintln!("{error}");
        if !error.starts_with("Usage:") {
            notify("Parker", &error);
        }
        std::process::exit(1);
    }
}

fn help() -> String {
    "Usage: parker <capture|record|stop|toggle|open|config|batch|--self-update>\n\nFedora Wayland needs: grim slurp wf-recorder ffmpeg tesseract wl-clipboard libnotify.\nBind desktop shortcuts to `parker capture`, `parker toggle`, and `parker open`.".into()
}

fn capture() -> Result<(), String> {
    require(&["tesseract", "wl-copy"])?;
    let image = capture_region()?;
    let qr = decode_qr(&image)?;
    if let Some(value) = qr {
        copy_text(&value)?;
        if value.starts_with("http://") || value.starts_with("https://") {
            let _ = Command::new("xdg-open").arg(&value).spawn();
        }
        notify("Parker", "QR copied.");
        return Ok(());
    }
    let language = detect_language(&image)?;
    let output = Command::new("tesseract")
        .arg(&image)
        .arg("stdout")
        .arg("-l")
        .arg(&language)
        .arg("--psm")
        .arg(setting("PARKER_OCR_PSM", "6"))
        .output()
        .map_err(|e| format!("Could not run Tesseract: {e}"))?;
    if !output.status.success() {
        return Err(stderr("Tesseract", &output.stderr));
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if text.is_empty() {
        return Err("No text or QR code found.".into());
    }

    match crate::translate::translate(&text, &language) {
        Ok(Some(translated)) => {
            let result = match crate::translate::translate_output_mode().as_str() {
                "translation" => translated.clone(),
                "both" => format!("{text}\n\n{translated}"),
                _ => text.clone(),
            };
            copy_text(&result)?;
            let preview = crate::translate::preview(&translated, 220);
            notify(
                "Parker",
                &format!(
                    "OCR text {} copied. Translated: {preview}",
                    language_name(&language)
                ),
            );
        }
        Ok(None) => {
            copy_text(&text)?;
            notify("Parker", "OCR text copied.");
        }
        Err(error) => {
            copy_text(&text)?;
            notify(
                "Parker",
                &format!("OCR text copied, but translation failed: {error}"),
            );
        }
    }
    Ok(())
}

fn detect_language(image: &Path) -> Result<String, String> {
    let configured = setting("PARKER_OCR_LANG", "eng");
    if !crate::translate::lang_auto_enabled() {
        return Ok(configured);
    }
    crate::translate::detect_language("tesseract", image, &configured)
}

fn language_name(language: &str) -> &str {
    match language {
        "afr" => "Afrikaans",
        "ara" => "Arabic",
        "aze" => "Azerbaijani",
        "bel" => "Belarusian",
        "bul" => "Bulgarian",
        "cat" => "Catalan",
        "ces" => "Czech",
        "chi_sim" => "Chinese (Simplified)",
        "chi_tra" => "Chinese (Traditional)",
        "dan" => "Danish",
        "deu" => "German",
        "ell" => "Greek",
        "eng" => "English",
        "fin" => "Finnish",
        "fra" => "French",
        "heb" => "Hebrew",
        "hin" => "Hindi",
        "hun" => "Hungarian",
        "ind" => "Indonesian",
        "ita" => "Italian",
        "jpn" => "Japanese",
        "kor" => "Korean",
        "mkd" => "Macedonian",
        "msa" => "Malay",
        "nld" => "Dutch",
        "nor" => "Norwegian",
        "pol" => "Polish",
        "por" => "Portuguese",
        "ron" => "Romanian",
        "rus" => "Russian",
        "slk" => "Slovak",
        "slv" => "Slovenian",
        "spa" => "Spanish",
        "srp" => "Serbian",
        "swe" => "Swedish",
        "tha" => "Thai",
        "tur" => "Turkish",
        "ukr" => "Ukrainian",
        "vie" => "Vietnamese",
        _ => language,
    }
}

fn start_recording() -> Result<(), String> {
    require(&["slurp", "wf-recorder", "ffmpeg"])?;
    if recording_state().is_some() {
        return Err("Recording already active.".into());
    }
    let region = select_region()?;
    let output = recordings_dir()?.join(format!("{}.capture.mkv", stamp()));
    let child = Command::new("wf-recorder")
        .args(["-g", &region, "-f"])
        .arg(&output)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not start wf-recorder: {e}"))?;
    fs::write(
        state_path()?,
        format!("{}\n{}", child.id(), output.display()),
    )
    .map_err(|e| format!("Could not save recording state: {e}"))?;
    notify("Parker", "Recording started. Run parker stop to finish.");
    Ok(())
}

fn stop_recording() -> Result<(), String> {
    let (pid, source) = recording_state().ok_or("No active recording.")?;
    let status = Command::new("kill")
        .args(["-INT", &pid])
        .status()
        .map_err(|e| format!("Could not stop recorder: {e}"))?;
    if !status.success() {
        return Err("Recorder is no longer running.".into());
    }
    thread::sleep(Duration::from_secs(1));
    let _ = fs::remove_file(state_path()?);
    finalize(&source)?;
    notify("Parker", "Recording optimized and copied.");
    Ok(())
}

fn batch(dir: &Path) -> Result<(), String> {
    let entries =
        fs::read_dir(dir).map_err(|e| format!("Could not read {}: {e}", dir.display()))?;
    let mut count = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "mkv")
            && path.to_string_lossy().ends_with(".capture.mkv")
        {
            finalize(&path)?;
            count += 1;
        }
    }
    if count == 0 {
        return Err("No .capture.mkv files found.".into());
    }
    Ok(())
}

fn finalize(source: &Path) -> Result<(), String> {
    require(&["ffmpeg", "wl-copy"])?;
    for _ in 0..10 {
        if source.exists() && source.metadata().map(|m| m.len() > 0).unwrap_or(false) {
            break;
        }
        thread::sleep(Duration::from_millis(500));
    }
    if !source.exists() {
        return Err("Recorder did not create a video file.".into());
    }
    let target = source.with_file_name(
        source
            .file_name()
            .unwrap()
            .to_string_lossy()
            .replace(".capture.mkv", ".mp4"),
    );
    let status = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(source)
        .args(["-map", "0:v:0", "-map", "0:a?", "-c:v", "libx264", "-crf"])
        .arg(setting("PARKER_POST_CRF", "24"))
        .args(["-preset"])
        .arg(setting("PARKER_POST_PRESET", "medium"))
        .args([
            "-pix_fmt",
            "yuv420p",
            "-movflags",
            "+faststart",
            "-c:a",
            "aac",
        ])
        .arg(&target)
        .status()
        .map_err(|e| format!("Could not run FFmpeg: {e}"))?;
    if !status.success() {
        return Err("FFmpeg could not optimize recording.".into());
    }
    copy_file(&target)?;
    fs::remove_file(source).map_err(|e| format!("Could not remove temporary recording: {e}"))?;
    Ok(())
}

fn capture_region() -> Result<PathBuf, String> {
    let path = env::temp_dir().join(format!("parker-{}.png", stamp()));
    let status = if available("grim") && available("slurp") {
        let region = select_region()?;
        Command::new("grim")
            .args(["-g", &region])
            .arg(&path)
            .status()
            .map_err(|e| format!("Could not run grim: {e}"))?
    } else {
        Command::new("spectacle")
            .args(["--region", "--background", "--nonotify", "--output"])
            .arg(&path)
            .status()
            .map_err(|e| format!("Could not run Spectacle: {e}"))?
    };
    if !status.success() {
        return Err("Screen capture failed. Check Wayland screencopy permission.".into());
    }
    Ok(path)
}

fn select_region() -> Result<String, String> {
    let output = Command::new("slurp")
        .output()
        .map_err(|e| format!("Could not run slurp: {e}"))?;
    if !output.status.success() {
        return Err("Selection cancelled.".into());
    }
    let region = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if region.is_empty() {
        return Err("Selection cancelled.".into());
    }
    Ok(region)
}

fn decode_qr(path: &Path) -> Result<Option<String>, String> {
    let image = ImageReader::open(path)
        .map_err(|e| format!("Could not read capture: {e}"))?
        .decode()
        .map_err(|e| format!("Could not decode capture: {e}"))?
        .to_luma8();
    let mut prepared = PreparedImage::prepare(image);
    let grids = prepared.detect_grids();
    for grid in grids {
        if let Ok((_meta, content)) = grid.decode() {
            return Ok(Some(content));
        }
    }
    Ok(None)
}

fn copy_text(value: &str) -> Result<(), String> {
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not start wl-copy: {e}"))?;
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .ok_or("Could not open wl-copy input.")?
        .write_all(value.as_bytes())
        .map_err(|e| format!("Could not copy text: {e}"))?;
    if child
        .wait()
        .map_err(|e| format!("Could not finish wl-copy: {e}"))?
        .success()
    {
        Ok(())
    } else {
        Err("wl-copy failed.".into())
    }
}

fn copy_file(path: &Path) -> Result<(), String> {
    let mut child = Command::new("wl-copy")
        .args(["--type", "text/uri-list"])
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not start wl-copy: {e}"))?;
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .ok_or("Could not open wl-copy input.")?
        .write_all(format!("file://{}\n", path.display()).as_bytes())
        .map_err(|e| format!("Could not copy video: {e}"))?;
    if child
        .wait()
        .map_err(|e| format!("Could not finish wl-copy: {e}"))?
        .success()
    {
        Ok(())
    } else {
        Err("wl-copy failed.".into())
    }
}

fn open_recordings() -> Result<(), String> {
    let dir = recordings_dir()?;
    Command::new("xdg-open")
        .arg(dir)
        .spawn()
        .map_err(|e| format!("Could not open recordings: {e}"))?;
    Ok(())
}

fn open_settings() -> Result<(), String> {
    let path = settings_path()?;
    if !path.exists() {
        fs::write(&path, "PARKER_OCR_LANG_AUTO=1\nPARKER_OCR_LANG=eng\nPARKER_OCR_PSM=6\nPARKER_TRANSLATE_BACKEND=none\nPARKER_TRANSLATE_TARGET=en\nPARKER_TRANSLATE_OUTPUT=original\nPARKER_POST_CRF=24\nPARKER_POST_PRESET=medium\n").map_err(|e| format!("Could not create settings: {e}"))?;
    }
    Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map_err(|e| format!("Could not open settings: {e}"))?;
    Ok(())
}

fn self_update() -> Result<(), String> {
    let status = Update::configure()
        .repo_owner("AkumaNomu")
        .repo_name("Parker")
        .bin_name("parker")
        .target("linux-x64")
        .show_download_progress(true)
        .current_version(env!("CARGO_PKG_VERSION"))
        .no_confirm(true)
        .build()
        .and_then(|update| update.update())
        .map_err(|e| format!("Self-update failed: {e}"))?;
    match status {
        Status::UpToDate(_) | Status::Updated(_) => Ok(()),
    }
}

fn recordings_dir() -> Result<PathBuf, String> {
    let path = env::var_os("PARKER_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Command::new("xdg-user-dir")
                .arg("VIDEOS")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()))
                .unwrap_or_else(|| home().join("Videos"))
                .join("Parker")
        });
    fs::create_dir_all(&path).map_err(|e| format!("Could not create {}: {e}", path.display()))?;
    Ok(path)
}

fn state_path() -> Result<PathBuf, String> {
    let dir = home().join(".local/state/parker");
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create state directory: {e}"))?;
    Ok(dir.join("recording"))
}
fn settings_path() -> Result<PathBuf, String> {
    let dir = home().join(".config/parker");
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create settings directory: {e}"))?;
    Ok(dir.join("settings.env"))
}
fn home() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
fn setting(name: &str, fallback: &str) -> String {
    env::var(name)
        .ok()
        .or_else(|| {
            fs::read_to_string(settings_path().ok()?)
                .ok()?
                .lines()
                .find_map(|line| line.strip_prefix(&format!("{name}=")).map(str::to_owned))
        })
        .unwrap_or_else(|| fallback.into())
}
fn recording_state() -> Option<(String, PathBuf)> {
    let data = fs::read_to_string(state_path().ok()?).ok()?;
    let (pid, path) = data.split_once('\n')?;
    Some((pid.into(), PathBuf::from(path)))
}
fn require(programs: &[&str]) -> Result<(), String> {
    for program in programs {
        if !available(program) {
            return Err(format!(
                "Missing {program}. Install Fedora runtime: sudo dnf install grim slurp wf-recorder ffmpeg tesseract wl-clipboard libnotify"
            ));
        }
    }
    Ok(())
}
fn available(program: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {program}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}
fn notify(title: &str, body: &str) {
    let _ = Command::new("notify-send").args([title, body]).spawn();
}
fn stderr(label: &str, output: &[u8]) -> String {
    format!("{label} failed: {}", String::from_utf8_lossy(output).trim())
}
fn stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
