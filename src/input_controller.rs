#![allow(dead_code)]

use crate::win::*;
use std::mem::size_of;
use std::ptr::null_mut;
use std::thread;
use std::time::Duration;

pub fn send_input(inputs: &[INPUT]) -> u32 {
    unsafe {
        let ptr = inputs.as_ptr() as *mut INPUT;
        SendInput(
            inputs.len() as UINT,
            ptr,
            size_of::<INPUT>() as INT,
        )
    }
}

use std::mem::ManuallyDrop;

fn make_input(type_: DWORD, u: INPUT_UNION) -> INPUT {
    INPUT { type_, u }
}

fn make_mouse(dw_flags: DWORD, dx: LONG, dy: LONG, mouse_data: DWORD) -> INPUT {
    make_input(
        INPUT_MOUSE,
        INPUT_UNION {
            mi: ManuallyDrop::new(MOUSEINPUT {
                dx,
                dy,
                mouseData: mouse_data,
                dwFlags: dw_flags,
                time: 0,
                dwExtraInfo: 0,
            }),
        },
    )
}

fn make_key(w_vk: WORD, w_scan: WORD, dw_flags: DWORD) -> INPUT {
    make_input(
        INPUT_KEYBOARD,
        INPUT_UNION {
            ki: ManuallyDrop::new(KEYBDINPUT {
                wVk: w_vk,
                wScan: w_scan,
                dwFlags: dw_flags,
                time: 0,
                dwExtraInfo: 0,
            }),
        },
    )
}

// ── Mouse ─────────────────────────────────────────────────────────────

pub fn move_mouse(x: i32, y: i32) {
    send_input(&[make_mouse(MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE, absolute_x(x), absolute_y(y), 0)]);
}

pub fn move_mouse_relative(dx: i32, dy: i32) {
    send_input(&[make_mouse(MOUSEEVENTF_MOVE, dx, dy, 0)]);
}

pub fn get_cursor_pos() -> (i32, i32) {
    let mut pt = POINT { x: 0, y: 0 };
    unsafe { GetCursorPos(&mut pt); }
    (pt.x, pt.y)
}

pub fn click_left() {
    click_left_at(None);
}

pub fn click_left_at(pos: Option<(i32, i32)>) {
    let inputs = if let Some((x, y)) = pos {
        vec![
            make_mouse(MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE, absolute_x(x), absolute_y(y), 0),
            make_mouse(MOUSEEVENTF_LEFTDOWN, 0, 0, 0),
            make_mouse(MOUSEEVENTF_LEFTUP, 0, 0, 0),
        ]
    } else {
        vec![
            make_mouse(MOUSEEVENTF_LEFTDOWN, 0, 0, 0),
            make_mouse(MOUSEEVENTF_LEFTUP, 0, 0, 0),
        ]
    };
    send_input(&inputs);
}

pub fn click_right() {
    send_input(&[
        make_mouse(MOUSEEVENTF_RIGHTDOWN, 0, 0, 0),
        make_mouse(MOUSEEVENTF_RIGHTUP, 0, 0, 0),
    ]);
}

pub fn double_click() {
    click_left();
    thread::sleep(Duration::from_millis(50));
    click_left();
}

pub fn scroll_wheel(delta: i32) {
    send_input(&[make_mouse(MOUSEEVENTF_WHEEL, 0, 0, delta as DWORD)]);
}

pub fn drag(from: (i32, i32), to: (i32, i32)) {
    send_input(&[make_mouse(MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE, absolute_x(from.0), absolute_y(from.1), 0)]);
    thread::sleep(Duration::from_millis(100));
    send_input(&[make_mouse(MOUSEEVENTF_LEFTDOWN, 0, 0, 0)]);
    thread::sleep(Duration::from_millis(50));
    let steps = 10;
    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let x = from.0 as f64 + (to.0 - from.0) as f64 * t;
        let y = from.1 as f64 + (to.1 - from.1) as f64 * t;
        send_input(&[make_mouse(MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE, absolute_x(x as i32), absolute_y(y as i32), 0)]);
        thread::sleep(Duration::from_millis(10));
    }
    send_input(&[make_mouse(MOUSEEVENTF_LEFTUP, 0, 0, 0)]);
}

