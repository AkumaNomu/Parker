use crate::selector::ScreenRect;
use crate::win::{
    GetLocalTime, BELOW_NORMAL_PRIORITY_CLASS, CREATE_NO_WINDOW, SYSTEMTIME,
};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

pub struct RecordingResult {
    pub path: PathBuf,
    pub encoder: String,
    pub source_bytes: u64,
    pub final_bytes: u64,
}

pub struct Recorder {
    output_directory: PathBuf,
    child: Option<Child>,
    capture_path: Option<PathBuf>,
    final_path: Option<PathBuf>,
    ffmpeg_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct CompressionConfig {
    crf: u8,
    preset: String,
    max_width: u32,
    max_height: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EncoderKind {
    Nvenc,
    Qsv,
    Amf,
    X264,
}

static DETECTED_ENCODERS: OnceLock<Result<Vec<EncoderKind>, String>> = OnceLock::new();

impl EncoderKind {
    fn ffmpeg_name(self) -> &'static str {
        match self {
            Self::Nvenc => "h264_nvenc",
            Self::Qsv => "h264_qsv",
            Self::Amf => "h264_amf",
            Self::X264 => "libx264",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Nvenc => "NVIDIA NVENC",
            Self::Qsv => "Intel Quick Sync",
            Self::Amf => "AMD AMF",
            Self::X264 => "x264 software encoding",
        }
    }
}

impl Recorder {
    pub fn new() -> Result<Self, String> {
        let output_directory = output_directory();
        fs::create_dir_all(&output_directory).map_err(|error| {
            format!(
                "Could not create the recording directory {}: {error}",
                output_directory.display()
            )
        })?;

        Ok(Self {
            output_directory,
            child: None,
            capture_path: None,
            final_path: None,
            ffmpeg_path: None,
        })
    }

    pub fn is_recording(&mut self) -> bool {
        if let Some(child) = self.child.as_mut() {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.child = None;
                    self.capture_path = None;
                    self.final_path = None;
                    self.ffmpeg_path = None;
                    false
                }
                Ok(None) => true,
                Err(_) => true,
            }
        } else {
            false
        }
    }

    pub fn start(&mut self, rect: ScreenRect) -> Result<PathBuf, String> {
        if self.is_recording() {
            return Err("A recording is already active.".to_string());
        }

        let width = rect.width - rect.width.rem_euclid(2);
        let height = rect.height - rect.height.rem_euclid(2);
        if width < 4 || height < 4 {
            return Err("The selected recording region is too small.".to_string());
        }

        let ffmpeg = locate_ffmpeg().ok_or_else(|| {
            "FFmpeg was not found. Run setup.cmd/install.ps1, place ffmpeg.exe beside parker.exe, or set PARKER_FFMPEG."
                .to_string()
        })?;
        let base = timestamped_basename();
        let capture_path = self.output_directory.join(format!("{base}.capture.mkv"));
        let final_path = self.output_directory.join(format!("{base}.mp4"));
        let log_path = self.output_directory.join("ffmpeg.log");
        let log = open_log(&log_path)?;

        let mut command = Command::new(&ffmpeg);
        command
            .creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS)
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("warning")
            .arg("-y")
            .arg("-thread_queue_size")
            .arg("1024")
            .arg("-f")
            .arg("gdigrab")
            .arg("-framerate")
            .arg(recording_fps().to_string())
            .arg("-offset_x")
            .arg(rect.x.to_string())
            .arg("-offset_y")
            .arg(rect.y.to_string())
            .arg("-video_size")
            .arg(format!("{width}x{height}"))
            .arg("-draw_mouse")
            .arg("0")
            .arg("-i")
            .arg("desktop")
            .arg("-an")
            .arg("-c:v")
            .arg("libx264")
            .arg("-preset")
            .arg("ultrafast")
            .arg("-tune")
            .arg("zerolatency")
            .arg("-crf")
            .arg("18")
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg("-f")
            .arg("matroska")
            .arg(&capture_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::from(log));

        let mut child = command
            .spawn()
            .map_err(|error| format!("Could not start FFmpeg at {}: {error}", ffmpeg.display()))?;

        thread::sleep(Duration::from_millis(350));
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("Could not inspect FFmpeg: {error}"))?
        {
            return Err(format!(
                "FFmpeg exited immediately with {status}. Details are in {}.",
                log_path.display()
            ));
        }

        self.capture_path = Some(capture_path);
        self.final_path = Some(final_path.clone());
        self.ffmpeg_path = Some(ffmpeg);
        self.child = Some(child);
        Ok(final_path)
    }

    pub fn stop(&mut self) -> Result<RecordingResult, String> {
        let mut child = self
            .child
            .take()
            .ok_or_else(|| "No recording is active.".to_string())?;
        let capture_path = self
            .capture_path
            .take()
            .ok_or_else(|| "The active recording has no capture path.".to_string())?;
        let final_path = self
            .final_path
            .take()
            .ok_or_else(|| "The active recording has no final output path.".to_string())?;
        let ffmpeg = self
            .ffmpeg_path
            .take()
            .ok_or_else(|| "The active recording has no FFmpeg path.".to_string())?;

        if let Some(mut input) = child.stdin.take() {
            let _ = input.write_all(b"q\n");
            let _ = input.flush();
        }

        wait_for_capture(&mut child)?;
        let source_bytes = verify_nonempty(&capture_path, "capture")?;
        let encoder = post_process(&ffmpeg, &capture_path, &final_path, &self.output_directory)?;
        let final_bytes = verify_nonempty(&final_path, "recording")?;
        let _ = fs::remove_file(&capture_path);

        Ok(RecordingResult {
            path: final_path,
            encoder,
            source_bytes,
            final_bytes,
        })
    }

    pub fn output_directory(&self) -> &Path {
        &self.output_directory
    }
}

