use crate::selector::ScreenRect;
use crate::win::*;
use std::collections::HashMap;
use std::ptr::null_mut;
use std::sync::{mpsc, Mutex, OnceLock};
use std::thread;
use std::time::Instant;

pub const WM_RECORDING_INDICATOR_STOP: UINT = WM_APP + 2;

const WIDTH: i32 = 230;
const HEIGHT: i32 = 64;
const MARGIN: i32 = 12;
const TIMER_ID: usize = 1;

static CLASS_REGISTRATION: OnceLock<Result<(), String>> = OnceLock::new();
static STATES: OnceLock<Mutex<HashMap<usize, IndicatorState>>> = OnceLock::new();

struct IndicatorState {
    app_window: HWND,
    started: Instant,
    dragging: bool,
    drag_offset: POINT,
}

unsafe impl Send for IndicatorState {}

fn states() -> &'static Mutex<HashMap<usize, IndicatorState>> {
    STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

pub struct RecordingIndicator {
    window: HWND,
}

unsafe impl Send for RecordingIndicator {}

impl RecordingIndicator {
    pub fn show(app_window: HWND, rect: ScreenRect) -> Result<Self, String> {
        initialize()?;
        let (sender, receiver) = mpsc::sync_channel(1);
        let app_window = app_window as usize;

        thread::spawn(move || unsafe {
            let class_name = wide_null("ParkerRecordingIndicator");
            let title = wide_null("Parker recording control");
            let instance = GetModuleHandleW(null_mut());
            let (x, y) = initial_position(rect);
            let window = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_POPUP,
                x,
                y,
                WIDTH,
                HEIGHT,
                null_mut(),
                null_mut(),
                instance,
                null_mut(),
            );

            if window.is_null() {
                let _ = sender.send(Err(
                    "Could not create the recording control window.".to_string()
                ));
                return;
            }

            let region = CreateRoundRectRgn(0, 0, WIDTH + 1, HEIGHT + 1, 24, 24);
            if !region.is_null() && SetWindowRgn(window, region, TRUE) == 0 {
                DeleteObject(region as HGDIOBJ);
            }
            if SetWindowDisplayAffinity(window, WDA_EXCLUDEFROMCAPTURE) == 0 {
                DestroyWindow(window);
                let _ = sender.send(Err(
                    "Windows could not exclude the recording control from capture.".to_string(),
                ));
                return;
            }

            if let Ok(mut map) = states().lock() {
                map.insert(
                    window as usize,
                    IndicatorState {
                        app_window: app_window as HWND,
                        started: Instant::now(),
                        dragging: false,
                        drag_offset: POINT::default(),
                    },
                );
            }

            ShowWindow(window, SW_SHOWNOACTIVATE);
            UpdateWindow(window);
            SetTimer(window, TIMER_ID, 250, None);
            let _ = sender.send(Ok(window as usize));

            let mut event = MSG::default();
            while GetMessageW(&mut event, null_mut(), 0, 0) > 0 {
                TranslateMessage(&event);
                DispatchMessageW(&event);
            }
        });

        let window = receiver
            .recv()
            .map_err(|_| "The recording control thread did not start.".to_string())??;
        Ok(Self {
            window: window as HWND,
        })
    }

    pub fn close(&mut self) {
        if !self.window.is_null() {
            unsafe {
                PostMessageW(self.window, WM_CLOSE, 0, 0);
            }
            self.window = null_mut();
        }
    }
}

impl Drop for RecordingIndicator {
    fn drop(&mut self) {
        self.close();
    }
}

fn initialize() -> Result<(), String> {
    CLASS_REGISTRATION
        .get_or_init(|| {
            let class_name = wide_null("ParkerRecordingIndicator");
            let instance = unsafe { GetModuleHandleW(null_mut()) };
            let class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(indicator_window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: unsafe { LoadIconW(instance, 101usize as *const u16) },
                hCursor: unsafe { LoadCursorW(null_mut(), IDC_ARROW as *const u16) },
                hbrBackground: null_mut(),
                lpszMenuName: null_mut(),
                lpszClassName: class_name.as_ptr(),
            };

            if unsafe { RegisterClassW(&class) } == 0 {
                Err("Could not register the recording control window.".to_string())
            } else {
                Ok(())
            }
        })
        .clone()
}

fn initial_position(rect: ScreenRect) -> (i32, i32) {
    let virtual_left = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let virtual_top = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let virtual_right = virtual_left + unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let virtual_bottom = virtual_top + unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    let x = rect
        .x
        .clamp(virtual_left + MARGIN, virtual_right - WIDTH - MARGIN);

    if rect.y - HEIGHT - MARGIN >= virtual_top {
        return (x, rect.y - HEIGHT - MARGIN);
    }
    if rect.y + rect.height + HEIGHT + MARGIN <= virtual_bottom {
        return (x, rect.y + rect.height + MARGIN);
    }
    if rect.x + rect.width + WIDTH + MARGIN <= virtual_right {
        return (
            rect.x + rect.width + MARGIN,
            rect.y.max(virtual_top + MARGIN),
        );
    }
    if rect.x - WIDTH - MARGIN >= virtual_left {
        return (rect.x - WIDTH - MARGIN, rect.y.max(virtual_top + MARGIN));
    }

    (
        virtual_right - WIDTH - MARGIN,
        virtual_bottom - HEIGHT - MARGIN,
    )
}

