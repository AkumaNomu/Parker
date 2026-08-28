use crate::smart::{self, OcrKind, OcrMode};
use image::imageops::FilterType;
use image::{GrayImage, ImageReader};
use rqrr::PreparedImage;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Session {
    Wayland,
    X11,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ClipboardTool {
    WlCopy,
    XClip,
}

pub fn run() {
    load_settings();
    let args: Vec<String> = env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        None => show_gui(),
        Some("capture") => capture(),
        Some("shot") => shot(),
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
        Some("gui") => show_gui(),
        Some("shortcuts") => show_shortcuts(),
        Some("--self-update") => self_update(),
        Some("--version") | Some("-V") => {
            println!("Parker {}", crate::updater::current_version());
            Ok(())
        }
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

fn show_gui() -> Result<(), String> {
    if env::var_os("DISPLAY").is_none() && env::var_os("WAYLAND_DISPLAY").is_none() {
        return show_shortcuts();
    }
    loop {
        let Some(action) = gui_home_action()? else {
            return Ok(());
        };
        let action = if action == "More" {
            let Some(action) = gui_more_action()? else {
                continue;
            };
            action
        } else {
            action
        };
        let result = match action.as_str() {
            "Smart capture" => capture(),
            "Screenshot" => shot(),
            "Record region" => start_recording(),
            "Stop recording" => stop_recording(),
            "Open recordings" => open_recordings(),
            "Settings" => edit_settings_gui(),
            "Open settings file" => open_settings(),
            "Check for updates" => self_update(),
            "Shortcuts" => show_shortcuts_gui(&shortcuts_text()),
            _ => Ok(()),
        };
        if let Err(error) = result {
            if !is_gui_cancellation(&error) {
                show_gui_error(&error);
            }
        }
    }
}

fn gui_home_action() -> Result<Option<String>, String> {
    let (recording, recoverable_recording) = gui_recording_state();
    let recording_action = recording_action(recording || recoverable_recording);
    let gnome_wayland = is_gnome_wayland();
    let status = if recording {
        "Recording active. Stop recording finalizes and copies it."
    } else if recoverable_recording {
        "The recorder stopped. Stop recording recovers the saved video."
    } else if gnome_wayland {
        "Ready for a selected region. Recording is unavailable on GNOME Wayland."
    } else {
        "Ready for a selected region. Capture stays on this machine."
    };
    let text = format!("Parker {}\n\n{status}", crate::updater::current_version());
    if available("zenity") {
        let mut command = Command::new("zenity");
        command.args([
            "--question",
            "--title",
            "Parker",
            "--text",
            &text,
            "--ok-label",
            "Smart capture",
            "--cancel-label",
            "Close",
            "--width",
            "900",
        ]);
        command.args(["--extra-button", "Screenshot"]);
        if !gnome_wayland || recording || recoverable_recording {
            command.args(["--extra-button", recording_action]);
        }
        command.args(["--extra-button", "More"]);
        let output = command
            .output()
            .map_err(|error| format!("Could not open Parker GUI: {error}"))?;
        return Ok(dialog_choice(&output, "Smart capture"));
    }
    if available("kdialog") {
        let mut command = Command::new("kdialog");
        command.args(["--title", "Parker", "--menu", &text]);
        command.args(["Smart capture", "Copy QR, table, code, or text"]);
        command.args(["Screenshot", "Copy selected image"]);
        if !gnome_wayland || recording || recoverable_recording {
            command.args([recording_action, "Start or finalize recording"]);
        }
        command.args(["More", "Settings, recordings, updates, and shortcuts"]);
        let output = command
            .output()
            .map_err(|error| format!("Could not open Parker GUI: {error}"))?;
        return Ok(output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .filter(|action| !action.is_empty()));
    }
    show_shortcuts()
        .map(|()| None)
        .map_err(|error| format!("Could not open Parker GUI: {error}"))
}

fn gui_more_action() -> Result<Option<String>, String> {
    if available("zenity") {
        let output = Command::new("zenity")
            .args([
                "--question",
                "--title",
                "Parker",
                "--text",
                "Settings and utilities",
                "--ok-label",
                "Open recordings",
                "--cancel-label",
                "Back",
                "--width",
                "1100",
                "--extra-button",
                "Settings",
                "--extra-button",
                "Open settings file",
                "--extra-button",
                "Check for updates",
                "--extra-button",
                "Shortcuts",
            ])
            .output()
            .map_err(|error| format!("Could not open Parker GUI: {error}"))?;
        return Ok(dialog_choice(&output, "Open recordings"));
    }
    if available("kdialog") {
        let output = Command::new("kdialog")
            .args([
                "--title",
                "Parker",
                "--menu",
                "Settings and utilities",
                "Open recordings",
                "Open output folder",
                "Settings",
                "Edit common settings",
                "Open settings file",
                "Open full settings.env",
                "Check for updates",
                "Check GitHub Releases",
                "Shortcuts",
                "Show commands and keyboard shortcuts",
            ])
            .output()
            .map_err(|error| format!("Could not open Parker GUI: {error}"))?;
        return Ok(output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .filter(|action| !action.is_empty()));
    }
    Ok(None)
}

fn dialog_choice(output: &std::process::Output, primary: &str) -> Option<String> {
    dialog_choice_text(
        output.status.success(),
        &String::from_utf8_lossy(&output.stdout),
        primary,
    )
}

fn dialog_choice_text(success: bool, output: &str, primary: &str) -> Option<String> {
    let action = output.trim().to_string();
    if !action.is_empty() {
        Some(action)
    } else if success {
        Some(primary.to_string())
    } else {
        None
    }
}

fn recording_action(recording: bool) -> &'static str {
    if recording {
        "Stop recording"
    } else {
        "Record region"
    }
}

fn gui_recording_state() -> (bool, bool) {
    let Some((pid, source)) = recording_state() else {
        return (false, false);
    };
    if !source.exists() {
        return (false, false);
    }
    let active = pid
        .parse::<u32>()
        .ok()
        .is_some_and(|pid| recorder_alive(pid) && is_wf_recorder(pid));
    (active, true)
}

fn is_gui_cancellation(error: &str) -> bool {
    matches!(error.trim(), "Selection cancelled." | "Selection canceled.")
}

fn show_gui_error(error: &str) {
    if available("zenity") {
        let _ = Command::new("zenity")
            .args(["--error", "--title", "Parker", "--text", error])
            .status();
    } else if available("kdialog") {
        let _ = Command::new("kdialog")
            .args(["--title", "Parker", "--error", error])
            .status();
    } else {
        eprintln!("{error}");
    }
}

fn show_shortcuts() -> Result<(), String> {
    let text = shortcuts_text();
    println!("{text}");
    let _ = show_shortcuts_gui(&text);
    Ok(())
}

fn shortcuts_text() -> String {
    format!(
        "Parker {}\n\n\
         Shortcuts (bind in Settings → Keyboard):\n  \
         Ctrl+Shift+F8  →  parker capture   (QR / table / code / text)\n  \
         Ctrl+Shift+F9  →  parker shot      (image to clipboard)\n  \
         Ctrl+Shift+F10 →  parker open      (recordings folder)\n  \
         Ctrl+Shift+F11 →  parker toggle    (start/stop recording)\n\n\
         Commands:\n  \
         gui       — action buttons and common settings\n  \
         capture  — select region → QR or smart OCR to clipboard\n  \
         shot     — select region → image to clipboard\n  \
         record   — select region and start recording\n  \
         stop     — finalize MP4 and copy file URI\n  \
         toggle   — start or stop recording\n  \
         open     — open recordings folder\n  \
         config   — open settings file (~/.config/parker/settings.env)\n  \
         batch DIR — finalize leftover .capture.mkv files\n  \
         shortcuts — show command reference\n  \
         --self-update — update from GitHub Releases\n  \
         --version     — print version\n\n\
         Tip: Esc or right-click cancels the selector.",
        crate::updater::current_version()
    )
}

fn show_shortcuts_gui(text: &str) -> Result<(), String> {
    if env::var_os("DISPLAY").is_none() && env::var_os("WAYLAND_DISPLAY").is_none() {
        return Ok(());
    }
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    if available("zenity") {
        let _ = Command::new("zenity")
            .args([
                "--info",
                "--title",
                "Parker — Shortcuts",
                "--text",
                text,
                "--width",
                "560",
                "--height",
                "380",
                "--no-wrap",
            ])
            .status();
        return Ok(());
    }
    if available("kdialog") {
        let _ = Command::new("kdialog")
            .args(["--title", "Parker — Shortcuts", "--msgbox", text])
            .status();
        return Ok(());
    }
    if available("yad") {
        let _ = Command::new("yad")
            .args([
                "--title",
                "Parker — Shortcuts",
                "--text",
                text,
                "--button",
                "OK:0",
                "--width",
                "560",
                "--height",
                "380",
                "--center",
            ])
            .status();
        return Ok(());
    }
    let html = format!(
        "<!doctype html><meta charset=utf-8><title>Parker — Shortcuts</title>\
         <style>body{{font:14px monospace;background:#1c1d21;color:#f5f5f7;padding:28px;line-height:1.6}}\
         h1{{font-size:20px;margin:0 0 12px}}pre{{white-space:pre-wrap;background:#232428;padding:16px;border-radius:8px}}\
         a{{color:#8ab4ff}}</style><h1>Parker — Shortcuts</h1><pre>{}</pre>",
        escaped
    );
    if let Ok(path) = write_shortcuts_html(&html) {
        let _ = Command::new("xdg-open").arg(&path).spawn();
    }
    Ok(())
}

fn write_shortcuts_html(html: &str) -> Result<PathBuf, String> {
    let path = env::temp_dir().join(format!("parker-shortcuts-{}.html", std::process::id()));
    fs::write(&path, html).map_err(|e| format!("Could not write shortcuts page: {e}"))?;
    Ok(path)
}

fn help() -> String {
    "Usage: parker <gui|capture|shot|record|stop|toggle|open|config|batch|--self-update|--version>\n\n\
     gui      Open action buttons and common settings.\n\
     capture  Select a region and copy QR data or smart OCR text (table/code/text).\n\
     shot     Select a region and copy the screenshot image itself.\n\
     record   Select a region and start recording; `stop` finalizes an MP4 and copies it.\n\
     toggle   Start or stop a region recording.\n\
     open     Open the recordings folder.\n\
     config   Open the settings file.\n\
     batch DIR  Finalize any .capture.mkv files in DIR (default: current directory).\n\n\
     Bind desktop shortcuts to `parker capture` (F8), `parker shot` (F9), `parker open` (F10), and `parker toggle` (F11)."
        .into()
}

fn session() -> Session {
    if env::var_os("WAYLAND_DISPLAY").is_some() {
        Session::Wayland
    } else if env::var_os("DISPLAY").is_some() {
        Session::X11
    } else {
        Session::Unknown
    }
}

fn clipboard_tool() -> Result<ClipboardTool, String> {
    let preferred: &[ClipboardTool] = match session() {
        Session::Wayland => &[ClipboardTool::WlCopy, ClipboardTool::XClip],
        _ => &[ClipboardTool::XClip, ClipboardTool::WlCopy],
    };
    for tool in preferred {
        let program = match tool {
            ClipboardTool::WlCopy => "wl-copy",
            ClipboardTool::XClip => "xclip",
        };
        if available(program) {
            return Ok(*tool);
        }
    }
    Err("No clipboard tool found. Install wl-copy (Wayland) or xclip (X11).".into())
}

fn clipboard_spawn(tool: ClipboardTool, mime: Option<&str>) -> Result<std::process::Child, String> {
    match tool {
        ClipboardTool::WlCopy => {
            let mut command = Command::new("wl-copy");
            if let Some(mime) = mime {
                command.args(["--type", mime]);
            }
            command
                .stdin(Stdio::piped())
                .spawn()
                .map_err(|e| format!("Could not start wl-copy: {e}"))
        }
        ClipboardTool::XClip => {
            let mut command = Command::new("xclip");
            command.args(["-selection", "clipboard"]);
            if let Some(mime) = mime {
                command.args(["-t", mime]);
            }
            command
                .args(["-i"])
                .stdin(Stdio::piped())
                .spawn()
                .map_err(|e| format!("Could not start xclip: {e}"))
        }
    }
}

fn clipboard_finish(
    mut child: std::process::Child,
    payload: &[u8],
    label: &str,
) -> Result<(), String> {
    child
        .stdin
        .as_mut()
        .ok_or_else(|| format!("Could not open {label} input."))?
        .write_all(payload)
        .map_err(|e| format!("Could not write to {label}: {e}"))?;
    drop(child.stdin.take());
    if child
        .wait()
        .map_err(|e| format!("Could not finish {label}: {e}"))?
        .success()
    {
        Ok(())
    } else {
        Err(format!("{label} failed."))
    }
}

fn copy_text(value: &str) -> Result<(), String> {
    let tool = clipboard_tool()?;
    let mime = match tool {
        ClipboardTool::WlCopy => None,
        ClipboardTool::XClip => Some("text/plain"),
    };
    clipboard_spawn(tool, mime)
        .and_then(|child| clipboard_finish(child, value.as_bytes(), "clipboard"))
}

fn copy_file(path: &Path) -> Result<(), String> {
    let tool = clipboard_tool()?;
    let uri = format!("file://{}\n", path.display());
    let mime = match tool {
        ClipboardTool::WlCopy => Some("text/uri-list"),
        ClipboardTool::XClip => Some("text/uri-list"),
    };
    clipboard_spawn(tool, mime)
        .and_then(|child| clipboard_finish(child, uri.as_bytes(), "file clipboard"))
}

fn copy_image(path: &Path) -> Result<(), String> {
    let tool = clipboard_tool()?;
    let png = fs::read(path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    let mime = match tool {
        ClipboardTool::WlCopy => Some("image/png"),
        ClipboardTool::XClip => Some("image/png"),
    };
    clipboard_spawn(tool, mime).and_then(|child| clipboard_finish(child, &png, "image clipboard"))
}

fn capture() -> Result<(), String> {
    require(&["tesseract"])?;
    let keep = flag_or("PARKER_KEEP_OCR_CAPTURE", false);
    let image = capture_region_png(keep)?;
    let outcome = smart_capture(&image);
    if !keep {
        let _ = fs::remove_file(&image);
    }
    match outcome? {
        Some(text) => deliver_text(kind_label(text.0), &text.1, &text.2),
        None => Ok(()),
    }
}

fn kind_label(kind: OcrKind) -> &'static str {
    match kind {
        OcrKind::Table => "Table",
        OcrKind::Code => "Code",
        OcrKind::Text => "Text",
    }
}

fn smart_capture(image: &Path) -> Result<Option<(OcrKind, String, String)>, String> {
    let qr_payloads = decode_qr_with_retry(image)?;
    if !qr_payloads.is_empty() {
        copy_text(&qr_payloads.join("\n"))?;
        if flag_or("PARKER_QR_AUTO_OPEN", true) {
            if let Some(url) = qr_payloads
                .iter()
                .find(|payload| crate::qr_common::is_safe_web_url(payload))
            {
                let _ = Command::new("xdg-open").arg(url).spawn();
            }
        }
        notify(
            "Parker",
            &format!("Copied {} QR value(s).", qr_payloads.len()),
        );
        return Ok(None);
    }

    let language = detect_language(image)?;
    let mode = smart::parse_mode(&setting("PARKER_OCR_MODE", "auto"))?;

    let result = match mode {
        OcrMode::Text => {
            let plain = smart::clean_text(&run_tesseract(image, false, &language)?);
            if plain.is_empty() {
                return Err("No text was detected in the selected region.".into());
            }
            (OcrKind::Text, plain, language)
        }
        OcrMode::Code => {
            let plain = smart::clean_text(&run_tesseract(image, false, &language)?);
            if plain.is_empty() {
                return Err("No code text was detected in the selected region.".into());
            }
            (OcrKind::Code, smart::normalize_code(&plain), language)
        }
        OcrMode::Table => {
            let tsv = run_tesseract(image, true, &language)?;
            smart::extract_table(&tsv)
                .map(|text| (OcrKind::Table, text, language.clone()))
                .ok_or_else(|| {
                    "The selected region did not contain a consistently aligned table.".to_string()
                })?
        }
        OcrMode::Auto => {
            let tsv = run_tesseract(image, true, &language)?;
            if let Some(table) = smart::extract_table(&tsv) {
                (OcrKind::Table, table, language)
            } else {
                let plain = smart::clean_text(&smart::reconstruct_text_from_tsv(&tsv));
                if plain.is_empty() {
                    return Err(
                        "No text, code, table, or QR code was detected in the selected region."
                            .into(),
                    );
                }
                if smart::looks_like_code(&plain) {
                    (OcrKind::Code, smart::normalize_code(&plain), language)
                } else {
                    (OcrKind::Text, plain, language)
                }
            }
        }
    };
    Ok(Some(result))
}

fn deliver_text(label: &str, text: &str, language: &str) -> Result<(), String> {
    match crate::translate::translate(text, language) {
        Ok(Some(translated)) => {
            let result = match crate::translate::translate_output_mode().as_str() {
                "translation" => translated.clone(),
                "both" => format!("{text}\n\n{translated}"),
                _ => text.to_string(),
            };
            copy_text(&result)?;
            notify(
                "Parker",
                &format!(
                    "{label} ({}) copied. Translated: {}",
                    language_name(language),
                    crate::translate::preview(&translated, 220)
                ),
            );
        }
        Ok(None) => {
            copy_text(text)?;
            notify("Parker", &format!("{label} copied."));
        }
        Err(error) => {
            copy_text(text)?;
            notify(
                "Parker",
                &format!("{label} copied, but translation failed: {error}"),
            );
        }
    }
    Ok(())
}

fn shot() -> Result<(), String> {
    let keep = flag_or("PARKER_KEEP_OCR_CAPTURE", false);
    let image = capture_region_png(keep)?;
    let result = copy_image(&image);
    if !keep {
        let _ = fs::remove_file(&image);
    }
    result?;
    notify("Parker", "Screenshot copied.");
    Ok(())
}

fn detect_language(image: &Path) -> Result<String, String> {
    let configured = setting("PARKER_OCR_LANG", "eng");
    if !crate::translate::lang_auto_enabled() {
        return Ok(configured);
    }
    crate::translate::detect_language("tesseract", image, &configured)
}

fn run_tesseract(image: &Path, tsv: bool, language: &str) -> Result<String, String> {
    let mut command = Command::new("tesseract");
    command
        .arg(image)
        .arg("stdout")
        .arg("-l")
        .arg(language)
        .arg("--psm")
        .arg(setting("PARKER_OCR_PSM", "6"))
        .arg("-c")
        .arg("preserve_interword_spaces=1");
    if tsv {
        command.arg("tsv");
    }
    let output = command
        .output()
        .map_err(|e| format!("Could not run Tesseract: {e}"))?;
    if !output.status.success() {
        return Err(stderr("Tesseract", &output.stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
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

fn captures_dir(keep: bool) -> Result<PathBuf, String> {
    let directory = if keep {
        xdg_user_dir("PICTURES")
            .unwrap_or_else(|| home().join("Pictures"))
            .join("Parker")
    } else {
        env::temp_dir()
    };
    fs::create_dir_all(&directory)
        .map_err(|e| format!("Could not create {}: {e}", directory.display()))?;
    Ok(directory)
}

fn capture_region_png(keep: bool) -> Result<PathBuf, String> {
    let path = captures_dir(keep)?.join(format!("parker-capture-{}.png", stamp()));
    let mut failures: Vec<String> = Vec::new();

    if is_gnome_wayland() {
        match portal_capture(&path) {
            Ok(true) => return Ok(path),
            Ok(false) => {}
            Err(error) => {
                let _ = fs::remove_file(&path);
                return Err(error);
            }
        }
    }

    if !is_gnome_wayland() && available("grim") && available("slurp") {
        let region = select_region("slurp")?;
        let success = Command::new("grim")
            .args(["-g", &region])
            .arg(&path)
            .status();
        if capture_attempt("grim+slurp", success, &path, &mut failures) {
            return Ok(path);
        }
    }
    if !is_gnome_wayland() && available("spectacle") {
        let status = Command::new("spectacle")
            .args(["-r", "-b", "-n", "-o"])
            .arg(&path)
            .status();
        if capture_attempt("spectacle", status, &path, &mut failures) {
            return Ok(path);
        }
    }
    if available("gnome-screenshot") {
        let status = Command::new("gnome-screenshot")
            .args(["-a", "-f"])
            .arg(&path)
            .status();
        if capture_attempt("gnome-screenshot", status, &path, &mut failures) {
            return Ok(path);
        }
    }
    if session() != Session::Wayland && available("maim") {
        let status = Command::new("maim").arg("-s").arg(&path).status();
        if capture_attempt("maim", status, &path, &mut failures) {
            return Ok(path);
        }
    }
    if session() != Session::Wayland && available("scrot") {
        let status = Command::new("scrot")
            .arg("-s")
            .arg("-o")
            .arg(&path)
            .status();
        if capture_attempt("scrot", status, &path, &mut failures) {
            return Ok(path);
        }
    }
    if session() != Session::Wayland && available("import") {
        let status = Command::new("import").arg(&path).status();
        if capture_attempt("import", status, &path, &mut failures) {
            return Ok(path);
        }
    }
    if failures.is_empty() {
        return Err(no_capture_tool_message());
    }
    let _ = fs::remove_file(&path);
    Err(format!(
        "Screen capture failed. Tried: {}",
        failures.join("; ")
    ))
}

fn portal_capture(path: &Path) -> Result<bool, String> {
    let Some(helper) = portal_helper_path() else {
        return Ok(false);
    };
    if !available("python3") {
        return Ok(false);
    }
    let output = Command::new("python3")
        .arg(helper)
        .arg(path)
        .output()
        .map_err(|error| format!("Could not run the GNOME screenshot portal: {error}"))?;
    if output.status.success()
        && path.exists()
        && path
            .metadata()
            .map(|metadata| metadata.len() > 0)
            .unwrap_or(false)
    {
        return Ok(true);
    }
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if detail == "Selection cancelled." {
        return Err(detail);
    }
    if detail.is_empty() {
        Err(format!(
            "GNOME screenshot portal failed (exit {}).",
            output.status.code().unwrap_or(-1)
        ))
    } else {
        Err(format!("GNOME screenshot portal failed: {detail}"))
    }
}

fn portal_helper_path() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(executable) = env::current_exe() {
        if let Some(parent) = executable.parent() {
            candidates.push(parent.join("portal_capture.py"));
            candidates.push(parent.join("../lib/parker/portal_capture.py"));
        }
    }
    candidates
        .push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/linux/portal_capture.py"));
    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn capture_attempt(
    label: &str,
    status: Result<std::process::ExitStatus, std::io::Error>,
    path: &Path,
    failures: &mut Vec<String>,
) -> bool {
    match status {
        Ok(status)
            if status.success()
                && path.exists()
                && path
                    .metadata()
                    .map(|metadata| metadata.len() > 0)
                    .unwrap_or(false) =>
        {
            return true;
        }
        Ok(status) => failures.push(format!("{label}: exit {}", status.code().unwrap_or(-1))),
        Err(error) => failures.push(format!("{label}: {error}")),
    }
    let _ = fs::remove_file(path);
    false
}

fn no_capture_tool_message() -> String {
    if is_gnome_wayland() {
        return "GNOME Wayland did not provide a usable region-capture tool. Install gnome-screenshot or use an X11/KDE session.".into();
    }
    match session() {
        Session::Wayland => {
            "No screen capture tool. Wayland needs grim+slurp (or KDE Spectacle).".into()
        }
        Session::X11 => {
            "No screen capture tool. X11 needs maim, scrot, or ImageMagick import.".into()
        }
        Session::Unknown => {
            "No screen capture tool found. Install grim+slurp (Wayland) or maim/scrot (X11).".into()
        }
    }
}

fn select_region(tool: &str) -> Result<String, String> {
    let output = Command::new(tool)
        .output()
        .map_err(|e| format!("Could not run {tool}: {e}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if detail.is_empty() {
            "Selection cancelled.".into()
        } else {
            format!("Could not select a region with {tool}: {detail}")
        });
    }
    let region = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if region.is_empty() {
        return Err("Selection cancelled.".into());
    }
    Ok(region)
}

fn decode_qr_with_retry(path: &Path) -> Result<Vec<String>, String> {
    let image = ImageReader::open(path)
        .map_err(|e| format!("Could not read capture: {e}"))?
        .decode()
        .map_err(|e| format!("Could not decode capture: {e}"))?
        .to_luma8();
    let payloads = decode_qr_image(&image);
    if !payloads.is_empty() {
        return Ok(payloads);
    }
    for scale in [2u32, 3] {
        let width = image.width().checked_mul(scale).unwrap_or(0);
        let height = image.height().checked_mul(scale).unwrap_or(0);
        if width == 0 || height == 0 || width > 8000 || height > 8000 {
            continue;
        }
        let upscaled = image::imageops::resize(&image, width, height, FilterType::CatmullRom);
        let payloads = decode_qr_image(&upscaled);
        if !payloads.is_empty() {
            return Ok(payloads);
        }
    }
    Ok(Vec::new())
}

fn decode_qr_image(image: &GrayImage) -> Vec<String> {
    let mut prepared = PreparedImage::prepare(image.clone());
    let mut payloads = Vec::new();
    for grid in prepared.detect_grids() {
        if let Ok((_meta, content)) = grid.decode() {
            let content = content.trim().to_string();
            if !content.is_empty() && !payloads.iter().any(|payload| payload == &content) {
                payloads.push(content);
            }
        }
    }
    payloads
}

fn audio_requested() -> bool {
    flag("PARKER_RECORD_AUDIO")
        || env::var("PARKER_AUDIO_DEVICE")
            .ok()
            .is_some_and(|device| !device.trim().is_empty())
}

fn start_recording() -> Result<(), String> {
    require(&["wf-recorder", "ffmpeg"])?;
    require_selection_tool()?;
    if let Some((pid, _)) = recording_state() {
        let pid = pid.parse().map_err(|_| "Corrupt recording state.")?;
        if recorder_alive(pid) && is_wf_recorder(pid) {
            return Err("Recording already active.".into());
        }
        let _ = fs::remove_file(state_path()?);
    }
    let region = select_region("slurp")?;
    let output = recordings_dir()?.join(format!("{}.capture.mkv", stamp()));
    let mut command = Command::new("wf-recorder");
    command.args(["-g", &region, "-f"]).arg(&output);
    if audio_requested() {
        command.arg("--audio");
        if let Ok(device) = env::var("PARKER_AUDIO_DEVICE") {
            if !device.trim().is_empty() {
                command.args(["--audio-device", device.trim()]);
            }
        }
    }
    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not start wf-recorder: {e}. It requires a compositor with wlr-screencopy support."))?;
    thread::sleep(Duration::from_millis(400));
    if !recorder_alive(child.id()) {
        let _ = fs::remove_file(&output);
        return Err("wf-recorder exited immediately. The region may be invalid, or this compositor lacks wlr-screencopy support.".into());
    }
    fs::write(
        state_path()?,
        format!("{}\n{}", child.id(), output.display()),
    )
    .map_err(|e| format!("Could not save recording state: {e}"))?;
    detach_child(child);
    notify("Parker", "Recording started. Run parker stop to finish.");
    Ok(())
}

fn recorder_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn is_wf_recorder(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .is_some_and(|name| process_is_wf_recorder(&name))
}

fn process_is_wf_recorder(name: &str) -> bool {
    name.trim() == "wf-recorder"
}

fn detach_child(child: std::process::Child) {
    std::thread::spawn(move || {
        let mut child = child;
        let _ = child.wait();
    });
}

fn require_selection_tool() -> Result<(), String> {
    if is_gnome_wayland() {
        return Err(
            "Recording needs a compositor with wlr-screencopy support; GNOME Wayland is not supported by wf-recorder."
                .into(),
        );
    }
    if available("slurp") {
        return Ok(());
    }
    Err("Missing slurp. Install it for region selection.".into())
}

fn is_gnome_wayland() -> bool {
    session() == Session::Wayland
        && ["XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP"]
            .iter()
            .filter_map(|name| env::var(name).ok())
            .any(|desktop| desktop_is_gnome(&desktop))
}

fn desktop_is_gnome(desktop: &str) -> bool {
    desktop.to_ascii_lowercase().contains("gnome")
}

fn stop_recording() -> Result<(), String> {
    let (pid, source) = recording_state().ok_or("No active recording.")?;
    let pid = pid.parse::<u32>().map_err(|_| "Corrupt recording state.")?;
    let alive = recorder_alive(pid);
    if alive {
        if !is_wf_recorder(pid) {
            let _ = fs::remove_file(state_path()?);
            return Err(
                "Recording state belongs to another process; Parker did not signal it.".into(),
            );
        }
        let status = Command::new("kill")
            .args(["-INT", &pid.to_string()])
            .status()
            .map_err(|e| format!("Could not stop recorder: {e}"))?;
        if !status.success() {
            return Err("Recorder could not be signalled.".into());
        }
        let deadline = Instant::now() + Duration::from_secs(30);
        while Instant::now() < deadline {
            if !recorder_alive(pid) {
                break;
            }
            thread::sleep(Duration::from_millis(200));
        }
        if recorder_alive(pid) {
            return Err(
                "Recorder did not exit after 30 seconds. It may still be finalizing; try again."
                    .into(),
            );
        }
    } else {
        notify(
            "Parker",
            "Recorder was not running; recovering previous capture.",
        );
    }
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
    notify("Parker", &format!("Finalized {count} recording(s)."));
    Ok(())
}

struct EncodeProfile {
    crf: &'static str,
    max_width: i64,
    max_height: i64,
}

fn encode_profile() -> EncodeProfile {
    let (crf, width, height) = match setting("PARKER_COMPRESSION", "balanced").as_str() {
        "compact" => ("28", 1600, 900),
        "quality" => ("20", 2560, 1440),
        _ => ("24", 1920, 1080),
    };
    EncodeProfile {
        crf,
        max_width: width,
        max_height: height,
    }
}

fn parse_dimension(name: &str, fallback: i64) -> i64 {
    let raw = setting(name, "");
    let raw = raw.trim();
    if raw.is_empty() {
        return fallback;
    }
    raw.parse::<i64>().unwrap_or(fallback).max(0)
}

fn scale_filter(profile: &EncodeProfile) -> Option<String> {
    let max_w = parse_dimension("PARKER_MAX_WIDTH", profile.max_width);
    let max_h = parse_dimension("PARKER_MAX_HEIGHT", profile.max_height);
    if max_w == 0 || max_h == 0 {
        return None;
    }
    Some(format!(
        "scale=w='trunc(min(1,{max_w}/iw,{max_h}/ih)*iw/2)*2':h='trunc(min(1,{max_w}/iw,{max_h}/ih)*ih/2)*2'"
    ))
}

fn validated_crf(profile: &EncodeProfile) -> String {
    setting("PARKER_POST_CRF", "")
        .trim()
        .parse::<u8>()
        .ok()
        .filter(|value| *value <= 51)
        .map(|value| value.to_string())
        .unwrap_or_else(|| profile.crf.to_string())
}

fn ffmpeg_supports(encoder: &str) -> bool {
    static SUPPORTED: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    let supported = SUPPORTED.get_or_init(|| {
        Command::new("ffmpeg")
            .args(["-hide_banner", "-encoders"])
            .output()
            .map(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter_map(|line| line.split_whitespace().nth(1))
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    });
    supported.iter().any(|name| name == encoder)
}

fn encoder_candidates() -> Vec<Encoder> {
    const ALL: [Encoder; 4] = [Encoder::Nvenc, Encoder::Qsv, Encoder::Amf, Encoder::Vaapi];
    match setting("PARKER_VIDEO_ENCODER", "auto").as_str() {
        "nvenc" => vec![Encoder::Nvenc],
        "qsv" => vec![Encoder::Qsv],
        "amf" => vec![Encoder::Amf],
        "vaapi" => vec![Encoder::Vaapi],
        "software" | "x264" | "libx264" => Vec::new(),
        _ => ALL
            .iter()
            .copied()
            .filter(|encoder| encoder.available())
            .collect(),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Encoder {
    Nvenc,
    Qsv,
    Amf,
    Vaapi,
}

impl Encoder {
    fn name(self) -> &'static str {
        match self {
            Encoder::Nvenc => "h264_nvenc",
            Encoder::Qsv => "h264_qsv",
            Encoder::Amf => "h264_amf",
            Encoder::Vaapi => "h264_vaapi",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Encoder::Nvenc => "NVENC",
            Encoder::Qsv => "Quick Sync",
            Encoder::Amf => "AMF",
            Encoder::Vaapi => "VAAPI",
        }
    }

    fn available(self) -> bool {
        ffmpeg_supports(self.name())
    }
}

fn finalize(source: &Path) -> Result<(), String> {
    require(&["ffmpeg"])?;
    let profile = encode_profile();
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
    let crf = validated_crf(&profile);
    let preset_raw = setting("PARKER_POST_PRESET", "");
    let preset: Option<String> = if preset_raw.trim().is_empty() {
        None
    } else {
        Some(preset_raw.trim().to_string())
    };
    let scale = scale_filter(&profile);

    let mut attempts: Vec<(String, Vec<String>)> = Vec::new();
    for encoder in encoder_candidates() {
        attempts.push((
            encoder.label().to_string(),
            build_ffmpeg_args(source, &target, Some(encoder), &crf, None, scale.as_deref()),
        ));
    }
    attempts.push((
        "x264".to_string(),
        build_ffmpeg_args(
            source,
            &target,
            None,
            &crf,
            preset.as_deref(),
            scale.as_deref(),
        ),
    ));

    let log_path = recordings_dir().ok().map(|dir| dir.join("ffmpeg.log"));
    let mut failures: Vec<String> = Vec::new();
    for (label, args) in &attempts {
        let status = Command::new("ffmpeg").args(args).status();
        match status {
            Ok(status) if status.success() => {
                if label != "x264" {
                    notify("Parker", &format!("Encoded with {label}."));
                }
                copy_file(&target)?;
                let _ = fs::remove_file(source);
                return Ok(());
            }
            Ok(status) => failures.push(format!("{label}: exit {}", status.code().unwrap_or(-1))),
            Err(error) => failures.push(format!("{label}: {error}")),
        }
        append_log(
            log_path.as_deref(),
            &format!(
                "{} attempt failed for {}:\n  {:?}\n",
                label,
                source.display(),
                args
            ),
        );
    }
    Err(format!(
        "FFmpeg could not optimize the recording ({}). The .capture.mkv source was kept; see ffmpeg.log.",
        failures.join(", ")
    ))
}

fn build_ffmpeg_args(
    source: &Path,
    target: &Path,
    encoder: Option<Encoder>,
    crf: &str,
    preset: Option<&str>,
    scale: Option<&str>,
) -> Vec<String> {
    let mut args: Vec<String> = ["-y".into(), "-i".into(), source.display().to_string()]
        .into_iter()
        .collect();
    args.extend(["-map".into(), "0:v:0".into(), "-map".into(), "0:a?".into()]);
    args.extend(["-map_metadata".into(), "-1".into()]);
    match encoder {
        Some(Encoder::Nvenc) => {
            args.extend(["-c:v".into(), "h264_nvenc".into(), "-cq".into(), crf.into()]);
            if let Some(preset) = preset {
                args.extend(["-preset".into(), preset.into()]);
            }
            if let Some(scale) = scale {
                args.extend(["-vf".into(), scale.into()]);
            }
            args.extend(["-pix_fmt".into(), "yuv420p".into()]);
        }
        Some(Encoder::Qsv) => {
            args.extend([
                "-c:v".into(),
                "h264_qsv".into(),
                "-global_quality".into(),
                crf.into(),
            ]);
            if let Some(scale) = scale {
                args.extend(["-vf".into(), scale.into()]);
            }
            args.extend(["-pix_fmt".into(), "yuv420p".into()]);
        }
        Some(Encoder::Amf) => {
            args.extend([
                "-c:v".into(),
                "h264_amf".into(),
                "-quality".into(),
                "quality".into(),
                "-rc".into(),
                "cqp".into(),
                "-qp_i".into(),
                crf.into(),
                "-qp_p".into(),
                crf.into(),
            ]);
            if let Some(scale) = scale {
                args.extend(["-vf".into(), scale.into()]);
            }
            args.extend(["-pix_fmt".into(), "yuv420p".into()]);
        }
        Some(Encoder::Vaapi) => {
            args.extend([
                "-init_hw_device".into(),
                "vaapi=va:/dev/dri/renderD128".into(),
                "-filter_hw_device".into(),
                "va".into(),
            ]);
            let mut filter = String::from("format=nv12,hwupload");
            if let Some(scale) = scale {
                filter = format!("{scale},format=nv12,hwupload");
            }
            args.extend(["-vf".into(), filter]);
            args.extend([
                "-c:v".into(),
                "h264_vaapi".into(),
                "-global_quality".into(),
                crf.into(),
            ]);
        }
        None => {
            args.extend(["-c:v".into(), "libx264".into(), "-crf".into(), crf.into()]);
            args.extend(["-preset".into(), preset.unwrap_or("medium").into()]);
            if let Some(scale) = scale {
                args.extend(["-vf".into(), scale.into()]);
            }
            args.extend(["-pix_fmt".into(), "yuv420p".into()]);
        }
    }
    args.extend([
        "-c:a".into(),
        "aac".into(),
        "-movflags".into(),
        "+faststart".into(),
        target.display().to_string(),
    ]);
    args
}

fn append_log(path: Option<&Path>, message: &str) {
    if let Some(path) = path {
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "[{}] {}\n", stamp(), message);
        }
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
    let path = ensure_settings()?;
    Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map_err(|e| format!("Could not open settings: {e}"))?;
    Ok(())
}

#[derive(Clone, Copy)]
struct GuiSettingField {
    key: &'static str,
    label: &'static str,
    fallback: &'static str,
    choices: &'static [&'static str],
    allow_empty: bool,
}

const GUI_NO_CHOICES: &[&str] = &[];
const GUI_BOOL_CHOICES: &[&str] = &["0", "1"];
const GUI_OCR_MODES: &[&str] = &["auto", "text", "code", "table"];
const GUI_TRANSLATION_BACKENDS: &[&str] = &["none", "argos", "libretranslate"];
const GUI_TRANSLATION_OUTPUTS: &[&str] = &["original", "translation", "both"];
const GUI_COMPRESSION: &[&str] = &["compact", "balanced", "quality"];
const GUI_ENCODERS: &[&str] = &["auto", "libx264", "nvenc", "qsv", "amf", "vaapi"];
const GUI_SETTING_FIELDS: &[GuiSettingField] = &[
    GuiSettingField {
        key: "PARKER_OCR_LANG_AUTO",
        label: "Auto-detect OCR language (0=off, 1=on)",
        fallback: "1",
        choices: GUI_BOOL_CHOICES,
        allow_empty: false,
    },
    GuiSettingField {
        key: "PARKER_OCR_LANG",
        label: "OCR language (Tesseract code)",
        fallback: "eng",
        choices: GUI_NO_CHOICES,
        allow_empty: false,
    },
    GuiSettingField {
        key: "PARKER_OCR_MODE",
        label: "OCR mode",
        fallback: "auto",
        choices: GUI_OCR_MODES,
        allow_empty: false,
    },
    GuiSettingField {
        key: "PARKER_TRANSLATE_BACKEND",
        label: "Translation backend",
        fallback: "none",
        choices: GUI_TRANSLATION_BACKENDS,
        allow_empty: false,
    },
    GuiSettingField {
        key: "PARKER_TRANSLATE_TARGET",
        label: "Translation target (ISO code)",
        fallback: "en",
        choices: GUI_NO_CHOICES,
        allow_empty: false,
    },
    GuiSettingField {
        key: "PARKER_TRANSLATE_OUTPUT",
        label: "Translation output",
        fallback: "original",
        choices: GUI_TRANSLATION_OUTPUTS,
        allow_empty: false,
    },
    GuiSettingField {
        key: "PARKER_TRANSLATE_ENDPOINT",
        label: "LibreTranslate endpoint (optional)",
        fallback: "",
        choices: GUI_NO_CHOICES,
        allow_empty: true,
    },
    GuiSettingField {
        key: "PARKER_COMPRESSION",
        label: "Video compression",
        fallback: "balanced",
        choices: GUI_COMPRESSION,
        allow_empty: false,
    },
    GuiSettingField {
        key: "PARKER_VIDEO_ENCODER",
        label: "Video encoder (libx264 = software)",
        fallback: "auto",
        choices: GUI_ENCODERS,
        allow_empty: false,
    },
    GuiSettingField {
        key: "PARKER_KEEP_OCR_CAPTURE",
        label: "Keep OCR captures (0=off, 1=on)",
        fallback: "0",
        choices: GUI_BOOL_CHOICES,
        allow_empty: false,
    },
    GuiSettingField {
        key: "PARKER_QR_AUTO_OPEN",
        label: "Open QR links (0=off, 1=on)",
        fallback: "1",
        choices: GUI_BOOL_CHOICES,
        allow_empty: false,
    },
    GuiSettingField {
        key: "PARKER_RECORD_AUDIO",
        label: "Record audio (0=off, 1=on)",
        fallback: "0",
        choices: GUI_BOOL_CHOICES,
        allow_empty: false,
    },
    GuiSettingField {
        key: "PARKER_AUDIO_DEVICE",
        label: "Audio device (blank = default)",
        fallback: "",
        choices: GUI_NO_CHOICES,
        allow_empty: true,
    },
];

fn edit_settings_gui() -> Result<(), String> {
    let path = ensure_settings()?;
    if !available("zenity") {
        return open_settings();
    }

    loop {
        let current: Vec<String> = GUI_SETTING_FIELDS.iter().map(gui_setting_value).collect();
        let mut command = Command::new("zenity");
        command.args([
            "--forms",
            "--title",
            "Parker settings",
            "--text",
            "Choose common settings. Dropdowns prevent invalid values; full settings remain in settings.env.",
            "--separator",
            "|",
            "--width",
            "760",
            "--height",
            "760",
        ]);
        for (field, value) in GUI_SETTING_FIELDS.iter().zip(&current) {
            if field.choices.is_empty() {
                let display = if value.is_empty() { "not set" } else { value };
                let label = format!("{} (current: {display})", field.label);
                command.args(["--add-entry", &label]);
            } else {
                let values = gui_choice_values(value, field.choices);
                command.args(["--add-combo", field.label, "--combo-values", &values]);
            }
        }
        let output = command
            .output()
            .map_err(|error| format!("Could not open settings: {error}"))?;
        if !output.status.success() {
            return Ok(());
        }
        let values: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .trim_end_matches(['\r', '\n'])
            .split('|')
            .map(str::to_string)
            .collect();
        match gui_settings_updates(&current, &values) {
            Ok(updates) => return save_settings(&path, &updates),
            Err(error) => show_gui_error(&error),
        }
    }
}

fn gui_setting_value(field: &GuiSettingField) -> String {
    raw_setting(field.key).unwrap_or_else(|| field.fallback.to_string())
}

fn raw_setting(name: &str) -> Option<String> {
    if let Ok(value) = env::var(name) {
        return Some(value);
    }
    fs::read_to_string(settings_path().ok()?)
        .ok()?
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key.trim() == name).then(|| value.trim().to_string())
        })
}

fn gui_choice_values(current: &str, choices: &[&str]) -> String {
    let mut values = Vec::with_capacity(choices.len() + 1);
    if !current.is_empty() {
        values.push(current.to_string());
    }
    for choice in choices {
        if !values.iter().any(|value| value == choice) {
            values.push((*choice).to_string());
        }
    }
    values.join("|")
}

fn gui_settings_updates(
    current: &[String],
    values: &[String],
) -> Result<Vec<(&'static str, String)>, String> {
    if values.len() != GUI_SETTING_FIELDS.len() || current.len() != GUI_SETTING_FIELDS.len() {
        return Err("Settings dialog returned incomplete data.".into());
    }
    let mut updates = Vec::with_capacity(GUI_SETTING_FIELDS.len());
    for (index, field) in GUI_SETTING_FIELDS.iter().enumerate() {
        let entered = values[index].trim();
        let value = if entered.is_empty() && !field.allow_empty {
            current[index].clone()
        } else {
            entered.to_string()
        };
        if !field.choices.is_empty() && !field.choices.iter().any(|choice| *choice == value) {
            return Err(format!(
                "{} must be one of: {}.",
                field.label,
                field.choices.join(", ")
            ));
        }
        updates.push((field.key, value));
    }
    let backend = &updates[3].1;
    if backend == "libretranslate" && updates[6].1.trim().is_empty() {
        return Err("Set a LibreTranslate endpoint before enabling that backend.".into());
    }
    Ok(updates)
}

fn ensure_settings() -> Result<PathBuf, String> {
    let path = settings_path()?;
    if !path.exists() {
        fs::write(&path, default_settings())
            .map_err(|error| format!("Could not create settings: {error}"))?;
    }
    Ok(path)
}

fn save_settings(path: &Path, updates: &[(&str, String)]) -> Result<(), String> {
    let original = fs::read_to_string(path).unwrap_or_else(|_| default_settings());
    let output = merge_settings(&original, updates);
    for (key, value) in updates {
        env::set_var(key, value);
    }
    let temporary = path.with_extension("env.tmp");
    fs::write(&temporary, output).map_err(|error| format!("Could not write settings: {error}"))?;
    fs::rename(&temporary, path).map_err(|error| format!("Could not save settings: {error}"))
}

fn merge_settings(original: &str, updates: &[(&str, String)]) -> String {
    let mut seen = vec![false; updates.len()];
    let mut output = String::new();
    for line in original.lines() {
        if let Some((index, (_, value))) = updates.iter().enumerate().find(|(_, (key, _))| {
            line.strip_prefix(key)
                .is_some_and(|tail| tail.starts_with('='))
        }) {
            output.push_str(updates[index].0);
            output.push('=');
            output.push_str(value);
            output.push('\n');
            seen[index] = true;
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }
    for ((key, value), was_seen) in updates.iter().zip(seen) {
        if !was_seen {
            output.push_str(key);
            output.push('=');
            output.push_str(value);
            output.push('\n');
        }
    }
    output
}

fn default_settings() -> String {
    "\
PARKER_OCR_LANG_AUTO=1
PARKER_OCR_LANG=eng
PARKER_OCR_PSM=6
PARKER_OCR_MODE=auto
PARKER_QR_AUTO_OPEN=1
PARKER_KEEP_OCR_CAPTURE=0
PARKER_COMPRESSION=balanced
PARKER_VIDEO_ENCODER=auto
PARKER_POST_CRF=
PARKER_POST_PRESET=
PARKER_MAX_WIDTH=
PARKER_MAX_HEIGHT=
PARKER_RECORD_AUDIO=0
PARKER_TRANSLATE_BACKEND=none
PARKER_TRANSLATE_TARGET=en
PARKER_TRANSLATE_OUTPUT=original
PARKER_TRANSLATE_ENDPOINT=
PARKER_OUTPUT=
"
    .to_string()
}

fn self_update() -> Result<(), String> {
    crate::updater::check_self_update().map(|message| {
        println!("{message}");
        notify("Parker", &message);
    })
}

fn recordings_dir() -> Result<PathBuf, String> {
    let path = env::var_os("PARKER_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            xdg_user_dir("VIDEOS")
                .unwrap_or_else(|| home().join("Videos"))
                .join("Parker")
        });
    fs::create_dir_all(&path).map_err(|e| format!("Could not create {}: {e}", path.display()))?;
    Ok(path)
}

fn xdg_user_dir(name: &str) -> Option<PathBuf> {
    Command::new("xdg-user-dir")
        .arg(name)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()))
        .filter(|path| !path.as_os_str().is_empty())
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
fn load_settings() {
    let Ok(path) = settings_path() else {
        return;
    };
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };
    for (key, value) in settings_entries(&contents) {
        if env::var_os(&key).is_none() {
            env::set_var(key, value);
        }
    }
}

