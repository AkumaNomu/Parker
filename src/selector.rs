use crate::win::*;
use std::ptr::null_mut;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Debug)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Default)]
struct SelectionState {
    start: POINT,
    current: POINT,
    dragging: bool,
    done: bool,
    result: Option<ScreenRect>,
    virtual_x: i32,
    virtual_y: i32,
    prompt: String,
}

static STATE: OnceLock<Mutex<SelectionState>> = OnceLock::new();
static CLASS_REGISTRATION: OnceLock<Result<(), String>> = OnceLock::new();

fn state() -> &'static Mutex<SelectionState> {
    STATE.get_or_init(|| Mutex::new(SelectionState::default()))
}

pub fn select_region(prompt: &str) -> Result<Option<ScreenRect>, String> {
    ensure_window_class()?;

    let virtual_x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let virtual_y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let virtual_width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let virtual_height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };

    if virtual_width <= 0 || virtual_height <= 0 {
        return Err("Windows reported an invalid virtual desktop size.".to_string());
    }

    {
        let mut selection = state()
            .lock()
            .map_err(|_| "The region selector state is unavailable.".to_string())?;
        *selection = SelectionState {
            virtual_x,
            virtual_y,
            prompt: prompt.to_string(),
            ..SelectionState::default()
        };
    }

    let class_name = wide_null("ParkerRegionSelector");
    let title = wide_null("Parker region selector");
    let instance = unsafe { GetModuleHandleW(null_mut()) };
    let window = unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP,
            virtual_x,
            virtual_y,
            virtual_width,
            virtual_height,
            null_mut(),
            null_mut(),
            instance,
            null_mut(),
        )
    };

    if window.is_null() {
        return Err("Could not create the region selector.".to_string());
    }

    unsafe {
        SetLayeredWindowAttributes(window, 0, 145, LWA_ALPHA);
        ShowWindow(window, SW_SHOW);
        UpdateWindow(window);
        SetForegroundWindow(window);
        SetFocus(window);
    }

    let mut message = MSG::default();
    loop {
        let done = state()
            .lock()
            .map_err(|_| "The region selector state is unavailable.".to_string())?
            .done;
        if done {
            break;
        }

        let result = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
        if result <= 0 {
            unsafe {
                DestroyWindow(window);
            }
            return Err("The region selector message loop ended unexpectedly.".to_string());
        }

        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    let mut selection = state()
        .lock()
        .map_err(|_| "The region selector state is unavailable.".to_string())?;
    Ok(selection.result.take())
}

fn ensure_window_class() -> Result<(), String> {
    CLASS_REGISTRATION
        .get_or_init(|| {
            let class_name = wide_null("ParkerRegionSelector");
            let instance = unsafe { GetModuleHandleW(null_mut()) };
            let cursor = unsafe { LoadCursorW(null_mut(), IDC_CROSS as *const u16) };
            let class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(selector_window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: null_mut(),
                hCursor: cursor,
                hbrBackground: unsafe { GetStockObject(BLACK_BRUSH) } as HBRUSH,
                lpszMenuName: null_mut(),
                lpszClassName: class_name.as_ptr(),
            };

            if unsafe { RegisterClassW(&class) } == 0 {
                Err("Could not register the Parker region selector window class.".to_string())
            } else {
                Ok(())
            }
        })
        .clone()
}