fn wait_for_capture(child: &mut Child) -> Result<(), String> {
    for _ in 0..200 {
        match child.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    Ok(())
                } else {
                    Err(format!("FFmpeg capture exited with {status}."))
                };
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(error) => return Err(format!("Could not wait for FFmpeg: {error}")),
        }
    }

    let _ = child.kill();
    let _ = child.wait();
    Err("FFmpeg did not finalize the capture cleanly.".to_string())
}

fn post_process(
    ffmpeg: &Path,
    capture_path: &Path,
    final_path: &Path,
    output_directory: &Path,
) -> Result<String, String> {
    let log_path = output_directory.join("postprocess.log");
    let config = compression_config()?;
    let candidates = encoder_candidates(ffmpeg)?;
    let filter = scale_filter(config.max_width, config.max_height);

    for encoder in candidates {
        let _ = fs::remove_file(final_path);
        let mut log = open_log(&log_path)?;
        let _ = writeln!(
            log,
            "\n--- Parker post-process: {} (CRF/CQ {}, preset {}) ---",
            encoder.display_name(),
            config.crf,
            config.preset
        );

        let mut command = Command::new(ffmpeg);
        command
            .creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS)
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("warning")
            .arg("-y")
            .arg("-i")
            .arg(capture_path)
            .arg("-map")
            .arg("0:v:0")
            .arg("-map_metadata")
            .arg("-1")
            .arg("-an")
            .arg("-sn")
            .arg("-dn")
            .arg("-vf")
            .arg(&filter);

        append_encoder_arguments(&mut command, encoder, &config);

        let status = command
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg("-tag:v")
            .arg("avc1")
            .arg("-movflags")
            .arg("+faststart")
            .arg("-threads")
            .arg("0")
            .arg(final_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(log))
            .status();

        match status {
            Ok(status) if status.success() && verify_nonempty(final_path, "recording").is_ok() => {
                return Ok(encoder.display_name().to_string());
            }
            Ok(_) | Err(_) => {
                let _ = fs::remove_file(final_path);
            }
        }
    }

    Err(format!(
        "Automatic video post-processing failed with every available encoder. The original capture remains at {}. Details are in {}.",
        capture_path.display(),
        log_path.display()
    ))
}

