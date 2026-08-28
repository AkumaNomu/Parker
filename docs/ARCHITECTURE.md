# Architecture

Parker is a single Rust binary with two platform frontends: a Win32
event-driven application on Windows, and a CLI orchestrator on Linux that
drives standard tools (grim/slurp/wf-recorder/ffmpeg/tesseract) over Wayland
and X11. Shared logic lives in platform-neutral modules so both frontends stay
in sync.

## Shared modules

- `smart.rs`: the OCR classifier used by both platforms. Parses one Tesseract
  TSV pass into words, infers aligned tables (emitted as TSV), reconstructs
  lines and approximate indentation, and applies syntax heuristics to label
  code versus ordinary text. Also owns `PARKER_OCR_MODE` parsing.
- `translate.rs`: automatic OCR-language detection via Tesseract OSD plus
  confidence scoring across installed languages, and opt-in translation
  through argos-translate or a LibreTranslate endpoint (curl).
- `updater.rs`: GitHub Releases self-update. Queries the latest release via
  curl + the GitHub API, compares versions numerically, verifies the matching
  SHA-256 asset, safely extracts the platform binary, and replaces it with a
  rollback backup. No HTTP crate is involved; curl is required only when
  updating.
- `qr_common.rs`: shared safe HTTP(S) QR URL validation for both frontends.

## Windows frontend

- `windows_app.rs`: lifecycle, single-instance guard, hotkeys (`F8` capture, `F9`
  screenshot, `F10` recordings, `F11` record, `F12` exit), tray dispatch,
  smart routing, and workflow coordination.
- `dashboard.rs`: native dashboard window (capture `F8`, screenshot `F9`, record
  `F11`, recordings `F10`), app icon identity, workflow buttons, and status copy.
- `settings.rs`: first-run data-directory initialization, persistent
  `settings.env` creation, environment overrides, and settings opening.
- `tray.rs`: notification-area icon, context menu (capture `F8`, screenshot `F9`,
  record `F11`, recordings `F10`), tooltip state, and Explorer restart recovery.
- `selector.rs`: reusable, topmost virtual-desktop region selector.
- `screenshot.rs`: GDI `BitBlt` region capture and 32-bit BMP output.
- `qr.rs`: embedded QR detection with `rqrr`, HTTP(S) filtering, and browser
  opening through `ShellExecuteW`.
- `ocr.rs`: drives Tesseract and feeds it through `smart.rs`; also locates
  Tesseract and creates/retains capture files.
- `recorder.rs`: cursor-free FFmpeg region capture, graceful stop, hardware
  encoder detection/fallback, compression profiles, MP4 post-processing,
  validation, and cleanup. Recording is intentionally de-prioritized to `F11`.
- `config_ui.rs`: terminal helper for compression settings.
- `toast.rs`: non-activating Win32 toast-style windows excluded from capture.
- `clipboard.rs`: `CF_HDROP` file-copy, `CF_DIB` image-copy, and
  `CF_UNICODETEXT` text-copy behavior.
- `win.rs`: narrow Win32 FFI surface used by Parker.

## Linux frontend

`linux.rs` implements every subcommand by orchestrating CLI tools:

- Session detection (`WAYLAND_DISPLAY`/`DISPLAY`) picks clipboard and capture
  backends. Clipboard order is wl-copy then xclip under Wayland, reversed
  under X11; text uses plain input, images use `image/png`, files use
  `text/uri-list`.
- Capture providers are tried in order: the GNOME screenshot portal, grim+slurp,
  Spectacle, GNOME Screenshot area mode, maim, scrot, and ImageMagick import.
  Every provider must select a region; failures remove partial images before
  the next provider runs. X11-only providers are skipped on Wayland.
- Smart capture mirrors Windows: QR first (with a 2x/3x upscale retry), then a
  single Tesseract TSV pass classified by `smart.rs`. Temp captures are deleted
  unless `PARKER_KEEP_OCR_CAPTURE=1`, which stores them under `~/Pictures/Parker`.
- Recording shells out to wf-recorder (optionally `--audio` /
  `--audio-device`). State (PID + path) lives in
  `~/.local/state/parker/recording`; `stop` verifies the PID is `wf-recorder`
  before SIGINT, polls until the process exits (30 s budget), then finalizes.
- Linux loads `~/.config/parker/settings.env` at startup without overriding
  process environment values. `parker gui` provides action buttons and edits
  the common settings in place.
- Finalization tries hardware encoders first — NVENC, Quick Sync, AMF, VAAPI,
  filtered by parsing `ffmpeg -encoders`, or forced via `PARKER_VIDEO_ENCODER`
  — then software x264. Compression profiles and explicit size limits feed a
  downscale filter; metadata is stripped; failures append to `ffmpeg.log`
  beside the recordings while the `.capture.mkv` source is preserved until an
  encoder succeeds.