unsafe extern "system" fn selector_window_proc(
    window: HWND,
    message: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_LBUTTONDOWN => {
            let mut point = POINT::default();
            if GetCursorPos(&mut point) != 0 {
                if let Ok(mut selection) = state().lock() {
                    selection.start = point;
                    selection.current = point;
                    selection.dragging = true;
                }
                SetCapture(window);
                InvalidateRect(window, null_mut(), TRUE);
            }
            0
        }
        WM_MOUSEMOVE => {
            let mut point = POINT::default();
            if GetCursorPos(&mut point) != 0 {
                let mut should_redraw = false;
                if let Ok(mut selection) = state().lock() {
                    if selection.dragging {
                        selection.current = point;
                        should_redraw = true;
                    }
                }
                if should_redraw {
                    InvalidateRect(window, null_mut(), FALSE);
                }
            }
            0
        }
        WM_LBUTTONUP => {
            let mut point = POINT::default();
            GetCursorPos(&mut point);
            ReleaseCapture();

            if let Ok(mut selection) = state().lock() {
                selection.current = point;
                selection.dragging = false;
                selection.result = normalize_selection(selection.start, selection.current);
                selection.done = true;
            }

            DestroyWindow(window);
            0
        }
        WM_RBUTTONDOWN => {
            cancel_selection(window);
            0
        }
        WM_KEYDOWN if wparam == VK_ESCAPE => {
            cancel_selection(window);
            0
        }
        WM_PAINT => {
            paint_selector(window);
            0
        }
        WM_DESTROY => {
            if let Ok(mut selection) = state().lock() {
                selection.done = true;
            }
            0
        }
        _ => DefWindowProcW(window, message, wparam, lparam),
    }
}

unsafe fn cancel_selection(window: HWND) {
    ReleaseCapture();
    if let Ok(mut selection) = state().lock() {
        selection.dragging = false;
        selection.result = None;
        selection.done = true;
    }
    DestroyWindow(window);
}

unsafe fn paint_selector(window: HWND) {
    let mut paint = PAINTSTRUCT::default();
    let device = BeginPaint(window, &mut paint);
    if device.is_null() {
        return;
    }

    let mut client = RECT::default();
    GetClientRect(window, &mut client);
    FillRect(device, &client, GetStockObject(BLACK_BRUSH) as HBRUSH);
    SetBkMode(device, TRANSPARENT);
    SetTextColor(device, rgb(255, 255, 255));

    if let Ok(selection) = state().lock() {
        let instructions = wide_null(&selection.prompt);
        let mut instructions_rect = RECT {
            left: 24,
            top: 20,
            right: client.right - 24,
            bottom: 52,
        };
        DrawTextW(
            device,
            instructions.as_ptr(),
            -1,
            &mut instructions_rect,
            DT_LEFT | DT_SINGLELINE | DT_NOPREFIX,
        );

        if selection.dragging {
            let left = selection.start.x.min(selection.current.x) - selection.virtual_x;
            let top = selection.start.y.min(selection.current.y) - selection.virtual_y;
            let right = selection.start.x.max(selection.current.x) - selection.virtual_x;
            let bottom = selection.start.y.max(selection.current.y) - selection.virtual_y;

            let pen = CreatePen(PS_SOLID, 3, rgb(255, 255, 255));
            let old_pen = SelectObject(device, pen as HGDIOBJ);
            let old_brush = SelectObject(device, GetStockObject(NULL_BRUSH));
            Rectangle(device, left, top, right, bottom);
            SelectObject(device, old_brush);
            SelectObject(device, old_pen);
            DeleteObject(pen as HGDIOBJ);

            let label = format!("{} × {}", (right - left).abs(), (bottom - top).abs());
            let label_wide = wide_null(&label);
            let mut label_rect = RECT {
                left,
                top: (top - 26).max(58),
                right: left + 220,
                bottom: (top - 2).max(82),
            };
            DrawTextW(
                device,
                label_wide.as_ptr(),
                -1,
                &mut label_rect,
                DT_LEFT | DT_SINGLELINE | DT_NOPREFIX,
            );
        }
    }

    EndPaint(window, &paint);
}

fn normalize_selection(start: POINT, end: POINT) -> Option<ScreenRect> {
    let left = start.x.min(end.x);
    let top = start.y.min(end.y);
    let right = start.x.max(end.x);
    let bottom = start.y.max(end.y);
    let width = right - left;
    let height = bottom - top;

    if width < 4 || height < 4 {
        None
    } else {
        Some(ScreenRect {
            x: left,
            y: top,
            width,
            height,
        })
    }
}