fn append_encoder_arguments(command: &mut Command, encoder: EncoderKind, config: &CompressionConfig) {
    match encoder {
        EncoderKind::Nvenc => {
            command
                .arg("-c:v")
                .arg(encoder.ffmpeg_name())
                .arg("-preset")
                .arg("p5")
                .arg("-tune")
                .arg("hq")
                .arg("-rc")
                .arg("vbr")
                .arg("-cq")
                .arg(config.crf.to_string())
                .arg("-b:v")
                .arg("0");
        }
        EncoderKind::Qsv => {
            command
                .arg("-c:v")
                .arg(encoder.ffmpeg_name())
                .arg("-preset")
                .arg("medium")
                .arg("-global_quality")
                .arg(config.crf.to_string());
        }
        EncoderKind::Amf => {
            command
                .arg("-c:v")
                .arg(encoder.ffmpeg_name())
                .arg("-usage")
                .arg("transcoding")
                .arg("-quality")
                .arg("balanced")
                .arg("-rc")
                .arg("cqp")
                .arg("-qp_i")
                .arg(config.crf.to_string())
                .arg("-qp_p")
                .arg(config.crf.to_string())
                .arg("-qp_b")
                .arg(config.crf.to_string());
        }
        EncoderKind::X264 => {
            command
                .arg("-c:v")
                .arg(encoder.ffmpeg_name())
                .arg("-preset")
                .arg(&config.preset)
                .arg("-crf")
                .arg(config.crf.to_string())
                .arg("-tune")
                .arg("animation")
                .arg("-profile:v")
                .arg("high");
        }
    }
}

fn encoder_candidates(ffmpeg: &Path) -> Result<Vec<EncoderKind>, String> {
    let requested = env::var("PARKER_VIDEO_ENCODER")
        .unwrap_or_else(|_| "auto".to_string())
        .trim()
        .to_ascii_lowercase();

    if requested != "auto" {
        let explicit = match requested.as_str() {
            "nvenc" | "h264_nvenc" => EncoderKind::Nvenc,
            "qsv" | "h264_qsv" => EncoderKind::Qsv,
            "amf" | "h264_amf" => EncoderKind::Amf,
            "software" | "x264" | "libx264" => EncoderKind::X264,
            _ => {
                return Err(
                    "PARKER_VIDEO_ENCODER must be auto, nvenc, qsv, amf, or libx264."
                        .to_string(),
                )
            }
        };
        let mut candidates = vec![explicit];
        if explicit != EncoderKind::X264 {
            candidates.push(EncoderKind::X264);
        }
        return Ok(candidates);
    }

    DETECTED_ENCODERS
        .get_or_init(|| detect_available_encoders(ffmpeg))
        .clone()
}

fn detect_available_encoders(ffmpeg: &Path) -> Result<Vec<EncoderKind>, String> {
    let output = Command::new(ffmpeg)
        .creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS)
        .arg("-hide_banner")
        .arg("-encoders")
        .output()
        .map_err(|error| format!("Could not inspect FFmpeg encoders: {error}"))?;
    let encoders = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let mut candidates = Vec::new();
    for encoder in [EncoderKind::Nvenc, EncoderKind::Qsv, EncoderKind::Amf] {
        if encoders.contains(encoder.ffmpeg_name()) {
            candidates.push(encoder);
        }
    }
    candidates.push(EncoderKind::X264);
    Ok(candidates)
}

