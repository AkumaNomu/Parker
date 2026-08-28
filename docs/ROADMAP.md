# Roadmap

## High-value workflow additions

1. Searchable local capture and clipboard history.
3. System-audio and microphone recording profiles beyond the opt-in device
   passthrough.
4. Active-window recording that follows window movement.
5. Pause, resume, and discard controls for recordings.
6. Annotation and redaction before copying an image.
7. Scrolling capture for long pages and conversations.
8. Local speech transcription for completed recordings.
9. User-defined post-capture commands and application-specific routing rules.

Shipped recently: shared table/code/text OCR classification on both platforms,
Linux/Windows image-to-clipboard (`shot` / `Ctrl+Shift+F9`), X11
capture/clipboard fallbacks, VAAPI encoding, curl-based self-update,
multi-distro install commands, and screenshot-prioritized hotkeys
(`F8` capture, `F9` screenshot, `F11` record) (see CHANGELOG).

## Detection improvements

- Better table reconstruction for merged cells and right-aligned numeric data.
- Syntax-specific OCR correction dictionaries for common programming languages.
- Barcode formats beyond QR.
- Equation-to-LaTeX recognition through an optional specialized engine.
- URL, email, file-path, calendar-date, and error-stack actions.
- Optional confirmation before opening QR links from untrusted sources.

## Platform work

- Windows Graphics Capture or Desktop Duplication for hardware-accelerated
  recording.
- Per-monitor toast placement and stacked notifications.
- Signed MSIX release variant.
- Automatic update checks with explicit user consent.
- GNOME Wayland recording via the ScreenCast portal.
- Fix tray/dashboard hotkey labels to reflect `PARKER_HOTKEY_*` overrides;
  destroy tray icons on state changes; clean up GDI handles on error paths
  flagged in the 2026-08 audit.
