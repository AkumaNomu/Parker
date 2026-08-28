# Parker

Parker is a local, hotkey-first Windows and Linux capture utility written in
Rust. It can understand a selected screen region or record one, then place the
useful result straight onto your clipboard.

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

### Screenshot — `Ctrl + Shift + F9`

Drag over a region and Parker copies the pixels as `image/png` (Windows `CF_DIB`)
straight to the clipboard — no OCR. Paste into Slack, Figma, GIMP, or any image
target.

### Region recording — `Ctrl + Shift + F11`

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
- strips metadata, subtitle, and data streams, while preserving opt-in audio;
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
| `Ctrl + Shift + F9` | Screenshot — copy region pixels as image. |
| `Ctrl + Shift + F10` | Open the recordings directory. |
| `Ctrl + Shift + F11` | Select/start region recording; press again to optimize and copy. |
| `Ctrl + Shift + F12` | Finalize an active recording and exit. |

Press `Esc` or right-click to cancel a selector.

## Install a GitHub release

Download `parker-setup-<version>-windows-x64.exe` from the latest GitHub Release
and open it. The setup flow opens a small GUI where you can choose startup,
dependency, and launch options. The release also includes a ZIP for manual
installation.

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

### Fedora Linux and other distributions

Download `parker-<version>-linux-x64.tar.gz`, extract it, then run:

```bash
./install-linux.sh
```

Parker captures only selected regions. On Wayland, use `grim` with `slurp`,
KDE Spectacle, or the GNOME screenshot portal; on X11, use maim, scrot, or
ImageMagick.
For example, on Fedora:

```bash
sudo dnf install grim slurp wf-recorder ffmpeg tesseract wl-clipboard libnotify python3-gobject
```

