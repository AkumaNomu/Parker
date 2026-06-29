use crate::recorder::RecordingResult;
use crate::scroll_capture::ScrollCaptureResult;
use crate::toast;
use crate::win::Beep;

pub fn ready() {
    toast::show(
        "Parker is ready. Ctrl+Shift+F8 captures; Ctrl+Shift+F9 records; Ctrl+Shift+F7 clips; Ctrl+Shift+F11 scroll-captures.",
    );
    unsafe {
        Beep(740, 70);
        Beep(990, 90);
    }
}

pub fn selection_started() {
    unsafe {
        Beep(660, 65);
    }
}

pub fn recording_started() {
    toast::show(
        "Recording started. Drag the timer anywhere; click its stop button or press Ctrl+Shift+F9 to finish.",
    );
    start_sound();
}

pub fn clip_recording_started(seconds: usize) {
    toast::show(format!(
        "Clip recording started. Parker will keep the last {seconds} seconds; press Ctrl+Shift+F7 or F9 to stop."
    ));
    start_sound();
}

pub fn scroll_capture_started() {
    toast::show(
        "Scroll capture started. Scroll the page, then press Ctrl+Shift+F11 again to stitch it.",
    );
    start_sound();
}

pub fn file_copied(result: &RecordingResult) {
    toast::show(format!(
        "Recording optimized with {} ({}{}) and copied as an MP4 file.",
        result.encoder,
        format_bytes(result.final_bytes),
        size_reduction(result.source_bytes, result.final_bytes)
    ));
    saved_sound();
}

pub fn scroll_capture_saved(result: &ScrollCaptureResult) {
    toast::show(format!(
        "Scroll capture stitched from {} frames ({}{}) and copied as an image file.",
        result.frame_count,
        format_bytes(result.final_bytes),
        size_reduction(result.source_bytes, result.final_bytes)
    ));
    saved_sound();
}

pub fn text_copied() {
    toast::show("Text recognized and copied.");
    success();
}

pub fn code_copied() {
    toast::show("Code detected and copied with its line structure preserved.");
    success();
}

pub fn table_copied() {
    toast::show("Table detected and copied as tab-separated values.");
    success();
}

pub fn qr_opened() {
    toast::show("QR link opened and copied to the clipboard.");
    success();
}

pub fn success() {
    unsafe {
        Beep(1047, 70);
        Beep(1319, 70);
        Beep(1568, 110);
    }
}

pub fn cancelled() {
    toast::show("Capture cancelled.");
    unsafe {
        Beep(440, 80);
    }
}

pub fn error() {
    unsafe {
        Beep(260, 220);
    }
}

fn start_sound() {
    unsafe {
        Beep(880, 80);
        Beep(1175, 100);
    }
}

fn saved_sound() {
    unsafe {
        Beep(1175, 75);
        Beep(1568, 130);
    }
}

fn size_reduction(source_bytes: u64, final_bytes: u64) -> String {
    if source_bytes > final_bytes && source_bytes > 0 {
        format!(
            ", {}% smaller",
            100 - (final_bytes.saturating_mul(100) / source_bytes)
        )
    } else {
        String::new()
    }
}

fn format_bytes(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / MB)
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}