## Initialization

1. Parker sets per-monitor DPI awareness and a stable AppUserModelID.
2. A named mutex prevents duplicate instances.
3. `%LOCALAPPDATA%\Parker` and its settings/log directories are created.
4. Missing Windows `settings.env` is written atomically with safe defaults.
5. Settings are loaded unless a process environment variable already overrides
   the same key.
6. The dashboard window, global hotkeys, and notification-area icon are
   registered.
7. `TaskbarCreated` is monitored so the icon can be restored after Explorer
   restarts.

## Smart-capture workflow

1. `Ctrl+Shift+F8` opens the region selector over the virtual desktop.
2. Parker captures the selected rectangle to a temporary BMP (Windows) or PNG
   (Linux).
3. `rqrr` scans the grayscale image, retrying at 2x and 3x upscale before
   giving up. Decoded values are copied; the first safe HTTP(S) value is opened
   unless auto-opening is disabled.
4. If no QR is found, Tesseract emits TSV once in automatic mode.
5. Recurring aligned cell starts classify the selection as a table and produce
   TSV. Otherwise geometry reconstructs lines and approximate indentation,
   after which syntax heuristics classify code or ordinary text.
6. Unicode output is committed to the clipboard and the temporary capture is
   deleted unless retention is enabled.

## Screenshot workflow

`Ctrl+Shift+F9` (or `parker shot` on Linux) opens the selector, captures with
GDI `BitBlt` (Windows) or the GNOME portal/`grim`/`spectacle`/`maim` (Linux), and copies
`CF_DIB`/`image/png` straight to the clipboard. No OCR.

## Recording workflow

1. The first `Ctrl+Shift+F11` opens the region selector (Windows) or slurp
   (Linux). Recording is intentionally on `F11` to de-prioritize video.
2. Windows captures with FFmpeg `gdigrab`, always passing `-draw_mouse 0`;
   Linux runs wf-recorder on the selected geometry.
3. The second hotkey writes `q` to FFmpeg (Windows) or sends SIGINT to
   wf-recorder and waits for exit (Linux) so the temporary Matroska file closes
   cleanly.
4. Parker queries FFmpeg's encoder list. In automatic mode it attempts
   available NVENC, Quick Sync, AMF, and (on Linux) VAAPI paths before x264.
5. The selected compression profile controls quality, x264 preset, and default
   output bounds. User overrides are applied from `settings.env`.
6. The post-process strips metadata and non-selected streams, preserves opt-in
   audio when configured, constrains oversized captures, normalizes even
   dimensions, emits H.264 `yuv420p`, marks the codec as `avc1`, and enables
   MP4 fast-start.
7. Failed hardware attempts are removed before the next encoder is tried and
   logged to `ffmpeg.log`.
8. Parker verifies the final MP4, removes the intermediate file, and places it
   on the clipboard (`CF_HDROP` on Windows, `text/uri-list` elsewhere).

## Utility commands

- `parker gui` opens Linux action buttons and common settings; `parker config`
  opens the full settings file. On Windows `parker.exe config` opens the
  terminal helper.
- `parker shot` / `Ctrl+Shift+F9` copies a region image straight to the
  clipboard (`CF_DIB` on Windows, `image/png` on Linux) — primary image workflow.
- `parker batch <folder>` finalizes preserved `.capture.mkv` files.
- `parker --self-update` checks GitHub Releases for a newer Parker binary.
- `parker --version` prints the version.

## Performance decisions

- QR decoding is embedded and avoids a child process; upscale retries run only
  after a failed fast pass.
- Automatic OCR uses one Tesseract invocation instead of separate plain-text and
  TSV invocations.
- Tesseract and video post-processing run below normal process priority to
  reduce interference with foreground work (Windows).
- Capture uses an ultrafast temporary encode to minimize dropped frames; the
  final pass performs compression (Windows). Linux records directly with
  wf-recorder and compresses once during finalization.
- Hardware encoding is opportunistic and never required for correctness.
- Release builds enable LTO, one codegen unit, speed optimization, stripping, and
  abort-on-panic.

## Error recovery

Capture and post-processing logs are stored beside recordings. A failed
post-process preserves the `.capture.mkv` source. Clipboard failures never delete
the final MP4. On Windows errors appear as both a toast and a blocking message
box; on Linux they print to stderr and raise a desktop notification.

## External components

FFmpeg and Tesseract are runtime executables. QR decoding, image decoding, and
Windows resource embedding are build-time Rust dependencies. Self-update needs
curl at runtime but no network-capable crate is compiled in. Capture content is
never sent to a network service.