Bind these commands in **System Settings → Keyboard → Shortcuts → Add Command**
(or your compositor's keybind config):

| Shortcut | Command |
|---|---|
| `Ctrl+Shift+F8` | `parker capture` |
| `Ctrl+Shift+F9` | `parker shot` |
| `Ctrl+Shift+F10` | `parker open` |
| `Ctrl+Shift+F11` | `parker toggle` |

`capture` selects a region, copies QR data or smart OCR output — tables become
TSV, code keeps its layout, other text is copied as-is — and opens safe QR web
links. `shot` copies the region as an image instead of running OCR. `record`
selects and starts a region recording; `stop` finalizes an H.264 MP4 and
copies its file URI to the clipboard. `toggle` starts or stops a recording.
Wayland does not let ordinary apps register global shortcuts, so shortcuts stay
in the desktop settings.

Screenshots are the primary image workflow; video recording is available on
`F11`/`toggle` but intentionally de-prioritized. Region recordings try NVIDIA
NVENC, Intel Quick Sync, AMD AMF, and VAAPI hardware encoders before falling
back to software x264, honoring the same compression profiles and size limits
as Windows.

Note: `wf-recorder` requires a compositor with wlr-screencopy support. It does
not provide a portable KDE or GNOME recording path; use a supported compositor
or another recorder. On GNOME Wayland, screenshots use the local
`xdg-desktop-portal-gnome` picker through the bundled `portal_capture.py`
helper when PyGObject (`python3-gobject`) is installed.

## Useful runtime commands

```text
parker capture       Select a region: QR data or smart OCR text to clipboard.
parker shot          Select a region and copy the image itself.
parker record/stop   Start/finish a region recording (MP4 to clipboard).
parker toggle        Start a recording or finish the active one.
parker open          Open the recordings folder.
parker batch DIR     Finalize leftover .capture.mkv files.
parker gui           Open action buttons and a settings editor on Linux.
parker --self-update Update Parker from GitHub Releases (uses curl).
parker --version     Print the version.
```

On Linux, `gui` opens capture actions and a settings editor, while `config`
opens `~/.config/parker/settings.env`. On Windows,
`.\parker.exe config` starts the terminal settings helper.

## Build and install from source

### Option 1: Install scripts (recommended)

Windows — install Rust, reopen PowerShell, then run the installer:

```powershell
winget install --id Rustlang.Rustup --exact
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\windows\install.ps1
```

Linux — use the bundled installer after building or from a release tarball:

```bash
./scripts/linux/install-linux.sh
```

Build without installing (Windows — copies to `dist\parker.exe` with icon + manifest):

```powershell
.\scripts\windows\build.ps1
```

### Option 2: Build from source

Same as MemReRust — plain `cargo`:

```sh
git clone https://github.com/AkumaNomu/Parker
cd Parker
cargo build --release
sudo install -Dm755 target/release/parker /usr/local/bin/parker
```

On Linux you may also want the desktop entry:

```sh
sudo install -Dm644 packaging/linux/parker.desktop /usr/share/applications/io.github.akumanomu.Parker.desktop
```

On Windows:

```sh
git clone https://github.com/AkumaNomu/Parker
cd Parker
cargo build --release
# target\release\parker.exe is ready — run it or use .\scripts\windows\install.ps1 to register it
```

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
| `PARKER_AUDIO_DEVICE` | unset | Optional audio device: FFmpeg DirectShow name on Windows, PulseAudio source for wf-recorder on Linux. Setting it enables audio capture. |
| `PARKER_RECORD_AUDIO` | `0` | Linux only. Set to `1` to record the default PulseAudio device without naming one. |
| `PARKER_USE_GPU` | unset | Set to `1` to prefer GPU encoders only, with x264 fallback. |
| `PARKER_MAX_WIDTH` | Profile-defined | Optional maximum final width; `0` disables size limiting. |
| `PARKER_MAX_HEIGHT` | Profile-defined | Optional maximum final height; `0` disables size limiting. |
| `PARKER_POST_CRF` | Profile-defined | Optional x264/NVENC quality override, `0`–`51`. |
| `PARKER_POST_PRESET` | Profile-defined | Optional x264 speed/compression override. |
| `PARKER_HOTKEY_OCR` | `F8` | Override smart-capture key; Parker still uses `Ctrl+Shift`. |
| `PARKER_HOTKEY_SHOT` | `F9` | Override screenshot key; Parker still uses `Ctrl+Shift`. |
| `PARKER_HOTKEY_FOLDER` | `F10` | Override recordings-folder key; Parker still uses `Ctrl+Shift`. |
| `PARKER_HOTKEY_RECORD` | `F11` | Override recording key; Parker still uses `Ctrl+Shift`. |
| `PARKER_HOTKEY_QUIT` | `F12` | Override exit key; Parker still uses `Ctrl+Shift`. |

Compression profiles:

| Profile | Default quality | Maximum output | Intent |
|---|---:|---:|---|
| `compact` | CRF/CQ 28 | 1600×900 | Small files and fast sharing. |
| `balanced` | CRF/CQ 24 | 1920×1080 | Default workflow. |
| `quality` | CRF/CQ 20 | 2560×1440 | Higher visual fidelity. |

Explicit maximum dimensions in `settings.env` override the profile defaults.
Hardware encoders are attempted safely; a failed hardware path automatically
falls back to software x264. On Linux the candidates are NVENC, Quick Sync,
AMF, and VAAPI (`PARKER_VIDEO_ENCODER=vaapi` is Linux-only), and failed
attempts are appended to `ffmpeg.log` beside the recordings. Capture, OCR, and
transcoding processes run without console windows and at reduced priority so
normal desktop work remains responsive.

## Files and privacy

Final videos are stored in:

```text
Windows: %USERPROFILE%\Videos\Parker
Linux:   ~/Videos/Parker (XDG-aware)
```

OCR screenshots are deleted after processing unless retention is enabled
(retained captures live under `Pictures\Parker` on Windows and
`~/Pictures/Parker` on Linux). A failed video conversion preserves the
`.capture.mkv` source and writes details to `ffmpeg.log` (plus
`postprocess.log` on Windows) beside the recordings.

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
.\scripts\windows\check.ps1      # Windows
./scripts/linux/check-linux.sh   # Linux
```

This checks formatting, runs Clippy and tests, and builds a release executable.
Release builds use optimization level 3, link-time optimization, one codegen
unit, symbol stripping, and abort-on-panic to reduce runtime and binary overhead.
Windows CI performs the same validation. Tagged `v*` pushes generate a setup
EXE and a manual-install ZIP. Linux CI builds the Fedora-compatible tarball.

Repository layout:

```text
assets/                      Application icon
src/                         Rust application source
scripts/windows/             Windows build, setup, validation, and release
scripts/linux/               Linux install, validation, and release
packaging/linux/             Linux desktop integration files
.github/workflows/           Windows CI and releases
docs/                        Architecture, setup, usage, development, roadmap
settings.env.example         Template for the per-user settings file
```

For daily use see `docs/USAGE.md` — one guide for both platforms.

## Known limitations

- Audio is opt-in and depends on a valid device name (DirectShow on Windows,
  PulseAudio on Linux).
- OCR quality depends on source resolution, contrast, language data, and font.
- Dense borderless tables may not be classified correctly.
- Protected video and hardware overlays may appear blank.
- Windows hotkey labels in the tray and dashboard are fixed text even when
  `PARKER_HOTKEY_*` overrides are set.
- Custom toast overlays currently appear on the primary monitor.
- GNOME Wayland sessions can screenshot but cannot record (no
  wlr-screencopy).

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
