use crate::win::*;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr::copy_nonoverlapping;
use std::thread;
use std::time::Duration;

struct ClipboardGuard;

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}

pub fn copy_file(path: &Path, owner: HWND) -> Result<(), String> {
    let absolute = path
        .canonicalize()
        .map_err(|error| format!("Could not resolve the recording path: {error}"))?;
    let _guard = open_and_clear(owner)?;

    let drop_memory = allocate_dropfiles(&absolute)?;
    let result = unsafe { SetClipboardData(CF_HDROP, drop_memory) };
    if result.is_null() {
        unsafe {
            GlobalFree(drop_memory);
        }
        return Err("Could not place the recording file on the clipboard.".to_string());
    }

    let preferred_drop_effect = wide_null("Preferred DropEffect");
    let effect_format = unsafe { RegisterClipboardFormatW(preferred_drop_effect.as_ptr()) };
    if effect_format != 0 {
        if let Ok(effect_memory) = allocate_u32(1) {
            if unsafe { SetClipboardData(effect_format, effect_memory) }.is_null() {
                unsafe {
                    GlobalFree(effect_memory);
                }
            }
        }
    }

    Ok(())
}

pub fn copy_text(text: &str, owner: HWND) -> Result<(), String> {
    let _guard = open_and_clear(owner)?;
    let normalized = text
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\n', "\r\n");
    let mut utf16: Vec<u16> = normalized.encode_utf16().collect();
    utf16.push(0);
    let byte_count = utf16
        .len()
        .checked_mul(size_of::<u16>())
        .ok_or_else(|| "The recognized text is too large for the clipboard.".to_string())?;

    let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, byte_count) };
    if memory.is_null() {
        return Err("Could not allocate clipboard memory for recognized text.".to_string());
    }

    let pointer = unsafe { GlobalLock(memory) } as *mut u16;
    if pointer.is_null() {
        unsafe {
            GlobalFree(memory);
        }
        return Err("Could not lock clipboard memory for recognized text.".to_string());
    }

    unsafe {
        copy_nonoverlapping(utf16.as_ptr(), pointer, utf16.len());
        GlobalUnlock(memory);
    }

    if unsafe { SetClipboardData(CF_UNICODETEXT, memory) }.is_null() {
        unsafe {
            GlobalFree(memory);
        }
        return Err("Could not place recognized text on the clipboard.".to_string());
    }

    Ok(())
}

fn open_and_clear(owner: HWND) -> Result<ClipboardGuard, String> {
    let mut opened = false;
    for _ in 0..20 {
        if unsafe { OpenClipboard(owner) } != 0 {
            opened = true;
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }

    if !opened {
        return Err("The Windows clipboard is busy.".to_string());
    }

    let guard = ClipboardGuard;
    if unsafe { EmptyClipboard() } == 0 {
        return Err("Could not clear the Windows clipboard.".to_string());
    }

    Ok(guard)
}

fn allocate_dropfiles(path: &Path) -> Result<HGLOBAL, String> {
    let mut path_utf16: Vec<u16> = path.as_os_str().encode_wide().collect();
    path_utf16.push(0);
    path_utf16.push(0);

    let header_size = size_of::<DROPFILES>();
    let path_size = path_utf16.len() * size_of::<u16>();
    let total_size = header_size + path_size;

    let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, total_size) };
    if memory.is_null() {
        return Err("Could not allocate clipboard memory.".to_string());
    }

    let pointer = unsafe { GlobalLock(memory) } as *mut u8;
    if pointer.is_null() {
        unsafe {
            GlobalFree(memory);
        }
        return Err("Could not lock clipboard memory.".to_string());
    }

    let header = DROPFILES {
        pFiles: header_size as u32,
        pt: POINT { x: 0, y: 0 },
        fNC: FALSE,
        fWide: TRUE,
    };

    unsafe {
        std::ptr::write(pointer as *mut DROPFILES, header);
        copy_nonoverlapping(
            path_utf16.as_ptr() as *const u8,
            pointer.add(header_size),
            path_size,
        );
        GlobalUnlock(memory);
    }

    Ok(memory)
}

fn allocate_u32(value: u32) -> Result<HGLOBAL, String> {
    let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, size_of::<u32>()) };
    if memory.is_null() {
        return Err("Could not allocate clipboard effect memory.".to_string());
    }

    let pointer = unsafe { GlobalLock(memory) } as *mut u32;
    if pointer.is_null() {
        unsafe {
            GlobalFree(memory);
        }
        return Err("Could not lock clipboard effect memory.".to_string());
    }

    unsafe {
        pointer.write(value);
        GlobalUnlock(memory);
    }

    Ok(memory)
}
