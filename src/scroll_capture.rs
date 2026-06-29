use crate::screenshot::capture_region_to_bmp;
use crate::selector::ScreenRect;
use crate::win::{HWND, UINT, WM_APP};
use image::RgbaImage;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

pub const WM_SCROLL_CAPTURE_FINALIZED: UINT = WM_APP + 4;

pub struct ScrollCaptureResult {
    pub path: PathBuf,
    pub frame_count: usize,
    pub source_bytes: u64,
    pub final_bytes: u64,
}

pub struct ScrollCapture {
    output_directory: PathBuf,
    active: Option<ActiveScrollCapture>,
}

struct ActiveScrollCapture {
    stop_flag: Arc<AtomicBool>,
    worker: thread::JoinHandle<Result<usize, String>>,
    frame_dir: PathBuf,
    final_path: PathBuf,
}

impl ScrollCapture {
    pub fn new(output_directory: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&output_directory).map_err(|error| {
            format!(
                "Could not create the scroll capture directory {}: {error}",
                output_directory.display()
            )
        })?;

        Ok(Self {
            output_directory,
            active: None,
        })
    }

    pub fn is_capturing(&self) -> bool {
        self.active.is_some()
    }

    pub fn start(&mut self, rect: ScreenRect) -> Result<PathBuf, String> {
        if self.active.is_some() {
            return Err("A scroll capture is already active.".to_string());
        }

        let base = timestamped_basename();
        let frame_dir = temp_capture_directory(&base);
        fs::create_dir_all(&frame_dir).map_err(|error| {
            format!(
                "Could not create the temporary scroll capture directory {}: {error}",
                frame_dir.display()
            )
        })?;

        let final_path = self.output_directory.join(format!("{base}.scroll.png"));
        let stop_flag = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop_flag);
        let worker_dir = frame_dir.clone();
        let worker = thread::spawn(move || capture_frames(rect, &worker_dir, worker_stop));

        self.active = Some(ActiveScrollCapture {
            stop_flag,
            worker,
            frame_dir,
            final_path: final_path.clone(),
        });

        Ok(final_path)
    }

    pub fn stop_in_background(
        &mut self,
        notify_window: HWND,
    ) -> Result<mpsc::Receiver<Result<ScrollCaptureResult, String>>, String> {
        let active = self
            .active
            .take()
            .ok_or_else(|| "No active scroll capture is running.".to_string())?;

        active.stop_flag.store(true, Ordering::Relaxed);
        let notify_window = notify_window as usize;
        let (sender, receiver) = mpsc::sync_channel(1);

        thread::spawn(move || {
            let result = finalize_scroll_capture(active);
            let _ = sender.send(result);
            unsafe {
                crate::win::PostMessageW(notify_window as HWND, WM_SCROLL_CAPTURE_FINALIZED, 0, 0);
            }
        });

        Ok(receiver)
    }
}

