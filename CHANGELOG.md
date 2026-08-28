# Changelog

All notable changes to Parker are documented here.

## [Unreleased]

### Added

- Linux smart capture now matches Windows: one Tesseract TSV pass classifies
  the region as table (TSV), code (layout-preserving), or text, honoring
  `PARKER_OCR_MODE`. Shared classifier extracted into `src/smart.rs`.
- `parker shot` copies a selected region straight to the clipboard as an
  image without running OCR (Linux) and `Ctrl+Shift+F9` screenshot on Windows
  (`CF_DIB`); dashboard and tray now show screenshot before video.
- X11 session support: capture falls back to maim, scrot, or ImageMagick
  import with built-in selection; clipboard uses xclip when wl-copy is absent
  or the session is X11.
- KDE Spectacle and GNOME screenshot fallbacks for Wayland capture.
- GNOME Wayland region capture through the local screenshot portal helper, with
  X11-only tools skipped on Wayland.
- Opt-in Linux audio recording via `PARKER_RECORD_AUDIO=1` or
  `PARKER_AUDIO_DEVICE` (passed to `wf-recorder --audio [--audio-device]`).
- VAAPI hardware encoding on Linux alongside NVENC, Quick Sync, and AMF, each
  attempted before software x264 with automatic fallback and `ffmpeg.log`
  diagnostics.
- `parker --version`.

### Changed

- Linux GUI actions now stay in a single modal flow, keep buttons visible, hide
  unsupported GNOME recording, preserve current settings when text fields are
  left blank, and treat selector cancellation as a normal return.

- Removed `parker doctor` command.
- Self-update no longer bundles reqwest 0.9/native-tls/OpenSSL; it shells out
  to curl and the system tar/Expand-Archive. The dependency tree shrank from
  roughly 500 to about 35 crates, and Parker now builds on machines without a
  C toolchain. Windows shows a success toast after updating.
- Screenshots are now the primary image workflow: `Ctrl+Shift+F9` (`parker shot`)
  copies pixels as image, while video recording is de-prioritized to
  `Ctrl+Shift+F11` (`parker toggle`/`record`). Hotkeys are now
  `F8` capture, `F9` screenshot, `F10` recordings, `F11` record, `F12` exit.
  New `PARKER_HOTKEY_SHOT` override; `PARKER_HOTKEY_RECORD` default is now `F11`.
  Dashboard and tray list screenshot before video, and Linux shortcuts/help
  reflect the same ordering.
- All glow effects removed: selector no longer uses layered alpha, recording
  indicator is flat rectangles without rounded region or pulsing (see
  `src/selector.rs:60`, `src/recording_indicator.rs:71`, `src/recording_indicator.rs:294`).
- `inquire` and `embed-resource` are Windows-only dependencies now.
- Linux recording stop waits up to 30 seconds for wf-recorder to finalize,
  recovers gracefully from stale state files, and detects recorders that exit
  immediately (for example GNOME Wayland, which lacks wlr-screencopy).
- Linux finalization honors compression profiles, explicit
  `PARKER_MAX_WIDTH`/`PARKER_MAX_HEIGHT`, strips metadata, validates
  `PARKER_POST_CRF`, and applies `PARKER_POST_PRESET` to x264 only — matching
  documented Windows semantics.

### Fixed

- Linux capture now uses only region-capable backends, removes failed partial
  captures, and tries the next available backend after a capture failure.
- Linux settings load at startup, including translation settings; `parker gui`
  now provides capture actions and a common-settings editor.
- Linux recording verifies a saved PID is `wf-recorder` before signalling it.
- Self-update now requires and verifies the matching SHA-256 release asset,
  rejects unsafe archive members, and restores the old executable on a failed
  replacement.
- Windows cross-compilation no longer fails after translated OCR output is
  copied.
- QR decoding was broken on Linux: captures are PNG but only the BMP codec
  was compiled in. PNG is enabled now, plus a 2x/3x upscale retry matching
  README claims.
- Linux leaked temporary capture images in /tmp; they are deleted after every
  run unless `PARKER_KEEP_OCR_CAPTURE=1` retains them under `~/Pictures/Parker`.
- Windows recorder no longer leaks a partial `.capture.mkv` when FFmpeg exits
  immediately, deletes an unverifiable final MP4 instead of leaving it behind,
  and treats a failed encoder probe as "software only" rather than caching the
  failure for the whole session.
- Windows `config` helper matches settings keys exactly instead of by prefix
  and writes the settings file atomically.
- Windows batch mode no longer flashes a console window while locating FFmpeg
  (it reuses the recorder's locator).
- Windows screenshot capture releases GDI handles even when a region is too
  large, and the region selector destroys its overlay if selector state becomes
  unavailable mid-selection.

## [0.6.1] - 2026-07-29

### Added

- Fedora Linux Wayland commands for region OCR, QR actions, recording,
  conversion, clipboard copy, settings, recordings, batch recovery, and update.
- Fedora installer, desktop actions, Linux CI, and Linux release tarball with
  SHA-256 checksum.

## [0.4.3] - 2026-06-23

### Added

- More guided Windows setup experience with install destination, progress state,
  option grouping, audio guidance, and install-folder access after success.
- Runtime dashboard copy that surfaces version, audio support, and core
  shortcuts in one place.

### Changed

- Release metadata, installer fallback version, and publishing docs now target
  `0.4.3`.
- Audio recording support is documented as an opt-in `PARKER_AUDIO_DEVICE`
  path instead of hidden behavior.
- Config, batch, self-update, hotkey override, and GPU preference controls are
  surfaced in the README/settings docs.

## [0.4.1] - 2026-06-15

### Added

- Simple native dashboard for capture, recording, recordings, and settings.
- Taskbar window identity with Parker's embedded application icon.
- GUI setup wrapper for release installers with startup, dependency, and launch options.

### Changed

- Double-clicking the notification-area icon now opens Parker's dashboard.
- Tray icon loading now requests the embedded 16-pixel icon explicitly.
- Release setup falls back to the command-line installer if the GUI cannot start.

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
