# Changelog

All notable changes to Parker are documented here.

## [0.4.0] - 2026-06-15

### Added

- Persistent draggable recording timer with a direct stop control.
- Capture exclusion and outside-region placement for Parker's recording control.
- One-click self-extracting Windows setup EXE and standalone portable EXE release assets.

### Fixed

- Rust 1.93 type inference and strict Clippy compatibility in OCR and tray code.
- Installed-app version now follows the packaged application version.

## [0.3.0] - 2026-06-13

### Added

- Automatic QR detection before OCR, including safe HTTP/HTTPS opening.
- Code-aware OCR and table-to-TSV extraction.
- Drag-selected region recording with the cursor always excluded.
- Automatic H.264 MP4 post-processing and clipboard file copy.
- Hardware encoder detection with NVENC, Quick Sync, AMF, and x264 fallback.
- Compact, balanced, and quality compression profiles with output-size limits.
- Notification-area icon, context menu, recording-state tooltip, and Explorer
  restart recovery.
- Embedded multi-resolution application icon and Windows manifest.
- First-run local settings initialization and settings tray action.
- Source/release-aware one-click `setup.cmd` installation.
- Start menu, startup, and installed-app registration.
- Single-instance protection.
- Toast feedback across capture, OCR, recording, setup, and error workflows.

### Changed

- Automatic OCR now uses a single Tesseract TSV invocation.
- Tesseract, capture, encoder detection, and video post-processing run below normal priority.
- Hardware encoder capability detection is cached for the application session.
- Release output is compressed, metadata-free, fast-start enabled, and bounded
  to the selected profile's maximum dimensions by default.

## [0.1.0] - 2026-06-13

- Initial full-desktop video-to-clipboard and region OCR implementation.