fn capture_frames(
    rect: ScreenRect,
    frame_dir: &Path,
    stop_flag: Arc<AtomicBool>,
) -> Result<usize, String> {
    let mut count = 0usize;
    loop {
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }

        let frame_path = frame_dir.join(format!("frame-{count:05}.bmp"));
        capture_region_to_bmp(rect, &frame_path)?;
        count += 1;

        for _ in 0..8 {
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    Ok(count)
}

fn finalize_scroll_capture(active: ActiveScrollCapture) -> Result<ScrollCaptureResult, String> {
    let ActiveScrollCapture {
        stop_flag: _,
        worker,
        frame_dir,
        final_path,
    } = active;

    let frame_count = match worker.join() {
        Ok(result) => result?,
        Err(_) => {
            let _ = fs::remove_dir_all(&frame_dir);
            return Err("The scroll capture worker stopped unexpectedly.".to_string());
        }
    };

    if frame_count == 0 {
        let _ = fs::remove_dir_all(&frame_dir);
        return Err("No scroll capture frames were recorded.".to_string());
    }

    let frame_paths = collect_frame_paths(&frame_dir)?;
    if frame_paths.is_empty() {
        let _ = fs::remove_dir_all(&frame_dir);
        return Err("No scroll capture frames were found.".to_string());
    }

    let source_bytes = frame_paths.iter().try_fold(0u64, |total, path| {
        fs::metadata(path)
            .map(|meta| total.saturating_add(meta.len()))
            .map_err(|error| format!("Could not inspect {}: {error}", path.display()))
    })?;

    stitch_scroll_frames(&frame_paths, &final_path)?;
    let final_bytes = fs::metadata(&final_path)
        .map_err(|error| format!("Could not inspect {}: {error}", final_path.display()))?
        .len();

    let _ = fs::remove_dir_all(&frame_dir);

    Ok(ScrollCaptureResult {
        path: final_path,
        frame_count,
        source_bytes,
        final_bytes,
    })
}

fn collect_frame_paths(frame_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths: Vec<PathBuf> = fs::read_dir(frame_dir)
        .map_err(|error| format!("Could not read {}: {error}", frame_dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("bmp"))
        .collect();
    paths.sort();
    Ok(paths)
}

fn stitch_scroll_frames(frame_paths: &[PathBuf], output_path: &Path) -> Result<(), String> {
    let first = load_frame(&frame_paths[0])?;
    let width = first.width();
    let mut shifts = vec![0u32];
    let mut total_height = first.height();
    let mut previous = first;

    for path in &frame_paths[1..] {
        let next = load_frame(path)?;
        if next.width() != width {
            return Err("Scroll capture frames changed width unexpectedly.".to_string());
        }
        let shift = detect_vertical_shift(&previous, &next);
        total_height = total_height.saturating_add(next.height().saturating_sub(shift));
        shifts.push(shift);
        previous = next;
    }

    let mut canvas = RgbaImage::new(width, total_height);
    let mut current_y = 0u32;

    for (index, path) in frame_paths.iter().enumerate() {
        let frame = load_frame(path)?;
        let shift = shifts[index];
        if shift >= frame.height() {
            continue;
        }
        copy_frame_rows(&frame, shift, &mut canvas, current_y);
        current_y = current_y.saturating_add(frame.height().saturating_sub(shift));
    }

    canvas
        .save(output_path)
        .map_err(|error| format!("Could not write {}: {error}", output_path.display()))
}

fn load_frame(path: &Path) -> Result<RgbaImage, String> {
    Ok(image::open(path)
        .map_err(|error| format!("Could not open {}: {error}", path.display()))?
        .to_rgba8())
}

fn detect_vertical_shift(previous: &RgbaImage, next: &RgbaImage) -> u32 {
    let height = previous.height().min(next.height());
    if height == 0 {
        return 0;
    }

    let probe = (height / 2)
        .clamp(1, 32)
        .min(height.saturating_sub(1).max(1));
    let max_shift = height.saturating_sub(probe);
    let step = if max_shift > 256 { 4 } else { 1 };
    let mut best_shift = 0u32;
    let mut best_score = u64::MAX;

    for shift in (0..=max_shift).step_by(step) {
        let score = band_score(previous, next, shift, probe);
        if score < best_score {
            best_score = score;
            best_shift = shift;
        }
    }

    best_shift
}

fn band_score(previous: &RgbaImage, next: &RgbaImage, shift: u32, probe: u32) -> u64 {
    let width = previous.width().min(next.width());
    let sample_step = (width / 48).max(1);
    let mut score = 0u64;

    for row in 0..probe {
        let prev_y = shift + row;
        let next_y = row;
        for x in (0..width).step_by(sample_step as usize) {
            let a = previous.get_pixel(x, prev_y).0;
            let b = next.get_pixel(x, next_y).0;
            score += a
                .iter()
                .zip(b.iter())
                .map(|(left, right)| left.abs_diff(*right) as u64)
                .sum::<u64>();
        }
    }

    score
}

fn copy_frame_rows(frame: &RgbaImage, skip_rows: u32, canvas: &mut RgbaImage, dest_y: u32) {
    let width = frame.width() as usize;
    let row_bytes = width * 4;
    let source = frame.as_raw();
    let destination = canvas.as_mut();
    for (write_row, row) in (dest_y as usize..).zip(skip_rows as usize..frame.height() as usize) {
        let source_offset = row * row_bytes;
        let dest_offset = write_row * row_bytes;
        destination[dest_offset..dest_offset + row_bytes]
            .copy_from_slice(&source[source_offset..source_offset + row_bytes]);
    }
}

fn temp_capture_directory(base: &str) -> PathBuf {
    std::env::temp_dir()
        .join("Parker")
        .join(format!("{base}.scroll-frames"))
}

fn timestamped_basename() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("parker-{millis}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    fn frame_from_rows(rows: &[u8]) -> RgbaImage {
        let width = 4;
        let height = rows.len() as u32;
        let mut image = ImageBuffer::new(width, height);
        for (y, value) in rows.iter().enumerate() {
            for x in 0..width {
                image.put_pixel(x, y as u32, Rgba([*value, *value, *value, 255]));
            }
        }
        image
    }

    #[test]
    fn detect_shift_finds_overlap() {
        let previous = frame_from_rows(&[10, 20, 30, 40, 50, 60, 70, 80]);
        let next = frame_from_rows(&[40, 50, 60, 70, 80, 90, 100, 110]);
        assert_eq!(detect_vertical_shift(&previous, &next), 3);
    }

    #[test]
    fn stitch_rows_appends_new_content() {
        let dir = std::env::temp_dir().join("parker-scroll-test");
        let _ = fs::create_dir_all(&dir);
        let first = dir.join("frame-00001.bmp");
        let second = dir.join("frame-00002.bmp");
        frame_from_rows(&[1, 2, 3, 4, 5, 6])
            .save(&first)
            .expect("save first");
        frame_from_rows(&[4, 5, 6, 7, 8, 9])
            .save(&second)
            .expect("save second");

        let out = dir.join("out.png");
        stitch_scroll_frames(&[first, second], &out).expect("stitch");
        let combined = image::open(&out).expect("open stitched").to_rgba8();
        assert_eq!(combined.height(), 9);
    }
}