unsafe extern "system" fn indicator_window_proc(
    window: HWND,
    message: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_PAINT => {
            paint_indicator(window);
            0
        }
        WM_TIMER => {
            InvalidateRect(window, null_mut(), FALSE);
            0
        }
        WM_LBUTTONDOWN => {
            let mut cursor = POINT::default();
            let mut bounds = RECT::default();
            if GetCursorPos(&mut cursor) != 0 && GetWindowRect(window, &mut bounds) != 0 {
                let local_x = cursor.x - bounds.left;
                if local_x >= WIDTH - 58 {
                    if let Ok(map) = states().lock() {
                        if let Some(state) = map.get(&(window as usize)) {
                            PostMessageW(state.app_window, WM_RECORDING_INDICATOR_STOP, 0, 0);
                        }
                    }
                } else if let Ok(mut map) = states().lock() {
                    if let Some(state) = map.get_mut(&(window as usize)) {
                        state.dragging = true;
                        state.drag_offset = POINT {
                            x: cursor.x - bounds.left,
                            y: cursor.y - bounds.top,
                        };
                        SetCapture(window);
                    }
                }
            }
            0
        }
        WM_MOUSEMOVE => {
            if let Ok(map) = states().lock() {
                if let Some(state) = map.get(&(window as usize)) {
                    if state.dragging {
                        let mut cursor = POINT::default();
                        if GetCursorPos(&mut cursor) != 0 {
                            let virtual_left = GetSystemMetrics(SM_XVIRTUALSCREEN);
                            let virtual_top = GetSystemMetrics(SM_YVIRTUALSCREEN);
                            let virtual_right = virtual_left + GetSystemMetrics(SM_CXVIRTUALSCREEN);
                            let virtual_bottom = virtual_top + GetSystemMetrics(SM_CYVIRTUALSCREEN);
                            let x = (cursor.x - state.drag_offset.x)
                                .clamp(virtual_left, virtual_right - WIDTH);
                            let y = (cursor.y - state.drag_offset.y)
                                .clamp(virtual_top, virtual_bottom - HEIGHT);
                            SetWindowPos(
                                window,
                                null_mut(),
                                x,
                                y,
                                0,
                                0,
                                SWP_NOSIZE | SWP_NOACTIVATE,
                            );
                        }
                    }
                }
            }
            0
        }
        WM_LBUTTONUP => {
            ReleaseCapture();
            if let Ok(mut map) = states().lock() {
                if let Some(state) = map.get_mut(&(window as usize)) {
                    state.dragging = false;
                }
            }
            0
        }
        WM_CLOSE => {
            DestroyWindow(window);
            0
        }
        WM_DESTROY => {
            KillTimer(window, TIMER_ID);
            if let Ok(mut map) = states().lock() {
                map.remove(&(window as usize));
            }
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(window, message, wparam, lparam),
    }
}