fn absolute_x(x: i32) -> i32 {
    let (w, _) = get_screen_dimensions();
    if w == 0 { return 0; }
    (x as i64 * 65536 / w as i64) as i32
}

fn absolute_y(y: i32) -> i32 {
    let (_, h) = get_screen_dimensions();
    if h == 0 { return 0; }
    (y as i64 * 65536 / h as i64) as i32
}

fn get_screen_dimensions() -> (i32, i32) {
    let w = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let h = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    if w > 0 && h > 0 { (w, h) } else {
        (unsafe { GetSystemMetrics(SM_CXSCREEN) }, unsafe { GetSystemMetrics(SM_CYSCREEN) })
    }
}

// ── Keyboard ──────────────────────────────────────────────────────────

pub fn type_text(text: &str) {
    for c in text.chars() {
        type_char(c);
    }
}

fn type_char(c: char) {
    if let Some((vk, scan, shift)) = char_to_key(c) {
        if shift {
            send_input(&[make_key(VK_LSHIFT, 0x2A, 0)]);
            thread::sleep(Duration::from_millis(10));
        }
        if scan != 0 {
            send_input(&[make_key(vk, scan as WORD, KEYEVENTF_SCANCODE)]);
            thread::sleep(Duration::from_millis(10));
            send_input(&[make_key(vk, scan as WORD, KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP)]);
        } else {
            send_input(&[make_key(vk, 0, 0)]);
            thread::sleep(Duration::from_millis(10));
            send_input(&[make_key(vk, 0, KEYEVENTF_KEYUP)]);
        }
        if shift {
            thread::sleep(Duration::from_millis(10));
            send_input(&[make_key(VK_LSHIFT, 0x2A, KEYEVENTF_KEYUP)]);
        }
    }
    thread::sleep(Duration::from_millis(15));
}

pub fn press_key(vk: WORD) {
    send_input(&[make_key(vk, 0, 0)]);
    thread::sleep(Duration::from_millis(10));
    send_input(&[make_key(vk, 0, KEYEVENTF_KEYUP)]);
}

pub fn hold_key_down(vk: WORD) {
    send_input(&[make_key(vk, 0, 0)]);
}

pub fn release_key(vk: WORD) {
    send_input(&[make_key(vk, 0, KEYEVENTF_KEYUP)]);
}

pub fn key_combo(keys: &[WORD]) {
    for &k in keys {
        send_input(&[make_key(k, 0, 0)]);
        thread::sleep(Duration::from_millis(15));
    }
    for &k in keys.iter().rev() {
        thread::sleep(Duration::from_millis(15));
        send_input(&[make_key(k, 0, KEYEVENTF_KEYUP)]);
    }
}

pub fn hotkey(ctrl: bool, alt: bool, shift: bool, vk: WORD) {
    let mut keys = Vec::new();
    if ctrl { keys.push(VK_LCONTROL); }
    if alt { keys.push(VK_LMENU); }
    if shift { keys.push(VK_LSHIFT); }
    keys.push(vk);
    key_combo(&keys);
}

pub fn type_from_clipboard() {
    if let Ok(text) = crate::clipboard::read_text(null_mut()) {
        type_text(&text);
    }
}

// ── Window focus ──────────────────────────────────────────────────────

pub fn focus_window(hwnd: crate::win::HWND) {
    unsafe {
        SetForegroundWindow(hwnd);
        SetFocus(hwnd);
    }
}

pub fn find_window(title: &str) -> Option<crate::win::HWND> {
    let title_wide = crate::win::wide_null(title);
    let hwnd = unsafe { FindWindowW(null_mut(), title_wide.as_ptr()) };
    if hwnd.is_null() { None } else { Some(hwnd) }
}

pub fn activate_window(title: &str) -> bool {
    find_window(title).map(|hwnd| { focus_window(hwnd); true }).unwrap_or(false)
}

// ── Utility: wait ─────────────────────────────────────────────────────

pub fn wait_ms(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}
