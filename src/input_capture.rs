#![allow(clippy::upper_case_acronyms)]
#![allow(non_snake_case)]

use crate::win::{POINT, WH_KEYBOARD_LL, WH_MOUSE_LL};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Clone, Debug)]
pub enum InputEvent {
    Key {
        ts_ms: u128,
        key: i32,
        down: bool,
    },
    MouseMove {
        ts_ms: u128,
        x: i32,
        y: i32,
    },
    MouseButton {
        ts_ms: u128,
        button: String,
        down: bool,
    },
}

pub struct InputCaptureHandle {
    events: Arc<Mutex<VecDeque<InputEvent>>>,
    thread_id: u32,
}

static GLOBAL_EVENTS: Mutex<Option<Arc<Mutex<VecDeque<InputEvent>>>>> = Mutex::new(None);

fn current_events() -> Option<Arc<Mutex<VecDeque<InputEvent>>>> {
    GLOBAL_EVENTS.lock().ok().and_then(|guard| guard.clone())
}

impl InputCaptureHandle {
    pub fn dump_events(&self) -> Vec<InputEvent> {
        self.events
            .lock()
            .map(|guard| guard.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn stop(&self) {
        unsafe {
            // post WM_QUIT to the capture thread to break its message loop
            let _ = crate::win::PostThreadMessageW(self.thread_id, crate::win::WM_QUIT, 0, 0);
        }
        if let Ok(mut guard) = GLOBAL_EVENTS.lock() {
            *guard = None;
        }
    }
}

#[repr(C)]
struct KBDLLHOOKSTRUCT {
    vkCode: u32,
    scanCode: u32,
    flags: u32,
    time: u32,
    dwExtraInfo: usize,
}

#[repr(C)]
struct MSLLHOOKSTRUCT {
    pt: POINT,
    mouseData: u32,
    flags: u32,
    time: u32,
    dwExtraInfo: usize,
}

extern "system" fn keyboard_proc(code: i32, wParam: usize, lParam: isize) -> isize {
    if code >= 0 {
        unsafe {
            let kb = &*(lParam as *const KBDLLHOOKSTRUCT);
            let now = now_ms();
            let down = matches!(wParam as u32, 0x0100 | 0x0104); // WM_KEYDOWN | WM_SYSKEYDOWN
            if let Some(evq) = current_events() {
                if let Ok(mut g) = evq.lock() {
                    g.push_back(InputEvent::Key {
                        ts_ms: now,
                        key: kb.vkCode as i32,
                        down,
                    });
                    trim_events(&mut g);
                }
            }
        }
    }
    unsafe { crate::win::CallNextHookEx(std::ptr::null_mut(), code, wParam, lParam) }
}

extern "system" fn mouse_proc(code: i32, wParam: usize, lParam: isize) -> isize {
    if code >= 0 {
        unsafe {
            let ms = &*(lParam as *const MSLLHOOKSTRUCT);
            let now = now_ms();
            if let Some(evq) = current_events() {
                if let Ok(mut g) = evq.lock() {
                    match wParam as u32 {
                        0x0201 => g.push_back(InputEvent::MouseButton {
                            ts_ms: now,
                            button: "left".to_string(),
                            down: true,
                        }),
                        0x0202 => g.push_back(InputEvent::MouseButton {
                            ts_ms: now,
                            button: "left".to_string(),
                            down: false,
                        }),
                        0x0204 => g.push_back(InputEvent::MouseButton {
                            ts_ms: now,
                            button: "right".to_string(),
                            down: true,
                        }),
                        0x0205 => g.push_back(InputEvent::MouseButton {
                            ts_ms: now,
                            button: "right".to_string(),
                            down: false,
                        }),
                        _ => g.push_back(InputEvent::MouseMove {
                            ts_ms: now,
                            x: ms.pt.x,
                            y: ms.pt.y,
                        }),
                    }
                    trim_events(&mut g);
                }
            }
        }
    }
    unsafe { crate::win::CallNextHookEx(std::ptr::null_mut(), code, wParam, lParam) }
}

pub fn start_input_capture(_past_seconds: usize) -> InputCaptureHandle {
    let events = Arc::new(Mutex::new(VecDeque::new()));
    if let Ok(mut guard) = GLOBAL_EVENTS.lock() {
        *guard = Some(events.clone());
    }

    let (tx_tid, rx_tid) = std::sync::mpsc::channel();

    let _thread = thread::spawn(move || {
        unsafe {
            let tid = crate::win::GetCurrentThreadId();
            let _ = tx_tid.send(tid);
            let _hk1 = crate::win::SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_proc),
                std::ptr::null_mut(),
                0,
            );
            let _hk2 = crate::win::SetWindowsHookExW(
                WH_MOUSE_LL,
                Some(mouse_proc),
                std::ptr::null_mut(),
                0,
            );
            let mut msg = crate::win::MSG::default();
            while crate::win::GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) != 0 {
                crate::win::TranslateMessage(&msg);
                crate::win::DispatchMessageW(&msg);
            }
            // unhook
            // Note: we didn't store the hook handles separately; best-effort unhook via null handle is a no-op
        }
    });

    let tid = rx_tid.recv().unwrap_or(0);

    InputCaptureHandle {
        events,
        thread_id: tid,
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn trim_events(events: &mut VecDeque<InputEvent>) {
    while events.len() > 10_000 {
        events.pop_front();
    }
}