unsafe fn paint_indicator(window: HWND) {
    let mut paint = PAINTSTRUCT::default();
    let device = BeginPaint(window, &mut paint);
    if device.is_null() {
        return;
    }

    let elapsed = states()
        .lock()
        .ok()
        .and_then(|map| {
            map.get(&(window as usize))
                .map(|state| state.started.elapsed())
        })
        .unwrap_or_default();

    let background = CreateSolidBrush(rgb(30, 31, 35));
    let border = CreatePen(PS_SOLID, 1, rgb(68, 69, 74));
    let old_brush = SelectObject(device, background as HGDIOBJ);
    let old_pen = SelectObject(device, border as HGDIOBJ);
    RoundRect(device, 0, 0, WIDTH, HEIGHT, 24, 24);
    SelectObject(device, old_pen);
    SelectObject(device, old_brush);
    DeleteObject(border as HGDIOBJ);
    DeleteObject(background as HGDIOBJ);

    let pulse = if (elapsed.as_millis() / 500).is_multiple_of(2) {
        rgb(255, 59, 48)
    } else {
        rgb(178, 42, 36)
    };
    let red = CreateSolidBrush(pulse);
    let old_brush = SelectObject(device, red as HGDIOBJ);
    let old_pen = SelectObject(device, GetStockObject(NULL_BRUSH));
    Ellipse(device, 18, 24, 32, 38);
    SelectObject(device, old_pen);
    SelectObject(device, old_brush);
    DeleteObject(red as HGDIOBJ);

    let stop_background = CreateSolidBrush(rgb(76, 36, 38));
    let old_brush = SelectObject(device, stop_background as HGDIOBJ);
    let old_pen = SelectObject(device, GetStockObject(NULL_BRUSH));
    RoundRect(device, WIDTH - 54, 10, WIDTH - 10, HEIGHT - 10, 16, 16);
    SelectObject(device, old_pen);
    SelectObject(device, old_brush);
    DeleteObject(stop_background as HGDIOBJ);

    let stop = CreateSolidBrush(rgb(255, 245, 245));
    let stop_rect = RECT {
        left: WIDTH - 39,
        top: 25,
        right: WIDTH - 25,
        bottom: 39,
    };
    FillRect(device, &stop_rect, stop);
    DeleteObject(stop as HGDIOBJ);

    SetBkMode(device, TRANSPARENT);
    let face = wide_null("Segoe UI");
    let label_font = CreateFontW(
        -13,
        0,
        0,
        0,
        FW_SEMIBOLD,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        0,
        0,
        CLEARTYPE_QUALITY,
        DEFAULT_PITCH,
        face.as_ptr(),
    );
    let old_font = SelectObject(device, label_font as HGDIOBJ);
    SetTextColor(device, rgb(255, 100, 92));
    let rec = wide_null("REC");
    let mut rec_rect = RECT {
        left: 40,
        top: 10,
        right: 78,
        bottom: 31,
    };
    DrawTextW(
        device,
        rec.as_ptr(),
        -1,
        &mut rec_rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    SelectObject(device, old_font);
    DeleteObject(label_font as HGDIOBJ);

    let timer_font = CreateFontW(
        -20,
        0,
        0,
        0,
        FW_NORMAL,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        0,
        0,
        CLEARTYPE_QUALITY,
        DEFAULT_PITCH,
        face.as_ptr(),
    );
    let old_font = SelectObject(device, timer_font as HGDIOBJ);
    SetTextColor(device, rgb(247, 247, 249));
    let total_seconds = elapsed.as_secs();
    let timer = format!("{:02}:{:02}", total_seconds / 60, total_seconds % 60);
    let timer = wide_null(&timer);
    let mut timer_rect = RECT {
        left: 40,
        top: 27,
        right: WIDTH - 65,
        bottom: 55,
    };
    DrawTextW(
        device,
        timer.as_ptr(),
        -1,
        &mut timer_rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    SelectObject(device, old_font);
    DeleteObject(timer_font as HGDIOBJ);

    EndPaint(window, &paint);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screenshot;
    use std::fs;
    use std::time::Duration;

    #[test]
    #[ignore = "requires an interactive Windows desktop"]
    fn indicator_is_excluded_from_screen_capture() {
        let virtual_left = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
        let virtual_top = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
        let virtual_width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
        let virtual_height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
        let candidates = [
            (virtual_left + 24, virtual_top + 24),
            (virtual_left + virtual_width - WIDTH - 24, virtual_top + 24),
            (
                virtual_left + 24,
                virtual_top + virtual_height - HEIGHT - 80,
            ),
        ];
        let temp =
            std::env::temp_dir().join(format!("parker-indicator-test-{}", std::process::id()));
        fs::create_dir_all(&temp).expect("create indicator test directory");

        let mut stable = None;
        for (index, (x, y)) in candidates.into_iter().enumerate() {
            let area = ScreenRect {
                x,
                y,
                width: WIDTH,
                height: HEIGHT,
            };
            let before_a = temp.join(format!("before-{index}-a.bmp"));
            let before_b = temp.join(format!("before-{index}-b.bmp"));
            screenshot::capture_region_to_bmp(area, &before_a).expect("capture baseline A");
            thread::sleep(Duration::from_millis(150));
            screenshot::capture_region_to_bmp(area, &before_b).expect("capture baseline B");
            if fs::read(&before_a).expect("read baseline A")
                == fs::read(&before_b).expect("read baseline B")
            {
                stable = Some((area, before_b));
                break;
            }
        }

        let (area, before) = stable.expect("find a stable desktop patch for capture test");
        let selection = ScreenRect {
            x: area.x,
            y: area.y + HEIGHT + MARGIN,
            width: WIDTH,
            height: 120,
        };
        assert_eq!(initial_position(selection), (area.x, area.y));

        let indicator = RecordingIndicator::show(null_mut(), selection)
            .expect("show excluded recording indicator");
        thread::sleep(Duration::from_millis(600));
        let during = temp.join("during.bmp");
        screenshot::capture_region_to_bmp(area, &during).expect("capture with indicator visible");
        drop(indicator);

        assert_eq!(
            fs::read(before).expect("read stable baseline"),
            fs::read(during).expect("read capture with indicator"),
            "recording indicator changed captured pixels"
        );
        let _ = fs::remove_dir_all(temp);
    }
}
