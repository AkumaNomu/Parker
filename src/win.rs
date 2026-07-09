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
pub type SHORT = i16;
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
pub type HFONT = HANDLE;
pub type HGLOBAL = HANDLE;
pub type HMENU = HANDLE;
pub type HRGN = HANDLE;
pub type HMONITOR = HANDLE;
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
pub type HOOKPROC = Option<unsafe extern "system" fn(INT, WPARAM, LPARAM) -> LRESULT>;

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
#[derive(Clone, Copy)]
pub struct MONITORINFO {
    pub cbSize: DWORD,
    pub rcMonitor: RECT,
    pub rcWork: RECT,
    pub dwFlags: DWORD,
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
pub const WM_CLOSE: UINT = 0x0010;
pub const WM_PAINT: UINT = 0x000F;
pub const WM_KEYDOWN: UINT = 0x0100;
pub const WM_TIMER: UINT = 0x0113;
pub const WM_HOTKEY: UINT = 0x0312;
pub const WM_MOUSEMOVE: UINT = 0x0200;
pub const WM_LBUTTONDOWN: UINT = 0x0201;
pub const WM_LBUTTONUP: UINT = 0x0202;
pub const WM_RBUTTONDOWN: UINT = 0x0204;
pub const WH_KEYBOARD_LL: INT = 13;
pub const WH_MOUSE_LL: INT = 14;
pub const WM_QUIT: UINT = 0x0012;

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
pub const SWP_NOSIZE: UINT = 0x0001;
pub const SWP_NOACTIVATE: UINT = 0x0010;

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
pub const FW_NORMAL: INT = 400;
pub const FW_SEMIBOLD: INT = 600;
pub const DEFAULT_CHARSET: DWORD = 1;
pub const CLEARTYPE_QUALITY: DWORD = 5;
pub const DEFAULT_PITCH: DWORD = 0;

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
pub const MONITOR_DEFAULTTONEAREST: DWORD = 2;
pub const MONITORINFOF_PRIMARY: DWORD = 1;
pub const MOUSEEVENTF_WHEEL: DWORD = 0x0800;

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
    pub fn SystemParametersInfoW(
        action: UINT,
        param: UINT,
        value: *mut c_void,
        flags: UINT,
    ) -> BOOL;
    pub fn LoadCursorW(instance: HINSTANCE, cursorName: *const u16) -> HCURSOR;
    pub fn BeginPaint(hWnd: HWND, paint: *mut PAINTSTRUCT) -> HDC;
    pub fn EndPaint(hWnd: HWND, paint: *const PAINTSTRUCT) -> BOOL;
    pub fn GetClientRect(hWnd: HWND, rect: *mut RECT) -> BOOL;
    pub fn FillRect(hdc: HDC, rect: *const RECT, brush: HBRUSH) -> INT;
    pub fn InvalidateRect(hWnd: HWND, rect: *const RECT, erase: BOOL) -> BOOL;
    pub fn GetCursorPos(point: *mut POINT) -> BOOL;
    pub fn MonitorFromPoint(pt: POINT, flags: DWORD) -> HMONITOR;
    pub fn GetMonitorInfoW(monitor: HMONITOR, info: *mut MONITORINFO) -> BOOL;
    pub fn GetAsyncKeyState(vKey: INT) -> SHORT;
    pub fn SetCapture(hWnd: HWND) -> HWND;
    pub fn ReleaseCapture() -> BOOL;
    pub fn mouse_event(dwFlags: DWORD, dx: DWORD, dy: DWORD, dwData: DWORD, extraInfo: usize);
    pub fn DrawTextW(hdc: HDC, text: *const u16, count: INT, rect: *mut RECT, format: UINT) -> INT;
    pub fn SetProcessDpiAwarenessContext(value: HANDLE) -> BOOL;
    pub fn SetWindowDisplayAffinity(hWnd: HWND, affinity: DWORD) -> BOOL;
    pub fn AddClipboardFormatListener(hWnd: HWND) -> BOOL;
    pub fn RemoveClipboardFormatListener(hWnd: HWND) -> BOOL;
    pub fn SetWindowPos(
        hWnd: HWND,
        insertAfter: HWND,
        x: INT,
        y: INT,
        width: INT,
        height: INT,
        flags: UINT,
    ) -> BOOL;
    pub fn GetWindowRect(hWnd: HWND, rect: *mut RECT) -> BOOL;
    pub fn SetWindowRgn(hWnd: HWND, region: HRGN, redraw: BOOL) -> INT;
    pub fn MessageBoxW(hWnd: HWND, text: *const u16, caption: *const u16, kind: UINT) -> INT;
    pub fn SetTimer(
        hWnd: HWND,
        id: usize,
        interval: UINT,
        callback: Option<unsafe extern "system" fn(HWND, UINT, usize, DWORD)>,
    ) -> usize;
    pub fn KillTimer(hWnd: HWND, id: usize) -> BOOL;
    pub fn PostQuitMessage(exitCode: INT);
    pub fn OpenClipboard(hWnd: HWND) -> BOOL;
    pub fn CloseClipboard() -> BOOL;
    pub fn EmptyClipboard() -> BOOL;
    pub fn SetClipboardData(format: UINT, memory: HANDLE) -> HANDLE;
    pub fn RegisterClipboardFormatW(format: *const u16) -> UINT;
    pub fn IsClipboardFormatAvailable(format: UINT) -> BOOL;
    pub fn GetClipboardData(format: UINT) -> HANDLE;
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
    pub fn RoundRect(
        hdc: HDC,
        left: INT,
        top: INT,
        right: INT,
        bottom: INT,
        ellipseWidth: INT,
        ellipseHeight: INT,
    ) -> BOOL;
    pub fn Ellipse(hdc: HDC, left: INT, top: INT, right: INT, bottom: INT) -> BOOL;
    pub fn CreateRoundRectRgn(
        left: INT,
        top: INT,
        right: INT,
        bottom: INT,
        ellipseWidth: INT,
        ellipseHeight: INT,
    ) -> HRGN;
    pub fn CreateFontW(
        height: INT,
        width: INT,
        escapement: INT,
        orientation: INT,
        weight: INT,
        italic: DWORD,
        underline: DWORD,
        strikeOut: DWORD,
        charSet: DWORD,
        outPrecision: DWORD,
        clipPrecision: DWORD,
        quality: DWORD,
        pitchAndFamily: DWORD,
        face: *const u16,
    ) -> HFONT;
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
pub const MF_GRAYED: UINT = 0x0000_0001;
pub const MF_SEPARATOR: UINT = 0x0000_0800;
pub const TPM_RIGHTBUTTON: UINT = 0x0000_0002;
pub const TPM_NONOTIFY: UINT = 0x0000_0080;
pub const TPM_RETURNCMD: UINT = 0x0000_0100;

pub const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;
pub const COINIT_APARTMENTTHREADED: DWORD = 0x2;
pub const CLSCTX_LOCAL_SERVER: DWORD = 4;
pub const S_OK: HRESULT = 0;
pub const WM_CLIPBOARDUPDATE: UINT = 0x031D;

#[repr(C)]
pub struct IUnknownVtbl {
    pub QueryInterface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(*mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(*mut c_void) -> u32,
}

#[repr(C)]
pub struct VirtualDesktopManagerVtbl {
    pub query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub is_window_on_current: unsafe extern "system" fn(*mut c_void, HWND, *mut BOOL) -> HRESULT,
    pub get_window_desktop_id: unsafe extern "system" fn(*mut c_void, HWND, *mut GUID) -> HRESULT,
    pub move_window_to_desktop: unsafe extern "system" fn(*mut c_void, HWND, *const GUID) -> HRESULT,
}

#[allow(non_upper_case_globals)]
pub static CLSID_VirtualDesktopManager: GUID = GUID {
    Data1: 0xAA509086,
    Data2: 0x5CA9,
    Data3: 0x4C0C,
    Data4: [0x9B, 0x8C, 0x9C, 0x4B, 0x2B, 0xC5, 0xB5, 0xA5],
};

#[allow(non_upper_case_globals)]
pub static IID_IVirtualDesktopManager: GUID = GUID {
    Data1: 0xA5CD92FF,
    Data2: 0x29BE,
    Data3: 0x454C,
    Data4: [0x8D, 0x04, 0xD8, 0x28, 0x79, 0xFB, 0x3F, 0x1B],
};

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

#[link(name = "user32")]
extern "system" {
    pub fn SetWindowsHookExW(
        idHook: INT,
        lpfn: HOOKPROC,
        hMod: HINSTANCE,
        dwThreadId: DWORD,
    ) -> HANDLE;
    pub fn UnhookWindowsHookEx(hhk: HANDLE) -> BOOL;
    pub fn CallNextHookEx(hhk: HANDLE, nCode: INT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn GetCurrentThreadId() -> DWORD;
    pub fn PostThreadMessageW(idThread: DWORD, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> BOOL;
}

#[link(name = "shell32")]
extern "system" {
    pub fn Shell_NotifyIconW(message: DWORD, data: *mut NOTIFYICONDATAW) -> BOOL;
    pub fn SetCurrentProcessExplicitAppUserModelID(appId: *const u16) -> HRESULT;
}

#[link(name = "ole32")]
extern "system" {
    pub fn CoInitializeEx(reserved: *mut c_void, dwCoinit: DWORD) -> HRESULT;
    pub fn CoUninitialize();
    pub fn CoCreateInstance(
        rclsid: *const GUID,
        pUnkOuter: *mut c_void,
        dwClsContext: DWORD,
        riid: *const GUID,
        ppv: *mut *mut c_void,
    ) -> HRESULT;
}

pub const ERROR_ALREADY_EXISTS: DWORD = 183;

#[link(name = "user32")]
extern "system" {
    pub fn RegisterWindowMessageW(text: *const u16) -> UINT;
}

// ── Input simulation (SendInput) ───────────────────────────────────────
#[repr(C)]
pub struct MOUSEINPUT {
    pub dx: LONG,
    pub dy: LONG,
    pub mouseData: DWORD,
    pub dwFlags: DWORD,
    pub time: DWORD,
    pub dwExtraInfo: usize,
}

#[repr(C)]
pub struct KEYBDINPUT {
    pub wVk: WORD,
    pub wScan: WORD,
    pub dwFlags: DWORD,
    pub time: DWORD,
    pub dwExtraInfo: usize,
}

#[repr(C)]
pub struct HARDWAREINPUT {
    pub uMsg: DWORD,
    pub wParamL: WORD,
    pub wParamH: WORD,
}

#[repr(C)]
pub union INPUT_UNION {
    pub mi: std::mem::ManuallyDrop<MOUSEINPUT>,
    pub ki: std::mem::ManuallyDrop<KEYBDINPUT>,
    pub hi: std::mem::ManuallyDrop<HARDWAREINPUT>,
}

#[repr(C)]
pub struct INPUT {
    pub type_: DWORD,
    pub u: INPUT_UNION,
}

impl Default for INPUT {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

impl Default for INPUT_UNION {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

impl Default for MOUSEINPUT {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

impl Default for KEYBDINPUT {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

impl Default for HARDWAREINPUT {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

pub const INPUT_MOUSE: DWORD = 0;
pub const INPUT_KEYBOARD: DWORD = 1;

pub const MOUSEEVENTF_MOVE: DWORD = 0x0001;
pub const MOUSEEVENTF_LEFTDOWN: DWORD = 0x0002;
pub const MOUSEEVENTF_LEFTUP: DWORD = 0x0004;
pub const MOUSEEVENTF_RIGHTDOWN: DWORD = 0x0008;
pub const MOUSEEVENTF_RIGHTUP: DWORD = 0x0010;
pub const MOUSEEVENTF_MIDDLEDOWN: DWORD = 0x0020;
pub const MOUSEEVENTF_MIDDLEUP: DWORD = 0x0040;
pub const MOUSEEVENTF_ABSOLUTE: DWORD = 0x8000;

pub const KEYEVENTF_KEYUP: DWORD = 0x0002;
pub const KEYEVENTF_SCANCODE: DWORD = 0x0008;
pub const KEYEVENTF_EXTENDEDKEY: DWORD = 0x0001;

// ── Virtual-key codes ─────────────────────────────────────────────────
pub const VK_BACK: WORD = 0x08;
pub const VK_TAB: WORD = 0x09;
pub const VK_RETURN: WORD = 0x0D;
pub const VK_SHIFT: WORD = 0x10;
pub const VK_CONTROL: WORD = 0x11;
pub const VK_MENU: WORD = 0x12; // Alt
pub const VK_PAUSE: WORD = 0x13;
pub const VK_CAPITAL: WORD = 0x14; // Caps Lock
pub const VK_ESCAPE: WORD = 0x1B;
pub const VK_SPACE: WORD = 0x20;
pub const VK_PRIOR: WORD = 0x21; // Page Up
pub const VK_NEXT: WORD = 0x22;  // Page Down
pub const VK_END: WORD = 0x23;
pub const VK_HOME: WORD = 0x24;
pub const VK_LEFT: WORD = 0x25;
pub const VK_UP: WORD = 0x26;
pub const VK_RIGHT: WORD = 0x27;
pub const VK_DOWN: WORD = 0x28;
pub const VK_SNAPSHOT: WORD = 0x2C; // Print Screen
pub const VK_INSERT: WORD = 0x2D;
pub const VK_DELETE: WORD = 0x2E;
pub const VK_LWIN: WORD = 0x5B;
pub const VK_RWIN: WORD = 0x5C;
pub const VK_NUMPAD0: WORD = 0x60;
pub const VK_NUMPAD1: WORD = 0x61;
pub const VK_NUMPAD2: WORD = 0x62;
pub const VK_NUMPAD3: WORD = 0x63;
pub const VK_NUMPAD4: WORD = 0x64;
pub const VK_NUMPAD5: WORD = 0x65;
pub const VK_NUMPAD6: WORD = 0x66;
pub const VK_NUMPAD7: WORD = 0x67;
pub const VK_NUMPAD8: WORD = 0x68;
pub const VK_NUMPAD9: WORD = 0x69;
pub const VK_MULTIPLY: WORD = 0x6A;
pub const VK_ADD: WORD = 0x6B;
pub const VK_SEPARATOR: WORD = 0x6C;
pub const VK_SUBTRACT: WORD = 0x6D;
pub const VK_DECIMAL: WORD = 0x6E;
pub const VK_DIVIDE: WORD = 0x6F;
pub const VK_F1: WORD = 0x70;
pub const VK_F2: WORD = 0x71;
pub const VK_F3: WORD = 0x72;
pub const VK_F4: WORD = 0x73;
pub const VK_F5: WORD = 0x74;
pub const VK_F6: WORD = 0x75;
pub const VK_F7: WORD = 0x76;
pub const VK_F8: WORD = 0x77;
pub const VK_F9: WORD = 0x78;
pub const VK_F10: WORD = 0x79;
pub const VK_F11: WORD = 0x7A;
pub const VK_F12: WORD = 0x7B;
pub const VK_NONLOCK: WORD = 0x90; // Num Lock
pub const VK_SCROLL: WORD = 0x91;
pub const VK_LSHIFT: WORD = 0xA0;
pub const VK_RSHIFT: WORD = 0xA1;
pub const VK_LCONTROL: WORD = 0xA2;
pub const VK_RCONTROL: WORD = 0xA3;
pub const VK_LMENU: WORD = 0xA4;
pub const VK_RMENU: WORD = 0xA5;
pub const VK_OEM_1: WORD = 0xBA;    // ;:
pub const VK_OEM_PLUS: WORD = 0xBB; // =+
pub const VK_OEM_COMMA: WORD = 0xBC;
pub const VK_OEM_MINUS: WORD = 0xBD; // -_
pub const VK_OEM_PERIOD: WORD = 0xBE;
pub const VK_OEM_2: WORD = 0xBF;    // /?
pub const VK_OEM_3: WORD = 0xC0;    // `~
pub const VK_OEM_4: WORD = 0xDB;    // [{
pub const VK_OEM_5: WORD = 0xDC;    // \|
pub const VK_OEM_6: WORD = 0xDD;    // ]}
pub const VK_OEM_7: WORD = 0xDE;    // '"

pub const VK_A: WORD = 0x41;
pub const VK_B: WORD = 0x42;
pub const VK_C: WORD = 0x43;
pub const VK_D: WORD = 0x44;
pub const VK_E: WORD = 0x45;
pub const VK_F: WORD = 0x46;
pub const VK_G: WORD = 0x47;
pub const VK_H: WORD = 0x48;
pub const VK_I: WORD = 0x49;
pub const VK_J: WORD = 0x4A;
pub const VK_K: WORD = 0x4B;
pub const VK_L: WORD = 0x4C;
pub const VK_M: WORD = 0x4D;
pub const VK_N: WORD = 0x4E;
pub const VK_O: WORD = 0x4F;
pub const VK_P: WORD = 0x50;
pub const VK_Q: WORD = 0x51;
pub const VK_R: WORD = 0x52;
pub const VK_S: WORD = 0x53;
pub const VK_T: WORD = 0x54;
pub const VK_U: WORD = 0x55;
pub const VK_V: WORD = 0x56;
pub const VK_W: WORD = 0x57;
pub const VK_X: WORD = 0x58;
pub const VK_Y: WORD = 0x59;
pub const VK_Z: WORD = 0x5A;

pub const VK_0: WORD = 0x30;
pub const VK_1: WORD = 0x31;
pub const VK_2: WORD = 0x32;
pub const VK_3: WORD = 0x33;
pub const VK_4: WORD = 0x34;
pub const VK_5: WORD = 0x35;
pub const VK_6: WORD = 0x36;
pub const VK_7: WORD = 0x37;
pub const VK_8: WORD = 0x38;
pub const VK_9: WORD = 0x39;

// US keyboard scan codes for SendInput with KEYEVENTF_SCANCODE
// Table maps printable ASCII characters to (scan_code, shift_needed)
pub fn char_to_key(c: char) -> Option<(WORD, WORD, bool)> {
    // Returns (virtual_key, scan_code, shift_needed)
    Some(match c {
        'a' | 'A' => (VK_A, 0x1E, c.is_uppercase()),
        'b' | 'B' => (VK_B, 0x30, c.is_uppercase()),
        'c' | 'C' => (VK_C, 0x2E, c.is_uppercase()),
        'd' | 'D' => (VK_D, 0x20, c.is_uppercase()),
        'e' | 'E' => (VK_E, 0x12, c.is_uppercase()),
        'f' | 'F' => (VK_F, 0x21, c.is_uppercase()),
        'g' | 'G' => (VK_G, 0x22, c.is_uppercase()),
        'h' | 'H' => (VK_H, 0x23, c.is_uppercase()),
        'i' | 'I' => (VK_I, 0x17, c.is_uppercase()),
        'j' | 'J' => (VK_J, 0x24, c.is_uppercase()),
        'k' | 'K' => (VK_K, 0x25, c.is_uppercase()),
        'l' | 'L' => (VK_L, 0x26, c.is_uppercase()),
        'm' | 'M' => (VK_M, 0x32, c.is_uppercase()),
        'n' | 'N' => (VK_N, 0x31, c.is_uppercase()),
        'o' | 'O' => (VK_O, 0x18, c.is_uppercase()),
        'p' | 'P' => (VK_P, 0x19, c.is_uppercase()),
        'q' | 'Q' => (VK_Q, 0x10, c.is_uppercase()),
        'r' | 'R' => (VK_R, 0x13, c.is_uppercase()),
        's' | 'S' => (VK_S, 0x1F, c.is_uppercase()),
        't' | 'T' => (VK_T, 0x14, c.is_uppercase()),
        'u' | 'U' => (VK_U, 0x16, c.is_uppercase()),
        'v' | 'V' => (VK_V, 0x2F, c.is_uppercase()),
        'w' | 'W' => (VK_W, 0x11, c.is_uppercase()),
        'x' | 'X' => (VK_X, 0x2D, c.is_uppercase()),
        'y' | 'Y' => (VK_Y, 0x15, c.is_uppercase()),
        'z' | 'Z' => (VK_Z, 0x2C, c.is_uppercase()),
        '0' => (VK_0, 0x0B, false),
        '1' => (VK_1, 0x02, false),
        '2' => (VK_2, 0x03, false),
        '3' => (VK_3, 0x04, false),
        '4' => (VK_4, 0x05, false),
        '5' => (VK_5, 0x06, false),
        '6' => (VK_6, 0x07, false),
        '7' => (VK_7, 0x08, false),
        '8' => (VK_8, 0x09, false),
        '9' => (VK_9, 0x0A, false),
        '`' => (VK_OEM_3, 0x29, false),
        '~' => (VK_OEM_3, 0x29, true),
        '-' => (VK_OEM_MINUS, 0x0C, false),
        '_' => (VK_OEM_MINUS, 0x0C, true),
        '=' => (VK_OEM_PLUS, 0x0D, false),
        '+' => (VK_OEM_PLUS, 0x0D, true),
        '[' => (VK_OEM_4, 0x1A, false),
        '{' => (VK_OEM_4, 0x1A, true),
        ']' => (VK_OEM_6, 0x1B, false),
        '}' => (VK_OEM_6, 0x1B, true),
        '\\' => (VK_OEM_5, 0x2B, false),
        '|' => (VK_OEM_5, 0x2B, true),
        ';' => (VK_OEM_1, 0x27, false),
        ':' => (VK_OEM_1, 0x27, true),
        '\'' => (VK_OEM_7, 0x28, false),
        '"' => (VK_OEM_7, 0x28, true),
        ',' => (VK_OEM_COMMA, 0x33, false),
        '<' => (VK_OEM_COMMA, 0x33, true),
        '.' => (VK_OEM_PERIOD, 0x34, false),
        '>' => (VK_OEM_PERIOD, 0x34, true),
        '/' => (VK_OEM_2, 0x35, false),
        '?' => (VK_OEM_2, 0x35, true),
        '!' => (VK_1, 0x02, true),
        '@' => (VK_2, 0x03, true),
        '#' => (VK_3, 0x04, true),
        '$' => (VK_4, 0x05, true),
        '%' => (VK_5, 0x06, true),
        '^' => (VK_6, 0x07, true),
        '&' => (VK_7, 0x08, true),
        '*' => (VK_8, 0x09, true),
        '(' => (VK_9, 0x0A, true),
        ')' => (VK_0, 0x0B, true),
        ' ' => (VK_SPACE, 0x39, false),
        '\t' => (VK_TAB, 0x0F, false),
        '\n' | '\r' => (VK_RETURN, 0x1C, false),
        _ => return None,
    })
}

// ── Window enumeration ────────────────────────────────────────────────
pub type EnumWindowsProc = Option<unsafe extern "system" fn(HWND, LPARAM) -> BOOL>;

#[link(name = "user32")]
extern "system" {
    pub fn SendInput(cInputs: UINT, pInputs: *mut INPUT, cbSize: INT) -> UINT;
    pub fn EnumWindows(lpEnumFunc: EnumWindowsProc, lParam: LPARAM) -> BOOL;
    pub fn EnumChildWindows(hWndParent: HWND, lpEnumFunc: EnumWindowsProc, lParam: LPARAM) -> BOOL;
    pub fn GetWindowTextW(hWnd: HWND, lpString: *mut u16, nMaxCount: INT) -> INT;
    pub fn GetWindowTextLengthW(hWnd: HWND) -> INT;
    pub fn FindWindowW(lpClassName: *const u16, lpWindowName: *const u16) -> HWND;
    pub fn FindWindowExW(hWndParent: HWND, hWndChildAfter: HWND, lpszClass: *const u16, lpszWindow: *const u16) -> HWND;
    pub fn GetClassNameW(hWnd: HWND, lpClassName: *mut u16, nMaxCount: INT) -> INT;
    pub fn IsWindowVisible(hWnd: HWND) -> BOOL;
    pub fn ClientToScreen(hWnd: HWND, lpPoint: *mut POINT) -> BOOL;
    pub fn ScreenToClient(hWnd: HWND, lpPoint: *mut POINT) -> BOOL;
    pub fn GetWindowLongW(hWnd: HWND, nIndex: INT) -> LONG;
    pub fn GetWindowThreadProcessId(hWnd: HWND, lpdwProcessId: *mut DWORD) -> DWORD;
    pub fn SetCursorPos(x: INT, y: INT) -> BOOL;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn CreateMutexW(attributes: *mut c_void, initialOwner: BOOL, name: *const u16) -> HANDLE;
    pub fn GetLastError() -> DWORD;
    pub fn CloseHandle(handle: HANDLE) -> BOOL;
}
