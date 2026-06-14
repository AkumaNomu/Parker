use crate::win::*;
use image::imageops::{resize, FilterType};
use image::GrayImage;
use std::path::Path;
use std::ptr::null_mut;

pub fn detect(path: &Path) -> Result<Vec<String>, String> {
    let image = image::open(path)
        .map_err(|error| format!("Could not read the selected image for QR detection: {error}"))?
        .to_luma8();

    let mut payloads = decode_image(image.clone());

    // Small QR codes often become easier to localize after a nearest-neighbour
    // upscale. Only retry when the first pass found nothing, which keeps the
    // common path fast.
    if payloads.is_empty() && image.width() < 1800 && image.height() < 1800 {
        let scaled = resize(
            &image,
            image.width().saturating_mul(2),
            image.height().saturating_mul(2),
            FilterType::Nearest,
        );
        payloads = decode_image(scaled);
    }

    Ok(payloads)
}

fn decode_image(image: GrayImage) -> Vec<String> {
    let mut prepared = rqrr::PreparedImage::prepare(image);
    let grids = prepared.detect_grids();
    let mut payloads = Vec::new();

    for grid in grids {
        if let Ok((_, payload)) = grid.decode() {
            let payload = payload.trim().to_string();
            if !payload.is_empty() && !payloads.iter().any(|item| item == &payload) {
                payloads.push(payload);
            }
        }
    }

    payloads
}

pub fn first_web_url(payloads: &[String]) -> Option<&str> {
    payloads
        .iter()
        .map(String::as_str)
        .find(|payload| is_safe_web_url(payload))
}

pub fn open_web_url(url: &str) -> Result<(), String> {
    if !is_safe_web_url(url) {
        return Err("Only HTTP and HTTPS QR links can be opened automatically.".to_string());
    }

    let operation = wide_null("open");
    let target = wide_null(url);
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            null_mut(),
            null_mut(),
            SW_SHOWNORMAL,
        )
    } as isize;

    if result <= 32 {
        Err(format!(
            "Windows could not open the detected QR link (ShellExecute error {result})."
        ))
    } else {
        Ok(())
    }
}

fn is_safe_web_url(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.len() > 4096
        || trimmed
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        return false;
    }

    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("https://") || lower.starts_with("http://")
}

#[cfg(test)]
mod tests {
    use super::is_safe_web_url;

    #[test]
    fn accepts_http_urls_only() {
        assert!(is_safe_web_url("https://example.com/path"));
        assert!(is_safe_web_url("http://localhost:3000"));
        assert!(!is_safe_web_url("javascript:alert(1)"));
        assert!(!is_safe_web_url("https://example.com/a b"));
    }
}
