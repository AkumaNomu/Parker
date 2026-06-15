# Parker

Parker is a local, hotkey-first Windows capture utility written in Rust. It can
understand a selected screen region or record one, then place the useful result
straight onto the Windows clipboard.

## What Parker does

### Smart capture — `Ctrl + Shift + F8`

Drag over a screen region. Parker routes it automatically:

1. **QR code:** decodes all QR codes, copies their contents, and opens the first
   valid HTTP/HTTPS link.
2. **Table:** reconstructs aligned rows and columns and copies TSV that pastes
   directly into Excel, Google Sheets, databases, and text editors.
3. **Code:** detects common programming syntax and preserves line structure and
   inferred indentation.
4. **Text:** copies standard Unicode OCR text.

Automatic OCR mode uses one Tesseract TSV pass for classification and text
reconstruction instead of running OCR twice. QR decoding only performs its
higher-cost upscale retry when the fast first pass finds nothing.

### Region recording — `Ctrl + Shift + F9`

Press the hotkey, drag over a region, and Parker starts recording it. The mouse
cursor is always excluded. A draggable timer and stop control remains visible
while recording. Parker places it outside the selected region when space allows
and asks Windows to exclude it from captured output. Click stop or press the same
hotkey again to finish.

Parker then automatically:

- finalizes a resilient temporary Matroska capture;
- detects NVIDIA NVENC, Intel Quick Sync, and AMD AMF encoders once per run;
- attempts supported hardware encoding before falling back to x264;
- compresses and optionally downscales oversized captures;
- strips metadata, audio, subtitle, and data streams;
- normalizes dimensions and emits broadly compatible H.264 `yuv420p` video;
- writes MP4 fast-start metadata for immediate playback;
- removes the intermediate file after successful conversion;
- copies the final MP4 as a Windows file clipboard entry.

## Notification-area controls

Parker installs a persistent icon in the Windows notification area. Right-click
it to:

- start smart capture;
- start or stop region recording;
- open recordings;
- open the settings file;
- exit Parker.

Double-clicking the icon opens the recordings folder. The icon is restored if
Windows Explorer restarts, and Parker prevents duplicate instances.

## Feedback

Parker shows non-activating toast overlays for startup, analysis, cancellation,
recording, optimization, clipboard completion, QR opening, OCR classification,
folder/settings opening, and errors. Parker's own overlays are excluded from
screen capture.

## Hotkeys

| Hotkey | Action |
|---|---|
| `Ctrl + Shift + F8` | Select a region for QR detection or smart OCR. |
| `Ctrl + Shift + F9` | Select/start region recording; press again to optimize and copy. |
| `Ctrl + Shift + F10` | Open the recordings directory. |
| `Ctrl + Shift + F12` | Finalize an active recording and exit. |

Press `Esc` or right-click to cancel a selector.

## Install a GitHub release

Download `parker-setup-<version>-windows-x64.exe` from the latest GitHub Release
and open it. The release also includes a portable `parker-<version>-windows-x64.exe`
and a ZIP for manual installation.

The per-user installer does not require administrator access. It:

- installs Parker under `%LOCALAPPDATA%\Parker`;
- downloads a local FFmpeg runtime;
- attempts to install Tesseract through `winget`;
- creates Start menu and optional startup shortcuts;
- registers Parker in Windows' installed-app list;
- creates a persistent settings file;
- launches Parker.

