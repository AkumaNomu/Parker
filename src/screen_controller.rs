use crate::win::*;
use std::ffi::c_void;
use std::mem::size_of;
use std::path::Path;
use std::ptr::null_mut;

// ── Screen image (in-memory pixel buffer) ────────────────────────────

#[derive(Clone)]
pub struct ScreenImage {
    pub width: i32,
    pub height: i32,
    pub pixels: Vec<u8>, // BGRA 32bpp, row-major
}

impl ScreenImage {
    #[allow(dead_code)]
    pub fn pixel(&self, x: i32, y: i32) -> Option<(u8, u8, u8)> {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            return None;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        Some((self.pixels[idx + 2], self.pixels[idx + 1], self.pixels[idx])) // R, G, B
    }

    #[allow(dead_code)]
    pub fn color_at(&self, x: i32, y: i32) -> Option<u32> {
        self.pixel(x, y).map(|(r, g, b)| (r as u32) << 16 | (g as u32) << 8 | b as u32)
    }

    pub fn save_bmp(&self, path: &Path) -> Result<(), String> {
        crate::screenshot::write_bmp(path, self.width, self.height, &self.pixels)
    }
}

// ── Capture ───────────────────────────────────────────────────────────

pub fn capture_screen() -> Result<ScreenImage, String> {
    let x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let w = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let h = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    capture_region(x, y, if w > 0 { w } else { unsafe { GetSystemMetrics(SM_CXSCREEN) } },
                             if h > 0 { h } else { unsafe { GetSystemMetrics(SM_CYSCREEN) } })
}

pub fn capture_primary() -> Result<ScreenImage, String> {
    let w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    capture_region(0, 0, w, h)
}

#[allow(dead_code)]
pub fn capture_monitor(index: u32) -> Result<ScreenImage, String> {
    let monitors = enumerate_monitors();
    let i = index as usize;
    if i >= monitors.len() {
        return Err(format!("Monitor {index} not found (have {})", monitors.len()));
    }
    let m = &monitors[i];
    capture_region(m.rect.left, m.rect.top,
                   m.rect.right - m.rect.left,
                   m.rect.bottom - m.rect.top)
}

pub fn capture_region(x: i32, y: i32, w: i32, h: i32) -> Result<ScreenImage, String> {
    if w <= 0 || h <= 0 {
        return Err("Capture region must have positive dimensions".to_string());
    }
    unsafe {
        let screen_dc = GetDC(null_mut());
        if screen_dc.is_null() {
            return Err("Could not get screen DC".to_string());
        }
        let memory_dc = CreateCompatibleDC(screen_dc);
        if memory_dc.is_null() {
            ReleaseDC(null_mut(), screen_dc);
            return Err("Could not create memory DC".to_string());
        }
        let bitmap = CreateCompatibleBitmap(screen_dc, w, h);
        if bitmap.is_null() {
            DeleteDC(memory_dc);
            ReleaseDC(null_mut(), screen_dc);
            return Err("Could not create compatible bitmap".to_string());
        }
        let old = SelectObject(memory_dc, bitmap as HGDIOBJ);
        let success = BitBlt(memory_dc, 0, 0, w, h, screen_dc, x, y, SRCCOPY | CAPTUREBLT);
        if success == 0 {
            SelectObject(memory_dc, old);
            DeleteObject(bitmap as HGDIOBJ);
            DeleteDC(memory_dc);
            ReleaseDC(null_mut(), screen_dc);
            return Err("BitBlt failed".to_string());
        }
        let row_size = w * 4;
        let pixel_size = (row_size * h) as usize;
        let mut pixels = vec![0u8; pixel_size];
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as DWORD,
                biWidth: w,
                biHeight: -h, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD { rgbBlue: 0, rgbGreen: 0, rgbRed: 0, rgbReserved: 0 }; 1],
        };
        let result = GetDIBits(
            memory_dc, bitmap, 0, h as UINT,
            pixels.as_mut_ptr() as *mut c_void,
            &mut info, DIB_RGB_COLORS,
        );
        SelectObject(memory_dc, old);
        DeleteObject(bitmap as HGDIOBJ);
        DeleteDC(memory_dc);
        ReleaseDC(null_mut(), screen_dc);
        if result == 0 {
            return Err("GetDIBits failed".to_string());
        }
        Ok(ScreenImage { width: w, height: h, pixels })
    }
}

// ── Monitor info ──────────────────────────────────────────────────────

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MonitorInfo {
    pub index: u32,
    pub rect: RECT,
    pub is_primary: bool,
    pub device_name: String,
}

#[allow(dead_code)]
pub fn enumerate_monitors() -> Vec<MonitorInfo> {
    let mut monitors = Vec::new();
    let (w, h) = (unsafe { GetSystemMetrics(SM_CXSCREEN) }, unsafe { GetSystemMetrics(SM_CYSCREEN) });
    // Use virtual screen approach with SM monitors
    let primary = RECT { left: 0, top: 0, right: w, bottom: h };
    monitors.push(MonitorInfo {
        index: 0,
        rect: primary,
        is_primary: true,
        device_name: "\\\\.\\DISPLAY1".to_string(),
    });

    let virt_w = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    if virt_w > w {
        // Check for secondary monitor via MonitorFromPoint
        for i in 1..8 {
            let test_x = w + (i - 1) * 100;
            let pt = POINT { x: test_x, y: h / 2 };
            let hm = unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST) };
            let mut mi = MONITORINFO { cbSize: size_of::<MONITORINFO>() as DWORD, ..zeroed_monitor_info() };
            if unsafe { GetMonitorInfoW(hm, &mut mi) } != 0 {
                if mi.rcMonitor.left >= w || mi.rcMonitor.top >= h || mi.rcMonitor.left < 0 {
                    monitors.push(MonitorInfo {
                        index: i as u32,
                        rect: mi.rcMonitor,
                        is_primary: (mi.dwFlags & MONITORINFOF_PRIMARY) != 0,
                        device_name: format!("\\\\.\\DISPLAY{}", i + 1),
                    });
                }
            }
        }
    }
    monitors
}