fn compression_config() -> Result<CompressionConfig, String> {
    let profile = env::var("PARKER_COMPRESSION")
        .unwrap_or_else(|_| "balanced".to_string())
        .trim()
        .to_ascii_lowercase();
    let (default_crf, default_preset, default_width, default_height) = match profile.as_str() {
        "compact" => (28, "slow", 1600, 900),
        "balanced" => (24, "medium", 1920, 1080),
        "quality" => (20, "slow", 2560, 1440),
        _ => {
            return Err(
                "PARKER_COMPRESSION must be compact, balanced, or quality.".to_string(),
            )
        }
    };

    let crf = env::var("PARKER_POST_CRF")
        .ok()
        .map(|value| {
            value
                .parse::<u8>()
                .map_err(|_| "PARKER_POST_CRF must be an integer from 0 to 51.".to_string())
        })
        .transpose()?
        .unwrap_or(default_crf);
    if crf > 51 {
        return Err("PARKER_POST_CRF must be an integer from 0 to 51.".to_string());
    }

    let preset = env::var("PARKER_POST_PRESET").unwrap_or_else(|_| default_preset.to_string());
    let allowed = [
        "ultrafast", "superfast", "veryfast", "faster", "fast", "medium", "slow",
        "slower", "veryslow",
    ];
    if !allowed.contains(&preset.as_str()) {
        return Err("PARKER_POST_PRESET is not a supported x264 preset.".to_string());
    }

    Ok(CompressionConfig {
        crf,
        preset,
        max_width: configured_dimension("PARKER_MAX_WIDTH", default_width)?,
        max_height: configured_dimension("PARKER_MAX_HEIGHT", default_height)?,
    })
}

fn configured_dimension(name: &str, default: u32) -> Result<u32, String> {
    let value = env::var(name)
        .ok()
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|_| format!("{name} must be a positive integer or 0."))
        })
        .transpose()?
        .unwrap_or(default);
    Ok(value)
}

fn scale_filter(max_width: u32, max_height: u32) -> String {
    if max_width == 0 || max_height == 0 {
        "scale=trunc(iw/2)*2:trunc(ih/2)*2:flags=lanczos".to_string()
    } else {
        format!(
            "scale=w='min({max_width},iw)':h='min({max_height},ih)':force_original_aspect_ratio=decrease:force_divisible_by=2:flags=lanczos"
        )
    }
}

fn verify_nonempty(path: &Path, label: &str) -> Result<u64, String> {
    let metadata = fs::metadata(path).map_err(|error| {
        format!(
            "The {label} file {} could not be read: {error}",
            path.display()
        )
    })?;
    if metadata.len() == 0 {
        Err(format!("The {label} file is empty."))
    } else {
        Ok(metadata.len())
    }
}

fn open_log(path: &Path) -> Result<std::fs::File, String> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("Could not open {}: {error}", path.display()))
}

fn recording_fps() -> u32 {
    env::var("PARKER_RECORD_FPS")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| (1..=120).contains(value))
        .unwrap_or(30)
}

fn locate_ffmpeg() -> Option<PathBuf> {
    if let Some(path) = env::var_os("PARKER_FFMPEG") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    if let Ok(executable) = env::current_exe() {
        if let Some(parent) = executable.parent() {
            let bundled = parent.join("ffmpeg.exe");
            if bundled.is_file() {
                return Some(bundled);
            }
        }
    }

    let output = Command::new("where.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .arg("ffmpeg.exe")
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

fn output_directory() -> PathBuf {
    if let Some(path) = env::var_os("PARKER_OUTPUT") {
        return PathBuf::from(path);
    }

    if let Some(profile) = env::var_os("USERPROFILE") {
        return PathBuf::from(profile).join("Videos").join("Parker");
    }

    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("Parker")
}

fn timestamped_basename() -> String {
    let mut time = SYSTEMTIME::default();
    unsafe {
        GetLocalTime(&mut time);
    }

    format!(
        "parker-{:04}{:02}{:02}-{:02}{:02}{:02}-{:03}",
        time.wYear,
        time.wMonth,
        time.wDay,
        time.wHour,
        time.wMinute,
        time.wSecond,
        time.wMilliseconds
    )
}
