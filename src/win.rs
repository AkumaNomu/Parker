#![allow(
    non_snake_case,
    non_camel_case_types,
    dead_code,
    clippy::upper_case_acronyms,
    clippy::too_many_arguments
)]

use std::ffi::c_void;

pub type BOOL = i32;
pub type BYTE = u8;
pub type WORD = u16;
pub type DWORD = u32;
pub type UINT = u32;
pub type LONG = i32;
pub type INT = i32;
pub type WPARAM = usize;
pub type LPARAM = isize;
pub type LRESULT = isize;
pub type HANDLE = *mut c_void;
pub type HWND = HANDLE;
pub type HINSTANCE = HANDLE;
pub type HICON = HANDLE;
pub type HCURSOR = HANDLE;
pub type HBRUSH = HANDLE;
pub type HGDIOBJ = HANDLE;
pub type HPEN = HANDLE;
pub type HDC = HANDLE;
pub type HBITMAP = HANDLE;
pub type HGLOBAL = HANDLE;
pub type HMENU = HANDLE;
pub type ATOM = WORD;
pub type COLORREF = DWORD;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct POINT {
    pub x: LONG,
    pub y: LONG,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct RECT {
    pub left: LONG,
    pub top: LONG,
    pub right: LONG,
    pub bottom: LONG,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: UINT,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: DWORD,
    pub pt: POINT,
    pub lPrivate: DWORD,
}

impl Default for MSG {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

pub type WNDPROC = Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT>;

#[repr(C)]
pub struct WNDCLASSW {
    pub style: UINT,
    pub lpfnWndProc: WNDPROC,
    pub cbClsExtra: INT,
    pub cbWndExtra: INT,
    pub hInstance: HINSTANCE,
    pub hIcon: HICON,
    pub hCursor: HCURSOR,
    pub hbrBackground: HBRUSH,
    pub lpszMenuName: *const u16,
    pub lpszClassName: *const u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PAINTSTRUCT {
    pub hdc: HDC,
    pub fErase: BOOL,
    pub rcPaint: RECT,
    pub fRestore: BOOL,
    pub fIncUpdate: BOOL,
    pub rgbReserved: [BYTE; 32],
}

impl Default for PAINTSTRUCT {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SYSTEMTIME {
    pub wYear: WORD,
    pub wMonth: WORD,
    pub wDayOfWeek: WORD,
    pub wDay: WORD,
    pub wHour: WORD,
    pub wMinute: WORD,
    pub wSecond: WORD,
    pub wMilliseconds: WORD,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DROPFILES {
    pub pFiles: DWORD,
    pub pt: POINT,
    pub fNC: BOOL,
    pub fWide: BOOL,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RGBQUAD {
    pub rgbBlue: BYTE,
    pub rgbGreen: BYTE,
    pub rgbRed: BYTE,
    pub rgbReserved: BYTE,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct BITMAPINFOHEADER {
    pub biSize: DWORD,
    pub biWidth: LONG,
    pub biHeight: LONG,
    pub biPlanes: WORD,
    pub biBitCount: WORD,
    pub biCompression: DWORD,
    pub biSizeImage: DWORD,
    pub biXPelsPerMeter: LONG,
    pub biYPelsPerMeter: LONG,
    pub biClrUsed: DWORD,
    pub biClrImportant: DWORD,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct BITMAPINFO {
    pub bmiHeader: BITMAPINFOHEADER,
    pub bmiColors: [RGBQUAD; 1],
}

pub const TRUE: BOOL = 1;
pub const FALSE: BOOL = 0;

pub const CS_VREDRAW: UINT = 0x0001;
pub const CS_HREDRAW: UINT = 0x0002;

pub const WM_DESTROY: UINT = 0x0002;
pub const WM_PAINT: UINT = 0x000F;
pub const WM_KEYDOWN: UINT = 0x0100;
pub const WM_TIMER: UINT = 0x0113;
pub const WM_HOTKEY: UINT = 0x0312;
pub const WM_MOUSEMOVE: UINT = 0x0200;
pub const WM_LBUTTONDOWN: UINT = 0x0201;
pub const WM_LBUTTONUP: UINT = 0x0202;
pub const WM_RBUTTONDOWN: UINT = 0x0204;

pub const VK_ESCAPE: WPARAM = 0x1B;
pub const VK_F8: UINT = 0x77;
pub const VK_F9: UINT = 0x78;
pub const VK_F10: UINT = 0x79;
pub const VK_F12: UINT = 0x7B;

pub const MOD_CONTROL: UINT = 0x0002;
pub const MOD_SHIFT: UINT = 0x0004;
pub const MOD_NOREPEAT: UINT = 0x4000;

pub const WS_POPUP: DWORD = 0x8000_0000;
pub const WS_EX_TOPMOST: DWORD = 0x0000_0008;
pub const WS_EX_TOOLWINDOW: DWORD = 0x0000_0080;
pub const WS_EX_LAYERED: DWORD = 0x0008_0000;
pub const WS_EX_NOACTIVATE: DWORD = 0x0800_0000;

pub const SW_SHOWNOACTIVATE: INT = 4;
pub const SW_SHOW: INT = 5;
pub const SW_SHOWNORMAL: INT = 1;
pub const LWA_ALPHA: DWORD = 0x0000_0002;

pub const SM_CXSCREEN: INT = 0;
pub const SM_CYSCREEN: INT = 1;
pub const SM_XVIRTUALSCREEN: INT = 76;
pub const SM_YVIRTUALSCREEN: INT = 77;
pub const SM_CXVIRTUALSCREEN: INT = 78;
pub const SM_CYVIRTUALSCREEN: INT = 79;

pub const IDC_ARROW: usize = 32512;
pub const IDC_CROSS: usize = 32515;
pub const BLACK_BRUSH: INT = 4;
pub const NULL_BRUSH: INT = 5;
pub const PS_SOLID: INT = 0;
pub const TRANSPARENT: INT = 1;

pub const DT_LEFT: UINT = 0x0000;
pub const DT_VCENTER: UINT = 0x0004;
pub const DT_WORDBREAK: UINT = 0x0010;
pub const DT_SINGLELINE: UINT = 0x0020;
pub const DT_NOPREFIX: UINT = 0x0800;

pub const CF_UNICODETEXT: UINT = 13;
pub const CF_HDROP: UINT = 15;
pub const GMEM_MOVEABLE: UINT = 0x0002;
pub const GMEM_ZEROINIT: UINT = 0x0040;

pub const MB_OK: UINT = 0x0000;
pub const MB_ICONERROR: UINT = 0x0010;
pub const MB_ICONINFORMATION: UINT = 0x0040;
pub const MB_TOPMOST: UINT = 0x0004_0000;

pub const BI_RGB: DWORD = 0;
pub const DIB_RGB_COLORS: UINT = 0;
pub const SRCCOPY: DWORD = 0x00CC_0020;
pub const CAPTUREBLT: DWORD = 0x4000_0000;

pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;
pub const SPI_GETWORKAREA: UINT = 0x0030;
pub const WDA_EXCLUDEFROMCAPTURE: DWORD = 0x0000_0011;

pub const fn rgb(red: BYTE, green: BYTE, blue: BYTE) -> COLORREF {
    red as DWORD | ((green as DWORD) << 8) | ((blue as DWORD) << 16)
}

pub const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;

#[link(name = "user32")]
extern "system" {
    pub fn RegisterHotKey(hWnd: HWND, id: INT, fsModifiers: UINT, vk: UINT) -> BOOL;
    pub fn UnregisterHotKey(hWnd: HWND, id: INT) -> BOOL;
    pub fn GetMessageW(lpMsg: *mut MSG, hWnd: HWND, min: UINT, max: UINT) -> BOOL;
    pub fn TranslateMessage(lpMsg: *const MSG) -> BOOL;
    pub fn DispatchMessageW(lpMsg: *const MSG) -> LRESULT;
    pub fn DefWindowProcW(hWnd: HWND, msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn RegisterClassW(lpWndClass: *const WNDCLASSW) -> ATOM;
    pub fn CreateWindowExW(
        exStyle: DWORD,
        className: *const u16,
        windowName: *const u16,
        style: DWORD,
        x: INT,
        y: INT,
        width: INT,
        height: INT,
        parent: HWND,
        menu: HMENU,
        instance: HINSTANCE,
        param: *mut c_void,
    ) -> HWND;
    pub fn DestroyWindow(hWnd: HWND) -> BOOL;
    pub fn ShowWindow(hWnd: HWND, cmdShow: INT) -> BOOL;
    pub fn UpdateWindow(hWnd: HWND) -> BOOL;
    pub fn SetForegroundWindow(hWnd: HWND) -> BOOL;
    pub fn SetFocus(hWnd: HWND) -> HWND;
    pub fn SetLayeredWindowAttributes(
        hWnd: HWND,
        colorKey: COLORREF,
        alpha: BYTE,
        flags: DWORD,
    ) -> BOOL;
    pub fn GetSystemMetrics(index: INT) -> INT;
    pub fn SystemParametersInfoW(action: UINT, param: UINT, value: *mut c_void, flags: UINT) -> BOOL;
    pub fn LoadCursorW(instance: HINSTANCE, cursorName: *const u16) -> HCURSOR;
    pub fn BeginPaint(hWnd: HWND, paint: *mut PAINTSTRUCT) -> HDC;
    pub fn EndPaint(hWnd: HWND, paint: *const PAINTSTRUCT) -> BOOL;
    pub fn GetClientRect(hWnd: HWND, rect: *mut RECT) -> BOOL;
    pub fn FillRect(hdc: HDC, rect: *const RECT, brush: HBRUSH) -> INT;
    pub fn InvalidateRect(hWnd: HWND, rect: *const RECT, erase: BOOL) -> BOOL;
    pub fn GetCursorPos(point: *mut POINT) -> BOOL;
    pub fn SetCapture(hWnd: HWND) -> HWND;
    pub fn ReleaseCapture() -> BOOL;
    pub fn DrawTextW(
        hdc: HDC,
        text: *const u16,
        count: INT,
        rect: *mut RECT,
        format: UINT,
    ) -> INT;
    pub fn SetProcessDpiAwarenessContext(value: HANDLE) -> BOOL;
    pub fn SetWindowDisplayAffinity(hWnd: HWND, affinity: DWORD) -> BOOL;
    pub fn MessageBoxW(hWnd: HWND, text: *const u16, caption: *const u16, kind: UINT) -> INT;
    pub fn SetTimer(hWnd: HWND, id: usize, interval: UINT, callback: Option<unsafe extern "system" fn(HWND, UINT, usize, DWORD)>) -> usize;
    pub fn KillTimer(hWnd: HWND, id: usize) -> BOOL;
    pub fn PostQuitMessage(exitCode: INT);
    pub fn OpenClipboard(hWnd: HWND) -> BOOL;
    pub fn CloseClipboard() -> BOOL;
    pub fn EmptyClipboard() -> BOOL;
    pub fn SetClipboardData(format: UINT, memory: HANDLE) -> HANDLE;
    pub fn RegisterClipboardFormatW(format: *const u16) -> UINT;
    pub fn GetDC(hWnd: HWND) -> HDC;
    pub fn ReleaseDC(hWnd: HWND, hdc: HDC) -> INT;
}

#[link(name = "gdi32")]
extern "system" {
    pub fn CreateSolidBrush(color: COLORREF) -> HBRUSH;
    pub fn CreatePen(style: INT, width: INT, color: COLORREF) -> HPEN;
    pub fn SelectObject(hdc: HDC, object: HGDIOBJ) -> HGDIOBJ;
    pub fn DeleteObject(object: HGDIOBJ) -> BOOL;
    pub fn GetStockObject(index: INT) -> HGDIOBJ;
    pub fn Rectangle(hdc: HDC, left: INT, top: INT, right: INT, bottom: INT) -> BOOL;
    pub fn SetTextColor(hdc: HDC, color: COLORREF) -> COLORREF;
    pub fn SetBkMode(hdc: HDC, mode: INT) -> INT;
    pub fn CreateCompatibleDC(hdc: HDC) -> HDC;
    pub fn DeleteDC(hdc: HDC) -> BOOL;
    pub fn CreateCompatibleBitmap(hdc: HDC, width: INT, height: INT) -> HBITMAP;
    pub fn BitBlt(
        hdc: HDC,
        x: INT,
        y: INT,
        width: INT,
        height: INT,
        source: HDC,
        source_x: INT,
        source_y: INT,
        operation: DWORD,
    ) -> BOOL;
    pub fn GetDIBits(
        hdc: HDC,
        bitmap: HBITMAP,
        start: UINT,
        lines: UINT,
        bits: *mut c_void,
        info: *mut BITMAPINFO,
        usage: UINT,
    ) -> INT;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(moduleName: *const u16) -> HINSTANCE;
    pub fn GlobalAlloc(flags: UINT, bytes: usize) -> HGLOBAL;
    pub fn GlobalLock(memory: HGLOBAL) -> *mut c_void;
    pub fn GlobalUnlock(memory: HGLOBAL) -> BOOL;
    pub fn GlobalFree(memory: HGLOBAL) -> HGLOBAL;
    pub fn GetLocalTime(time: *mut SYSTEMTIME);
    pub fn Beep(frequency: DWORD, duration: DWORD) -> BOOL;
}

pub fn wide_null(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[link(name = "shell32")]
extern "system" {
    pub fn ShellExecuteW(
        hWnd: HWND,
        operation: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show: INT,
    ) -> HINSTANCE;
}

// Shell notification area and menu support.
pub type HRESULT = i32;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct GUID {
    pub Data1: u32,
    pub Data2: u16,
    pub Data3: u16,
    pub Data4: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NOTIFYICONDATAW {
    pub cbSize: DWORD,
    pub hWnd: HWND,
    pub uID: UINT,
    pub uFlags: UINT,
    pub uCallbackMessage: UINT,
    pub hIcon: HICON,
    pub szTip: [u16; 128],
    pub dwState: DWORD,
    pub dwStateMask: DWORD,
    pub szInfo: [u16; 256],
    pub uVersion: UINT,
    pub szInfoTitle: [u16; 64],
    pub dwInfoFlags: DWORD,
    pub guidItem: GUID,
    pub hBalloonIcon: HICON,
}

impl Default for NOTIFYICONDATAW {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

pub const WM_NULL: UINT = 0x0000;
pub const WM_COMMAND: UINT = 0x0111;
pub const WM_CONTEXTMENU: UINT = 0x007B;
pub const WM_LBUTTONDBLCLK: UINT = 0x0203;
pub const WM_RBUTTONUP: UINT = 0x0205;
pub const WM_APP: UINT = 0x8000;

pub const NIM_ADD: DWORD = 0x0000_0000;
pub const NIM_MODIFY: DWORD = 0x0000_0001;
pub const NIM_DELETE: DWORD = 0x0000_0002;
pub const NIM_SETVERSION: DWORD = 0x0000_0004;
pub const NIF_MESSAGE: UINT = 0x0000_0001;
pub const NIF_ICON: UINT = 0x0000_0002;
pub const NIF_TIP: UINT = 0x0000_0004;
pub const NIF_SHOWTIP: UINT = 0x0000_0080;
pub const NOTIFYICON_VERSION_4: UINT = 4;

pub const MF_STRING: UINT = 0x0000_0000;
pub const MF_SEPARATOR: UINT = 0x0000_0800;
pub const TPM_RIGHTBUTTON: UINT = 0x0000_0002;
pub const TPM_NONOTIFY: UINT = 0x0000_0080;
pub const TPM_RETURNCMD: UINT = 0x0000_0100;

pub const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;

#[link(name = "user32")]
extern "system" {
    pub fn LoadIconW(instance: HINSTANCE, iconName: *const u16) -> HICON;
    pub fn CreatePopupMenu() -> HMENU;
    pub fn AppendMenuW(menu: HMENU, flags: UINT, item: usize, text: *const u16) -> BOOL;
    pub fn TrackPopupMenu(
        menu: HMENU,
        flags: UINT,
        x: INT,
        y: INT,
        reserved: INT,
        window: HWND,
        rect: *const RECT,
    ) -> BOOL;
    pub fn DestroyMenu(menu: HMENU) -> BOOL;
    pub fn PostMessageW(window: HWND, message: UINT, wparam: WPARAM, lparam: LPARAM) -> BOOL;
}

#[link(name = "shell32")]
extern "system" {
    pub fn Shell_NotifyIconW(message: DWORD, data: *mut NOTIFYICONDATAW) -> BOOL;
    pub fn SetCurrentProcessExplicitAppUserModelID(appId: *const u16) -> HRESULT;
}

pub const ERROR_ALREADY_EXISTS: DWORD = 183;

#[link(name = "user32")]
extern "system" {
    pub fn RegisterWindowMessageW(text: *const u16) -> UINT;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn CreateMutexW(attributes: *mut c_void, initialOwner: BOOL, name: *const u16) -> HANDLE;
    pub fn GetLastError() -> DWORD;
    pub fn CloseHandle(handle: HANDLE) -> BOOL;
}