#[allow(dead_code)]
fn zeroed_monitor_info() -> MONITORINFO {
    MONITORINFO {
        cbSize: 0,
        rcMonitor: RECT { left: 0, top: 0, right: 0, bottom: 0 },
        rcWork: RECT { left: 0, top: 0, right: 0, bottom: 0 },
        dwFlags: 0,
    }
}

// ── Pixel/color search ────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[allow(dead_code)]
pub fn find_color(img: &ScreenImage, target: u32, tolerance: u8) -> Vec<Point> {
    let tr = ((target >> 16) & 0xFF) as i16;
    let tg = ((target >> 8) & 0xFF) as i16;
    let tb = (target & 0xFF) as i16;
    let tol = tolerance as i16;
    let mut results = Vec::new();
    for y in 0..img.height {
        for x in 0..img.width {
            let idx = ((y * img.width + x) * 4) as usize;
            let r = img.pixels[idx + 2] as i16;
            let g = img.pixels[idx + 1] as i16;
            let b = img.pixels[idx] as i16;
            if (r - tr).abs() <= tol && (g - tg).abs() <= tol && (b - tb).abs() <= tol {
                results.push(Point { x, y });
            }
        }
    }
    results
}

#[allow(dead_code)]
pub fn find_first_color(img: &ScreenImage, target: u32, tolerance: u8) -> Option<Point> {
    let tr = ((target >> 16) & 0xFF) as i16;
    let tg = ((target >> 8) & 0xFF) as i16;
    let tb = (target & 0xFF) as i16;
    let tol = tolerance as i16;
    for y in 0..img.height {
        for x in 0..img.width {
            let idx = ((y * img.width + x) * 4) as usize;
            let r = img.pixels[idx + 2] as i16;
            let g = img.pixels[idx + 1] as i16;
            let b = img.pixels[idx] as i16;
            if (r - tr).abs() <= tol && (g - tg).abs() <= tol && (b - tb).abs() <= tol {
                return Some(Point { x, y });
            }
        }
    }
    None
}

#[allow(dead_code)]
pub fn get_pixel_color(x: i32, y: i32) -> Option<u32> {
    let cap = capture_region(x, y, 1, 1).ok()?;
    cap.color_at(0, 0)
}

// ── OCR-based text search ─────────────────────────────────────────────

pub fn find_text_on_screen(text: &str) -> Result<Vec<Point>, String> {
    let img = capture_screen()?;
    find_text_in_image(&img, text)
}

pub fn find_text_in_image(img: &ScreenImage, text: &str) -> Result<Vec<Point>, String> {
    let temp = crate::settings::data_directory().join("_find_text_.bmp");
    img.save_bmp(&temp)?;
    let result = crate::ocr::recognize_smart(&temp)?;
    let _ = std::fs::remove_file(&temp);
    let lower = result.text.to_lowercase();
    let search = text.to_lowercase();
    if !lower.contains(&search) {
        return Ok(Vec::new());
    }
    // Return center of screen as approximate location
    let center_x = img.width / 2;
    let center_y = img.height / 2;
    Ok(vec![Point { x: center_x, y: center_y }])
}

#[allow(dead_code)]
pub fn wait_for_text(text: &str, timeout_ms: u64) -> Result<Point, String> {
    let start = std::time::Instant::now();
    while start.elapsed().as_millis() < timeout_ms as u128 {
        if let Ok(points) = find_text_on_screen(text) {
            if let Some(p) = points.first() {
                return Ok(p.clone());
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    Err(format!("Text '{text}' not found within {timeout_ms}ms"))
}

// ── Template/pattern matching (simple SAD) ────────────────────────────

#[allow(dead_code)]
pub fn find_template(
    haystack: &ScreenImage,
    needle: &ScreenImage,
    threshold: f64,
) -> Vec<Point> {
    let mut matches = Vec::new();
    let max_x = haystack.width - needle.width;
    let max_y = haystack.height - needle.height;
    if max_x < 0 || max_y < 0 {
        return matches;
    }
    for y in 0..=max_y {
        for x in 0..=max_x {
            let mut diff = 0.0;
            let mut count = 0;
            'inner: for ny in 0..needle.height {
                for nx in 0..needle.width {
                    let hi = ((y + ny) * haystack.width + (x + nx)) * 4;
                    let ni = (ny * needle.width + nx) * 4;
                    let dr = haystack.pixels[hi as usize + 2] as f64 - needle.pixels[ni as usize + 2] as f64;
                    let dg = haystack.pixels[hi as usize + 1] as f64 - needle.pixels[ni as usize + 1] as f64;
                    let db = haystack.pixels[hi as usize] as f64 - needle.pixels[ni as usize] as f64;
                    diff += dr * dr + dg * dg + db * db;
                    count += 1;
                    if diff > threshold * count as f64 {
                        break 'inner;
                    }
                }
            }
            let avg = diff / count as f64;
            if avg < threshold {
                matches.push(Point { x, y });
            }
        }
    }
    matches
}
