use crate::selector::ScreenRect;
use crate::win::{
    GetLocalTime, PostMessageW, BELOW_NORMAL_PRIORITY_CLASS, CREATE_NO_WINDOW, HWND, SYSTEMTIME,
    UINT, WM_APP,
};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

pub struct RecordingResult {
    pub path: PathBuf,
    pub encoder: String,
    pub source_bytes: u64,
    pub final_bytes: u64,
}

pub const WM_RECORDING_FINALIZED: UINT = WM_APP + 3;

pub struct Recorder {
    output_directory: PathBuf,
    child: Option<Child>,
    capture_path: Option<PathBuf>,
    final_path: Option<PathBuf>,
    ffmpeg_path: Option<PathBuf>,
    last_error: Option<String>,
    input_handle: Option<crate::input_capture::InputCaptureHandle>,
    ring_seconds: Option<usize>,
}

struct ActiveRecording {
    child: Child,
    capture_path: PathBuf,
    final_path: PathBuf,
    ffmpeg_path: PathBuf,
    ring_seconds: Option<usize>,
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
            last_error: None,
            input_handle: None,
            ring_seconds: None,
        })
    }

    pub fn is_recording(&mut self) -> bool {
        if let Some(child) = self.child.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let capture = self.capture_path.take();
                    let capture_note = capture
                        .as_ref()
                        .map(|path| format!(" The partial capture remains at {}.", path.display()))
                        .unwrap_or_default();
                    self.last_error = Some(format!(
                        "FFmpeg stopped unexpectedly with {status}.{capture_note}"
                    ));
                    self.child = None;
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

    pub fn take_runtime_error(&mut self) -> Option<String> {
        self.last_error.take()
    }

    pub fn start(&mut self, rect: ScreenRect) -> Result<PathBuf, String> {
        self.start_with_ring_seconds(rect, None)
    }

    pub fn start_clip(&mut self, rect: ScreenRect, ring_seconds: usize) -> Result<PathBuf, String> {
        self.start_with_ring_seconds(rect, Some(ring_seconds))
    }

    fn start_with_ring_seconds(
        &mut self,
        rect: ScreenRect,
        ring_seconds_override: Option<usize>,
    ) -> Result<PathBuf, String> {
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
        let ring_seconds = ring_seconds_override.or_else(|| {
            std::env::var("PARKER_RING_SECONDS")
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
        });
        self.ring_seconds = ring_seconds;
        let capture_path = if ring_seconds.is_some() {
            self.output_directory
                .join(format!("{base}.capture.%03d.mkv"))
        } else {
            self.output_directory.join(format!("{base}.capture.mkv"))
        };
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
            .arg("-probesize")
            .arg("32M")
            .arg("-analyzeduration")
            .arg("0")
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
            .arg("desktop");

        if let Ok(audio_device) = std::env::var("PARKER_AUDIO_DEVICE") {
            command
                .arg("-f")
                .arg("dshow")
                .arg("-thread_queue_size")
                .arg("1024")
                .arg("-i")
                .arg(format!("audio={}", audio_device.trim()))
                .arg("-c:a")
                .arg("aac")
                .arg("-b:a")
                .arg("192k");
        } else {
            // Capture default microphone audio when no device is specified
            command
                .arg("-f")
                .arg("dshow")
                .arg("-i")
                .arg("audio=default")
                .arg("-c:a")
                .arg("aac")
                .arg("-b:a")
                .arg("192k");
        }

        command
            .arg("-c:v")
            .arg("libx264")
            .arg("-preset")
            .arg("ultrafast")
            .arg("-tune")
            .arg("zerolatency")
            .arg("-crf")
            .arg("18")
            .arg("-pix_fmt")
            .arg("yuv420p");

        if let Some(s) = ring_seconds {
            // Use ffmpeg segment muxer to keep a rolling buffer of the last N seconds (1s segments)
            command
                .arg("-f")
                .arg("segment")
                .arg("-segment_time")
                .arg("1")
                .arg("-segment_wrap")
                .arg(s.to_string())
                .arg("-reset_timestamps")
                .arg("1")
                .arg("-segment_format")
                .arg("matroska")
                .arg(&capture_path)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::from(log));
        } else {
            command
                .arg("-f")
                .arg("matroska")
                .arg(&capture_path)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::from(log));
        }

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

        // Start input capture for keystrokes/mouse if requested (defaults to enabled)
        if std::env::var("PARKER_CAPTURE_INPUT")
            .ok()
            .is_none_or(|v| v != "0")
        {
            let past = ring_seconds.unwrap_or(30);
            let handle = crate::input_capture::start_input_capture(past);
            self.input_handle = Some(handle);
        }
        Ok(final_path)
    }

    pub fn stop_in_background(
        &mut self,
        notify_window: HWND,
    ) -> Result<mpsc::Receiver<Result<RecordingResult, String>>, String> {
        if self.child.is_none()
            || self.capture_path.is_none()
            || self.final_path.is_none()
            || self.ffmpeg_path.is_none()
        {
            return Err("The active recording state is incomplete.".to_string());
        }

        // If we have input capture data, dump it to a file now
        if let Some(handle) = self.input_handle.take() {
            handle.stop();
            let events = handle.dump_events();
            let input_path = self.input_path_for_capture();
            if let Ok(s) = serde_json::to_string_pretty(&events) {
                let _ = std::fs::write(&input_path, s);
            }
        }

        let active = ActiveRecording {
            child: self.child.take().expect("child checked above"),
            capture_path: self
                .capture_path
                .take()
                .expect("capture path checked above"),
            final_path: self.final_path.take().expect("final path checked above"),
            ffmpeg_path: self.ffmpeg_path.take().expect("FFmpeg path checked above"),
            ring_seconds: self.ring_seconds,
        };
        let output_directory = self.output_directory.clone();
        let notify_window = notify_window as usize;
        let (sender, receiver) = mpsc::sync_channel(1);

        thread::spawn(move || {
            let result = finalize_recording(active, &output_directory);
            let _ = sender.send(result);
            unsafe {
                PostMessageW(notify_window as HWND, WM_RECORDING_FINALIZED, 0, 0);
            }
        });

        Ok(receiver)
    }

    pub fn output_directory(&self) -> &Path {
        &self.output_directory
    }

    fn input_path_for_capture(&self) -> PathBuf {
        let base = self
            .final_path
            .as_ref()
            .and_then(|path| path.file_stem())
            .and_then(|stem| stem.to_str())
            .unwrap_or("parker");
        self.output_directory.join(format!("{base}-input.json"))
    }
}