fn settings_entries(contents: &str) -> Vec<(String, String)> {
    contents
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim(), value.trim()))
        .filter(|(key, _)| key.starts_with("PARKER_") && !key.contains(char::is_whitespace))
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}
fn home() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
fn setting(name: &str, fallback: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            fs::read_to_string(settings_path().ok()?)
                .ok()?
                .lines()
                .find_map(|line| line.strip_prefix(&format!("{name}=")).map(str::to_owned))
        })
        .unwrap_or_else(|| fallback.into())
}
fn flag(name: &str) -> bool {
    flag_or(name, false)
}

fn flag_or(name: &str, fallback: bool) -> bool {
    let raw = setting(name, "");
    if raw.is_empty() {
        return fallback;
    }
    matches!(
        raw.as_str(),
        "1" | "true" | "TRUE" | "True" | "yes" | "YES" | "on" | "ON"
    )
}
fn recording_state() -> Option<(String, PathBuf)> {
    let data = fs::read_to_string(state_path().ok()?).ok()?;
    let (pid, path) = data.split_once('\n')?;
    Some((pid.into(), PathBuf::from(path)))
}
fn require(programs: &[&str]) -> Result<(), String> {
    for program in programs {
        if !available(program) {
            return Err(format!("Missing {program}."));
        }
    }
    Ok(())
}
fn available(program: &str) -> bool {
    if let Ok(path_var) = env::var("PATH") {
        for dir in path_var.split(':') {
            if dir.is_empty() {
                continue;
            }
            let candidate = Path::new(dir).join(program);
            if candidate.is_file() {
                return true;
            }
        }
    }
    for dir in ["/usr/bin", "/usr/local/bin", "/bin", "/snap/bin"] {
        if Path::new(dir).join(program).is_file() {
            return true;
        }
    }
    Command::new("sh")
        .args(["-c", &format!("command -v {program}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
        || Command::new("which")
            .arg(program)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
}
fn notify(title: &str, body: &str) {
    if !available("notify-send") {
        return;
    }
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

#[cfg(test)]
mod tests {
    use super::{
        build_ffmpeg_args, desktop_is_gnome, dialog_choice_text, encode_profile, gui_choice_values,
        gui_settings_updates, is_gui_cancellation, merge_settings, parse_dimension,
        process_is_wf_recorder, recording_action, scale_filter, settings_entries, Encoder,
        GUI_SETTING_FIELDS,
    };
    use crate::updater::newer;
    use std::path::Path;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn profiles_match_documentation() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("PARKER_COMPRESSION", "compact");
        assert_eq!(encode_profile().max_width, 1600);
        std::env::set_var("PARKER_COMPRESSION", "quality");
        assert_eq!(encode_profile().crf, "20");
        std::env::remove_var("PARKER_COMPRESSION");
    }

    #[test]
    fn dimensions_can_be_disabled() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("PARKER_MAX_WIDTH", "0");
        std::env::set_var("PARKER_MAX_HEIGHT", "0");
        assert_eq!(parse_dimension("PARKER_MAX_WIDTH", 1920), 0);
        std::env::remove_var("PARKER_MAX_WIDTH");
        std::env::remove_var("PARKER_MAX_HEIGHT");
    }

    #[test]
    fn version_comparison_is_numeric() {
        assert!(newer("0.7.0", "0.6.1"));
        assert!(newer("0.10.0", "0.9.9"));
        assert!(!newer("0.6.1", "0.6.1"));
        assert!(!newer("0.6", "0.6.1"));
    }

    #[test]
    fn scale_filter_respects_zero_limits() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("PARKER_MAX_WIDTH", "1280");
        std::env::set_var("PARKER_MAX_HEIGHT", "720");
        let filter = scale_filter(&encode_profile()).unwrap();
        assert!(filter.contains("1280"));
        std::env::remove_var("PARKER_MAX_WIDTH");
        std::env::remove_var("PARKER_MAX_HEIGHT");
    }

    #[test]
    fn either_zero_dimension_disables_scaling() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("PARKER_MAX_WIDTH", "0");
        std::env::set_var("PARKER_MAX_HEIGHT", "720");
        assert_eq!(scale_filter(&encode_profile()), None);
        std::env::remove_var("PARKER_MAX_WIDTH");
        std::env::remove_var("PARKER_MAX_HEIGHT");
    }

    #[test]
    fn x264_arguments_match_contract() {
        let source = Path::new("/tmp/a.capture.mkv");
        let target = Path::new("/tmp/a.mp4");
        let args = build_ffmpeg_args(
            source,
            target,
            None,
            "24",
            Some("fast"),
            Some("scale=w='trunc(min(1,1920/iw,1080/ih)*iw/2)*2':h='trunc(min(1,1920/iw,1080/ih)*ih/2)*2'"),
        );
        let text = args.join(" ");
        assert!(text.contains("-c:v libx264"));
        assert!(text.contains("-crf 24"));
        assert!(text.contains("-preset fast"));
        assert!(text.contains("-pix_fmt yuv420p"));
        assert!(text.contains("+faststart"));
        assert!(text.contains("-map_metadata -1"));
        assert!(text.contains("scale="));
        assert!(args.last().unwrap() == "/tmp/a.mp4");
    }

    #[test]
    fn hardware_arguments_use_device_paths() {
        let source = Path::new("/tmp/a.capture.mkv");
        let target = Path::new("/tmp/a.mp4");
        let vaapi = build_ffmpeg_args(source, target, Some(Encoder::Vaapi), "22", None, None);
        let text = vaapi.join(" ");
        assert!(text.contains("-init_hw_device vaapi=va:/dev/dri/renderD128"));
        assert!(text.contains("format=nv12,hwupload"));
        assert!(text.contains("h264_vaapi"));

        let nvenc = build_ffmpeg_args(source, target, Some(Encoder::Nvenc), "24", None, None);
        let text = nvenc.join(" ");
        assert!(text.contains("-cq 24"));
        assert!(!text.contains("-preset"));
    }

    #[test]
    fn settings_parser_ignores_comments_and_unknown_keys() {
        let settings = settings_entries(
            "# comment\nPARKER_TRANSLATE_TARGET=fr\nOTHER=value\nPARKER_OCR_MODE = code\n",
        );
        assert_eq!(
            settings,
            vec![
                ("PARKER_TRANSLATE_TARGET".into(), "fr".into()),
                ("PARKER_OCR_MODE".into(), "code".into()),
            ]
        );
    }

    #[test]
    fn recorder_identity_requires_exact_process_name() {
        assert!(process_is_wf_recorder("wf-recorder\n"));
        assert!(!process_is_wf_recorder("bash\n"));
        assert!(!process_is_wf_recorder("wf-recorder-helper\n"));
    }

    #[test]
    fn gui_record_action_tracks_recording_state() {
        assert_eq!(recording_action(false), "Record region");
        assert_eq!(recording_action(true), "Stop recording");
    }

    #[test]
    fn gui_extra_button_works_with_nonzero_status() {
        assert_eq!(
            dialog_choice_text(false, "Screenshot\n", "Smart capture"),
            Some("Screenshot".into())
        );
        assert_eq!(dialog_choice_text(false, "", "Smart capture"), None);
        assert_eq!(
            dialog_choice_text(true, "", "Smart capture"),
            Some("Smart capture".into())
        );
    }

    #[test]
    fn gui_settings_use_current_values_when_text_is_left_blank() {
        let current = vec![
            "1", "eng", "auto", "none", "en", "original", "", "balanced", "auto", "0", "1", "0", "",
        ]
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();
        let mut entered = current.clone();
        entered[1].clear();
        let updates = gui_settings_updates(&current, &entered).unwrap();
        assert_eq!(updates[1].1, "eng");
        assert_eq!(updates.len(), GUI_SETTING_FIELDS.len());
    }

    #[test]
    fn gui_settings_reject_invalid_choices_and_missing_endpoint() {
        let current = vec![
            "1", "eng", "auto", "none", "en", "original", "", "balanced", "auto", "0", "1", "0", "",
        ]
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();
        let mut invalid_choice = current.clone();
        invalid_choice[8] = "bogus".into();
        assert!(gui_settings_updates(&current, &invalid_choice).is_err());

        let mut missing_endpoint = current.clone();
        missing_endpoint[3] = "libretranslate".into();
        assert!(gui_settings_updates(&current, &missing_endpoint).is_err());
    }

    #[test]
    fn gui_choice_values_put_current_value_first() {
        assert_eq!(
            gui_choice_values("code", &["auto", "text", "code"]),
            "code|auto|text"
        );
    }

    #[test]
    fn gui_cancellation_is_not_an_error() {
        assert!(is_gui_cancellation("Selection cancelled."));
        assert!(is_gui_cancellation(" Selection canceled.\n"));
        assert!(!is_gui_cancellation("No screen capture tool found."));
    }

    #[test]
    fn gnome_desktop_detection_is_case_insensitive() {
        assert!(desktop_is_gnome("GNOME"));
        assert!(desktop_is_gnome("ubuntu:GNOME"));
        assert!(!desktop_is_gnome("KDE"));
    }

    #[test]
    fn settings_merge_preserves_comments_and_unknown_keys() {
        let merged = merge_settings(
            "# keep this\nPARKER_OCR_MODE=auto\nEXTRA=value\n",
            &[
                ("PARKER_OCR_MODE", "code".into()),
                ("PARKER_QR_AUTO_OPEN", "0".into()),
            ],
        );
        assert_eq!(
            merged,
            "# keep this\nPARKER_OCR_MODE=code\nEXTRA=value\nPARKER_QR_AUTO_OPEN=0\n"
        );
    }
}
