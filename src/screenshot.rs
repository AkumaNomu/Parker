use crate::selector::ScreenRect;
use crate::win::*;
use std::fs::File;
use std::io::Write;
use std::mem::size_of;
use std::path::Path;
use std::ptr::null_mut;

pub fn capture_virtual_screen() -> Result<crate::screen_controller::ScreenImage, String> {
    let x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let w = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let h = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    if w <= 0 || h <= 0 {
        return Err("Windows reported an invalid virtual desktop size.".to_string());
    }
    let screen_dc = unsafe { GetDC(null_mut()) };
    if screen_dc.is_null() {
        return Err("Could not access the desktop for screenshot capture.".to_string());
    }
    let memory_dc = unsafe { CreateCompatibleDC(screen_dc) };
    if memory_dc.is_null() {
        unsafe { ReleaseDC(null_mut(), screen_dc); }
        return Err("Could not create an in-memory screenshot surface.".to_string());
    }
    let bitmap = unsafe { CreateCompatibleBitmap(screen_dc, w, h) };
    if bitmap.is_null() {
        unsafe { DeleteDC(memory_dc); ReleaseDC(null_mut(), screen_dc); }
        return Err("Could not allocate the screenshot bitmap.".to_string());
    }
    let previous = unsafe { SelectObject(memory_dc, bitmap as HGDIOBJ) };
    let copied = unsafe { BitBlt(memory_dc, 0, 0, w, h, screen_dc, x, y, SRCCOPY | CAPTUREBLT) };
    unsafe { SelectObject(memory_dc, previous); }
    if copied == 0 {
        unsafe { DeleteObject(bitmap as HGDIOBJ); DeleteDC(memory_dc); ReleaseDC(null_mut(), screen_dc); }
        return Err("Windows could not copy the screen pixels.".to_string());
    }
    let image_size = (w as usize).checked_mul(h as usize).and_then(|p| p.checked_mul(4))
        .ok_or_else(|| "The screenshot is too large.".to_string())?;
    let mut pixels = vec![0u8; image_size];
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: image_size as u32,
            ..BITMAPINFOHEADER::default()
        },
        bmiColors: [RGBQUAD::default(); 1],
    };
    let scan_lines = unsafe { GetDIBits(memory_dc, bitmap, 0, h as u32, pixels.as_mut_ptr() as *mut _, &mut info, DIB_RGB_COLORS) };
    unsafe { DeleteObject(bitmap as HGDIOBJ); DeleteDC(memory_dc); ReleaseDC(null_mut(), screen_dc); }
    if scan_lines == 0 {
        return Err("Could not read the screenshot bitmap pixels.".to_string());
    }
    Ok(crate::screen_controller::ScreenImage { width: w, height: h, pixels })
}

pub fn crop_image(
    img: &crate::screen_controller::ScreenImage,
    region: ScreenRect,
) -> Result<crate::screen_controller::ScreenImage, String> {
    let virtual_x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let virtual_y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let px = (region.x - virtual_x) as usize;
    let py = (region.y - virtual_y) as usize;
    let pw = region.width as usize;
    let ph = region.height as usize;
    if pw == 0 || ph == 0 {
        return Err("The selected screenshot region is empty.".to_string());
    }
    if px + pw > img.width as usize || py + ph > img.height as usize {
        return Err("The selected region extends beyond the captured screen.".to_string());
    }
    let row_bytes = pw * 4;
    let mut cropped = vec![0u8; row_bytes * ph];
    for row in 0..ph {
        let src_start = ((py + row) * img.width as usize + px) * 4;
        let dst_start = row * row_bytes;
        cropped[dst_start..dst_start + row_bytes]
            .copy_from_slice(&img.pixels[src_start..src_start + row_bytes]);
    }
    Ok(crate::screen_controller::ScreenImage { width: pw as i32, height: ph as i32, pixels: cropped })
}