fn finalize_recording(
    mut active: ActiveRecording,
    output_directory: &Path,
) -> Result<RecordingResult, String> {
    let ActiveRecording {
        child,
        capture_path,
        final_path,
        ffmpeg_path,
        ring_seconds,
    } = &mut active;

    if let Some(mut input) = child.stdin.take() {
        let _ = input.write_all(b"q\n");
        let _ = input.flush();
    }

    wait_for_capture(child)?;

    // If capture_path is a segment pattern (contains '%'), gather recent segments and concat
    if capture_path.to_string_lossy().contains('%') {
        let s = ring_seconds.unwrap_or(30);
        let file_name = capture_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("capture");
        let prefix = file_name.split('%').next().unwrap_or(file_name);

        let mut segs: Vec<_> = std::fs::read_dir(output_directory)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .filter(|d| {
                if let Some(name) = d.file_name().to_str() {
                    name.starts_with(prefix) && name.ends_with(".mkv")
                } else {
                    false
                }
            })
            .map(|d| d.path())
            .collect();
        segs.sort();
        if segs.is_empty() {
            return Err("No capture segments found".to_string());
        }

        let take_n = std::cmp::min(s, segs.len());
        let last = &segs[segs.len() - take_n..];
        let list_txt = output_directory.join("_segments.txt");
        let mut listf = std::fs::File::create(&list_txt).map_err(|e| e.to_string())?;
        for seg in last {
            use std::io::Write;
            writeln!(listf, "file '{}'", seg.display()).ok();
        }

        let tmp = output_directory.join("_clip_temp.mkv");
        let ff = locate_ffmpeg().ok_or_else(|| "FFmpeg not found".to_string())?;
        let status = Command::new(&ff)
            .arg("-y")
            .arg("-f")
            .arg("concat")
            .arg("-safe")
            .arg("0")
            .arg("-i")
            .arg(&list_txt)
            .arg("-c")
            .arg("copy")
            .arg(&tmp)
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("Failed to concat segments".to_string());
        }

        let source_bytes = verify_nonempty(&tmp, "capture")?;
        let encoder = post_process(ffmpeg_path, &tmp, final_path, output_directory)?;
        let final_bytes = verify_nonempty(final_path, "recording")?;
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(&list_txt);

        return Ok(RecordingResult {
            path: final_path.clone(),
            encoder,
            source_bytes,
            final_bytes,
        });
    }

    let source_bytes = verify_nonempty(capture_path, "capture")?;
    let encoder = post_process(ffmpeg_path, capture_path, final_path, output_directory)?;
    let final_bytes = verify_nonempty(final_path, "recording")?;
    let _ = fs::remove_file(capture_path);

    Ok(RecordingResult {
        path: final_path.clone(),
        encoder,
        source_bytes,
        final_bytes,
    })
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

pub fn post_process(
    ffmpeg: &Path,
    capture_path: &Path,
    final_path: &Path,
    output_directory: &Path,
) -> Result<String, String> {
    let log_path = output_directory.join("postprocess.log");
    let config = compression_config()?;
    let mut candidates = encoder_candidates(ffmpeg)?;
    // If the user wants to force GPU encoding, filter out CPU-only encoders
    if std::env::var("PARKER_USE_GPU").ok().is_some_and(|v| {
        let lower = v.to_ascii_lowercase();
        !matches!(lower.as_str(), "0" | "false" | "no" | "off")
    }) {
        candidates
            .retain(|e| matches!(e, EncoderKind::Nvenc | EncoderKind::Qsv | EncoderKind::Amf));
        if candidates.is_empty() {
            // Fallback to software encoder if no GPU encoders are available
            candidates.push(EncoderKind::X264);
        }
    }
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
            .arg("-1");

        if std::env::var("PARKER_AUDIO_DEVICE").is_ok() {
            command.arg("-map").arg("0:a?").arg("-c:a").arg("copy");
        } else {
            command.arg("-an");
        }

        command.arg("-sn").arg("-dn").arg("-vf").arg(&filter);

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

fn append_encoder_arguments(
    command: &mut Command,
    encoder: EncoderKind,
    config: &CompressionConfig,
) {
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
                    "PARKER_VIDEO_ENCODER must be auto, nvenc, qsv, amf, or libx264.".to_string(),
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
        _ => return Err("PARKER_COMPRESSION must be compact, balanced, or quality.".to_string()),
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