PowerShell equivalent:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\install.ps1
```

Useful installer options:

```powershell
.\install.ps1 -NoStartup
.\install.ps1 -SkipDependencies
.\install.ps1 -NoLaunch
```

## Build and install from source

Install Rust, reopen PowerShell, then run the installer:

```powershell
winget install --id Rustlang.Rustup --exact
Set-ExecutionPolicy -Scope Process Bypass
.\install.ps1
```

Build without installing:

```powershell
.\build.ps1
```

The executable is copied to `dist\parker.exe` with the Parker icon and Windows
manifest embedded.

## Local settings

On first initialization Parker creates:

```text
%LOCALAPPDATA%\Parker\settings.env
```

Open it from the tray menu. Settings use `KEY=VALUE` lines and are applied when
Parker next starts. Process-level environment variables take precedence.

| Setting | Default | Purpose |
|---|---:|---|
| `PARKER_OUTPUT` | `%USERPROFILE%\Videos\Parker` | Final video directory. |
| `PARKER_FFMPEG` | Auto-detected | Explicit `ffmpeg.exe` path. |
| `PARKER_TESSERACT` | Auto-detected | Explicit `tesseract.exe` path. |
| `PARKER_OCR_LANG` | `eng` | Tesseract language, such as `eng+fra`. |
| `PARKER_OCR_PSM` | `6` | Tesseract page-segmentation mode, `0`–`13`. |
| `PARKER_OCR_MODE` | `auto` | `auto`, `text`, `code`, or `table`. |
| `PARKER_QR_AUTO_OPEN` | `1` | Set to `0` to copy QR URLs without opening them. |
| `PARKER_KEEP_OCR_CAPTURE` | `0` | Retain selected BMP captures when enabled. |
| `PARKER_RECORD_FPS` | `30` | Capture rate, `1`–`120`. |
| `PARKER_COMPRESSION` | `balanced` | `compact`, `balanced`, or `quality`. |
| `PARKER_VIDEO_ENCODER` | `auto` | `auto`, `nvenc`, `qsv`, `amf`, or `libx264`. |
| `PARKER_MAX_WIDTH` | Profile-defined | Optional maximum final width; `0` disables size limiting. |
| `PARKER_MAX_HEIGHT` | Profile-defined | Optional maximum final height; `0` disables size limiting. |
| `PARKER_POST_CRF` | Profile-defined | Optional x264/NVENC quality override, `0`–`51`. |
| `PARKER_POST_PRESET` | Profile-defined | Optional x264 speed/compression override. |

Compression profiles:

| Profile | Default quality | Maximum output | Intent |
|---|---:|---:|---|
| `compact` | CRF/CQ 28 | 1600×900 | Small files and fast sharing. |
| `balanced` | CRF/CQ 24 | 1920×1080 | Default workflow. |
| `quality` | CRF/CQ 20 | 2560×1440 | Higher visual fidelity. |

Explicit maximum dimensions in `settings.env` override the profile defaults.
Hardware encoders are detected once per Parker session and attempted safely; a
failed hardware path automatically falls back to software x264. Capture, OCR,
and transcoding processes run without console windows and at reduced priority
so normal desktop work remains responsive.

## Files and privacy

Final videos are stored in:

```text
%USERPROFILE%\Videos\Parker
```

OCR screenshots are deleted after processing unless retention is enabled. A
failed video conversion preserves the `.capture.mkv` source and writes details
to `ffmpeg.log` or `postprocess.log` beside the recordings.

Parker has no account, analytics, capture upload, or cloud OCR. The setup script
uses the network only to obtain dependencies. QR auto-opening is restricted to
whitespace-free HTTP and HTTPS values, but QR content should still be treated as
untrusted.

## Clipboard behavior

- Videos use Windows `CF_HDROP`, so compatible applications receive an MP4 file
  rather than raw bytes or a path string.
- QR, table, code, and text results use `CF_UNICODETEXT`.
- A copied video must remain at its saved path until it is pasted.

## Development

```powershell
.\scripts\check.ps1
```

This checks formatting, runs Clippy and tests, and builds a release executable.
Release builds use optimization level 3, link-time optimization, one codegen
unit, symbol stripping, and abort-on-panic to reduce runtime and binary overhead.
Windows CI performs the same validation. Tagged `v*` pushes generate a portable
release ZIP.

Repository layout:

```text
assets/                      Application icon
src/                         Rust application source
scripts/                     Validation and release packaging
.github/workflows/           Windows CI and releases
docs/                        Architecture, setup, development, and roadmap
setup.cmd                    Double-click setup entry point
install.ps1                  Source/release-aware per-user installer
```

## Known limitations

- Recording currently has no microphone or system audio.
- OCR quality depends on source resolution, contrast, language data, and font.
- Dense borderless tables may not be classified correctly.
- Protected video and hardware overlays may appear blank.
- Hotkeys are fixed in the current release.
- Custom toast overlays currently appear on the primary monitor.

## Uninstall

From the repository/release folder:

```powershell
.\uninstall.ps1
```

Use `-RemoveSettings` to also remove `%LOCALAPPDATA%\Parker`. Recordings under
`Videos\Parker` are always preserved.

## License

Parker is released under the MIT License. FFmpeg, Tesseract, `image`, `rqrr`, and
`embed-resource` remain governed by their own licenses.