pub fn capture_region_to_bmp(rect: ScreenRect, output: &Path) -> Result<(), String> {
    if rect.width <= 0 || rect.height <= 0 {
        return Err("The selected screenshot region is empty.".to_string());
    }

    let screen_dc = unsafe { GetDC(null_mut()) };
    if screen_dc.is_null() {
        return Err("Could not access the desktop for screenshot capture.".to_string());
    }

    let memory_dc = unsafe { CreateCompatibleDC(screen_dc) };
    if memory_dc.is_null() {
        unsafe {
            ReleaseDC(null_mut(), screen_dc);
        }
        return Err("Could not create an in-memory screenshot surface.".to_string());
    }

    let bitmap = unsafe { CreateCompatibleBitmap(screen_dc, rect.width, rect.height) };
    if bitmap.is_null() {
        unsafe {
            DeleteDC(memory_dc);
            ReleaseDC(null_mut(), screen_dc);
        }
        return Err("Could not allocate the screenshot bitmap.".to_string());
    }

    let previous = unsafe { SelectObject(memory_dc, bitmap as HGDIOBJ) };
    let copied = unsafe {
        BitBlt(
            memory_dc,
            0,
            0,
            rect.width,
            rect.height,
            screen_dc,
            rect.x,
            rect.y,
            SRCCOPY | CAPTUREBLT,
        )
    };
    unsafe {
        SelectObject(memory_dc, previous);
    }

    if copied == 0 {
        unsafe {
            DeleteObject(bitmap as HGDIOBJ);
            DeleteDC(memory_dc);
            ReleaseDC(null_mut(), screen_dc);
        }
        return Err("Windows could not copy the selected pixels.".to_string());
    }

    let image_size = (rect.width as usize)
        .checked_mul(rect.height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "The selected screenshot is too large.".to_string())?;
    let mut pixels = vec![0u8; image_size];
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: rect.width,
            biHeight: -rect.height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: image_size as u32,
            ..BITMAPINFOHEADER::default()
        },
        bmiColors: [RGBQUAD::default(); 1],
    };

    let scan_lines = unsafe {
        GetDIBits(
            memory_dc,
            bitmap,
            0,
            rect.height as u32,
            pixels.as_mut_ptr() as *mut _,
            &mut info,
            DIB_RGB_COLORS,
        )
    };

    unsafe {
        DeleteObject(bitmap as HGDIOBJ);
        DeleteDC(memory_dc);
        ReleaseDC(null_mut(), screen_dc);
    }

    if scan_lines == 0 {
        return Err("Could not read the screenshot bitmap pixels.".to_string());
    }

    write_bmp(output, rect.width, rect.height, &pixels)
}

pub fn write_bmp(path: &Path, width: i32, height: i32, pixels: &[u8]) -> Result<(), String> {
    let header_size = 14u32 + 40u32;
    let file_size = header_size
        .checked_add(pixels.len() as u32)
        .ok_or_else(|| "The screenshot bitmap is too large to write.".to_string())?;

    let mut file = File::create(path)
        .map_err(|error| format!("Could not create {}: {error}", path.display()))?;

    file.write_all(b"BM")
        .and_then(|_| file.write_all(&file_size.to_le_bytes()))
        .and_then(|_| file.write_all(&0u16.to_le_bytes()))
        .and_then(|_| file.write_all(&0u16.to_le_bytes()))
        .and_then(|_| file.write_all(&header_size.to_le_bytes()))
        .and_then(|_| file.write_all(&40u32.to_le_bytes()))
        .and_then(|_| file.write_all(&width.to_le_bytes()))
        .and_then(|_| file.write_all(&(-height).to_le_bytes()))
        .and_then(|_| file.write_all(&1u16.to_le_bytes()))
        .and_then(|_| file.write_all(&32u16.to_le_bytes()))
        .and_then(|_| file.write_all(&BI_RGB.to_le_bytes()))
        .and_then(|_| file.write_all(&(pixels.len() as u32).to_le_bytes()))
        .and_then(|_| file.write_all(&0i32.to_le_bytes()))
        .and_then(|_| file.write_all(&0i32.to_le_bytes()))
        .and_then(|_| file.write_all(&0u32.to_le_bytes()))
        .and_then(|_| file.write_all(&0u32.to_le_bytes()))
        .and_then(|_| file.write_all(pixels))
        .map_err(|error| format!("Could not write {}: {error}", path.display()))
}
