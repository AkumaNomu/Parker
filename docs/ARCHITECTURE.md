# Architecture

Parker is a single-process, event-driven Windows background application. A
hidden owner window receives global hotkeys and notification-area callbacks and
owns clipboard writes. Temporary selector and toast windows use small Win32
message loops and are excluded from capture where supported.

## Modules

- `main.rs`: lifecycle, single-instance guard, hotkeys, tray dispatch, smart
  routing, and workflow coordination.
- `settings.rs`: first-run data-directory initialization, persistent
  `settings.env` creation, environment overrides, and settings opening.
- `tray.rs`: notification-area icon, context menu, tooltip state, and Explorer
  restart recovery.
- `selector.rs`: reusable, topmost virtual-desktop region selector.
- `screenshot.rs`: GDI `BitBlt` region capture and 32-bit BMP output.
- `qr.rs`: embedded QR detection with `rqrr`, HTTP(S) filtering, and browser
  opening through `ShellExecuteW`.
- `ocr.rs`: one-pass automatic Tesseract TSV processing, code classification,
  line/indent reconstruction, table inference, and forced-mode overrides.
- `recorder.rs`: cursor-free FFmpeg region capture, graceful stop, hardware
  encoder detection/fallback, compression profiles, MP4 post-processing,
  validation, and cleanup.
- `toast.rs`: non-activating Win32 toast-style windows excluded from capture.
- `clipboard.rs`: `CF_HDROP` file-copy and `CF_UNICODETEXT` text-copy behavior.
- `win.rs`: narrow Win32 FFI surface used by Parker.

## Initialization

1. Parker sets per-monitor DPI awareness and a stable AppUserModelID.
2. A named mutex prevents duplicate instances.
3. `%LOCALAPPDATA%\Parker` and its settings/log directories are created.
4. Missing `settings.env` is written atomically with safe defaults.
5. Settings are loaded unless a process environment variable already overrides
   the same key.
6. The hidden app window, global hotkeys, and notification-area icon are
   registered.
7. `TaskbarCreated` is monitored so the icon can be restored after Explorer
   restarts.

## Smart-capture workflow

1. `Ctrl+Shift+F8` opens the region selector over the virtual desktop.
2. Parker captures the selected rectangle to a temporary BMP.
3. `rqrr` scans the grayscale image. Decoded values are copied; the first safe
   HTTP(S) value is opened unless auto-opening is disabled.
4. If no QR is found, Tesseract emits TSV once in automatic mode.
5. Recurring aligned cell starts classify the selection as a table and produce
   TSV. Otherwise geometry reconstructs lines and approximate indentation,
   after which syntax heuristics classify code or ordinary text.
6. Unicode output is committed to the clipboard and the temporary BMP is
   deleted unless retention is enabled.

## Recording workflow

1. The first `Ctrl+Shift+F9` opens the region selector.
2. FFmpeg `gdigrab` captures the selected coordinates and dimensions. Parker
   always passes `-draw_mouse 0`.
3. The second hotkey writes `q` to FFmpeg so the temporary Matroska file closes
   cleanly.
4. Parker queries FFmpeg's encoder list. In automatic mode it attempts available
   NVENC, Quick Sync, and AMF paths before x264.
5. The selected compression profile controls quality, x264 preset, and default
   output bounds. User overrides are applied from `settings.env`.
6. The post-process strips non-video streams and metadata, constrains oversized
   captures, normalizes even dimensions, emits H.264 `yuv420p`, marks the codec
   as `avc1`, and enables MP4 fast-start.
7. Failed hardware attempts are removed before the next encoder is tried.
8. Parker verifies the final MP4, removes the intermediate file, and places the
   final path on the clipboard as `CF_HDROP`.

## Performance decisions

- QR decoding is embedded and avoids a child process.
- Automatic OCR uses one Tesseract invocation instead of separate plain-text and
  TSV invocations.
- Tesseract and video post-processing run below normal process priority to
  reduce interference with foreground work.
- Capture uses an ultrafast temporary encode to minimize dropped frames; the
  final pass performs compression.
- Hardware encoding is opportunistic and never required for correctness.
- Release builds enable LTO, one codegen unit, speed optimization, stripping, and
  abort-on-panic.

## Error recovery

Capture and post-processing logs are stored beside recordings. A failed
post-process preserves the `.capture.mkv` source. Clipboard failures never delete
the final MP4. Errors appear as both a toast and a blocking message box.

## External components

FFmpeg and Tesseract are runtime executables. QR decoding, image decoding, and
Windows resource embedding are build-time Rust dependencies. Capture content is
never sent to a network service.
