# Parker — Usage Guide

This is the single document you need to install, configure, and use Parker daily on Windows and Linux. It reflects the current build (no glow effects, flat recording indicator and selector).

## Install

### Windows (per-user, no admin)

1. Download `parker-setup-<version>-windows-x64.exe` from GitHub Releases and open it.
2. The GUI lets you toggle startup, dependency install, and auto-launch. Defaults are fine for first use.
3. Or run headless from PowerShell beside the exe/ZIP:

   ```powershell
   Set-ExecutionPolicy -Scope Process Bypass
   .\install.ps1                 # defaults
   .\install.ps1 -NoStartup      # skip startup shortcut
   .\install.ps1 -SkipDependencies  # you already have FFmpeg/Tesseract
   .\install.ps1 -NoLaunch       # install only
   ```

Manual ZIP: extract and run `parker.exe` directly — first launch creates `%LOCALAPPDATA%\Parker\settings.env` and `Videos\Parker`.

### Linux (any distro)

#### Option 1: Release tarball

```bash
tar -xzf parker-<version>-linux-x64.tar.gz
cd parker-<version>-linux-x64
./install-linux.sh                  # → ~/.local/bin + desktop actions
./install-linux.sh --prefix ~/bin   # custom location
./install-linux.sh --system         # → /usr/local/bin (needs sudo)
./install-linux.sh --uninstall      # remove binary + desktop entry
```

#### Option 2: Build from source

Same pattern as MemReRust:

```sh
git clone https://github.com/AkumaNomu/Parker
cd Parker
cargo build --release
sudo install -Dm755 target/release/parker /usr/local/bin/parker
sudo install -Dm644 packaging/linux/parker.desktop /usr/share/applications/io.github.akumanomu.Parker.desktop
parker --version
```

Install the dependencies for your distribution:

| Distro | Example |
|---|---|
| Debian/Ubuntu/Mint | `sudo apt install grim slurp wf-recorder ffmpeg tesseract-ocr wl-clipboard libnotify-bin xclip` |
| Fedora/RHEL | `sudo dnf install grim slurp wf-recorder ffmpeg tesseract wl-clipboard libnotify python3-gobject` |
| Arch | `sudo pacman -S --needed grim slurp wf-recorder ffmpeg tesseract tesseract-data-eng wl-clipboard libnotify xclip` |
| openSUSE | `sudo zypper install grim slurp wf-recorder ffmpeg tesseract-ocr-traineddata-english wl-clipboard libnotify-tools xclip` |
| Alpine | `sudo apk add grim slurp wf-recorder ffmpeg tesseract-ocr wl-clipboard libnotify xclip` |

Tool roles: `grim+slurp` = Wayland region capture on wlroots compositors;
the GNOME screenshot portal (through the bundled Python helper), `spectacle`,
and `gnome-screenshot` = desktop-specific Wayland region capture;
`maim`/`scrot`/`import` = X11 region capture; `wf-recorder` = Wayland
recording on compositors with wlr-screencopy support; `ffmpeg` = encode;
`tesseract` = OCR; `wl-copy`/`xclip` = clipboard; `notify-send`/`curl` optional.

## Daily Use

Use hotkeys on Windows. On Linux, run commands or `parker gui`.

### Linux GUI

Run `parker gui` to open the primary actions: smart capture, screenshot, and
**More**. Recording is shown only when the current compositor supports it (or
when Parker has a saved recording to recover). **More** contains recordings,
settings, updates, and shortcuts. **Settings** edits common OCR, translation,
QR, audio, and video values. **Open settings file** opens every setting.

### Smart Capture — `Ctrl+Shift+F8` / `parker capture`

Drag a rectangle. Parker does one `rqrr` pass, retries at 2×/3× if needed, then one Tesseract TSV pass:

1. **QR** → all payloads copied (newline-separated), first `http`/`https` opened unless `PARKER_QR_AUTO_OPEN=0`.
2. **Table** → aligned columns become TSV (pastes into Sheets/Excel).
3. **Code** → keeps line breaks and leading indentation.
4. **Text** → plain Unicode.

The window is flat: solid dark toast, solid black selector dim, 1px white selection border. No glow, no rounded corners, no pulsing. Press `Esc` or right-click to cancel.

Linux command is `parker capture` — bind it to a shortcut:

| Desktop | Where |
|---|---|
| KDE | System Settings → Keyboard → Shortcuts → Add Command |
| GNOME | Settings → Keyboard → View and Customize → Custom Shortcuts |
| Sway/Hyprland | `bindsym $mod+Shift+F8 exec parker capture` in config |

### Screenshot — `Ctrl+Shift+F9` / `parker shot`

Copies the pixels, not OCR. Same selector, copies `image/png` to the clipboard via `wl-copy` or `xclip` on Linux and `CF_DIB` on Windows. Use when you want the picture itself. This is the primary image workflow; video below is intentionally de-prioritized.

```bash
parker shot   # select → image on clipboard, pasted into Slack/GIMP/etc.
```

Bind `parker shot` to `Ctrl+Shift+F9` (KDE/GNOME/Sway) — it sits next to capture and before video.

### Recording — `Ctrl+Shift+F11` / `parker record` + `parker stop`

1. `Ctrl+Shift+F11` (or `parker record`) → drag region → recording starts. A small flat timer with `REC 00:42` and a square Stop button appears (draggable, excluded from capture).
2. `Ctrl+Shift+F11` again (or `parker toggle` / `parker stop` / click Stop) → Parker writes `q`/`SIGINT`, waits up to 30s, then optimizes.

Optimization: probes `ffmpeg -encoders` once, tries NVENC / QSV / AMF / VAAPI (Linux) then `libx264`. Applies your compression profile, `PARKER_MAX_WIDTH`/`PARKER_MAX_HEIGHT` downscale (`0` disables), strips metadata, keeps opt-in audio, writes H.264 `yuv420p` + `avc1` + `+faststart`, verifies, deletes `.capture.mkv`, copies final MP4 URI (`CF_HDROP` on Windows, `text/uri-list` on Linux). Failures append to `ffmpeg.log` (and `postprocess.log` on Windows).

Recover leftovers:

```bash
parker batch ~/Videos/Parker   # finalizes any .capture.mkv
parker open                    # opens the recordings folder
```

### Tray / Dashboard (Windows)

Right-click the tray icon: Smart capture, Screenshot, Record/Stop, Open recordings, Open settings, Exit. Double-click opens recordings. The dashboard (`Parker` window) has Smart capture (`F8`), Screenshot (`F9`), Record (`F11`), Open recordings (`F10`), and Settings — screenshot is primary, video is de-prioritized to `F11`.

### Other Commands

```text
parker open              # recordings folder
parker config            # opens settings.env (Linux) / terminal helper (Windows)
parker batch DIR         # DIR defaults to .
parker gui               # Linux action buttons and common settings
parker --self-update     # curl → GitHub Releases → swap binary
parker --version         # e.g. 0.6.1
parker help              # usage
```

`--self-update` needs `curl`, `tar`, and `sha256sum` on Linux, or PowerShell
and `certutil` on Windows. Parker verifies the matching SHA-256 release asset
before extraction and keeps a rollback backup during replacement.

## Settings

File: Windows `%LOCALAPPDATA%\Parker\settings.env`, Linux
`~/.config/parker/settings.env`. Linux creates it when you open settings.
`KEY=VALUE` lines, env vars win.

| Key | Default | Notes |
|---|---|---|
| `PARKER_OCR_LANG` | `eng` | `eng+fra`, `jpn`, etc. |
| `PARKER_OCR_LANG_AUTO` | `1` | `0` disables OSD auto-detect |
| `PARKER_OCR_PSM` | `6` | `0`–`13` |
| `PARKER_OCR_MODE` | `auto` | `auto`/`text`/`code`/`table` |
| `PARKER_QR_AUTO_OPEN` | `1` | `0` = copy only |
| `PARKER_KEEP_OCR_CAPTURE` | `0` | `1` keeps captures in `Pictures/Parker` |
| `PARKER_RECORD_FPS` | `30` | `1`–`120` |
| `PARKER_COMPRESSION` | `balanced` | `compact`/`balanced`/`quality` |
| `PARKER_VIDEO_ENCODER` | `auto` | `auto`/`nvenc`/`qsv`/`amf`/`vaapi`(Linux)/`libx264` |
| `PARKER_AUDIO_DEVICE` | unset | DirectShow name (Win) / Pulse source (Linux); setting it enables audio |
| `PARKER_RECORD_AUDIO` | `0` | Linux `1` = default Pulse device |
| `PARKER_USE_GPU` | unset | `1` prefers GPU, falls back to x264 |
| `PARKER_MAX_WIDTH` | profile | `0` disables; `PARKER_MAX_HEIGHT` same |
| `PARKER_POST_CRF` | profile | `0`–`51`, overrides profile |
| `PARKER_POST_PRESET` | profile | x264 preset (x264 only on Linux) |
| `PARKER_OUTPUT` | `Videos/Parker` | final video dir |
| `PARKER_FFMPEG` | auto | explicit path |
| `PARKER_TESSERACT` | auto | explicit path |
| `PARKER_TRANSLATE_BACKEND` | `none` | `argos`/`libretranslate` |
| `PARKER_TRANSLATE_TARGET` | `en` | target lang |
| `PARKER_TRANSLATE_OUTPUT` | `original` | `translation`/`both` |
| `PARKER_TRANSLATE_ENDPOINT` | unset | e.g. `http://localhost:5000` |
| `PARKER_HOTKEY_*` | `F8`/`F9`/`F10`/`F11`/`F12` | Windows only, still behind `Ctrl+Shift` (`F8` capture, `F9` screenshot, `F10` recordings, `F11` record, `F12` exit) |

Profiles:

| Profile | CRF/CQ | Max | Intent |
|---|---|---|---|
| `compact` | 28 | 1600×900 | small, fast share |
| `balanced` | 24 | 1920×1080 | default |
| `quality` | 20 | 2560×1440 | high fidelity |

Edit, save, restart Parker.

## Troubleshooting

- **No shortcut fires (Linux):** Wayland does not allow apps to grab globals — bind in your desktop, not in Parker.
- **Recording fails:** `wf-recorder` needs wlr-screencopy support. Screenshots
  still work through a region-capable capture tool.
- **Selection cancels immediately:** On GNOME Wayland, Parker uses the desktop
  screenshot portal through the bundled helper. Install `python3-gobject` and
  make sure `xdg-desktop-portal-gnome` is running. KDE/wlroots sessions use
  their native selector tools.
- **Empty OCR:** Check `tesseract --list-langs`, contrast, language. Try `PARKER_OCR_PSM=3` or `PARKER_OCR_MODE=text`.
- **Table not TSV:** Borderless/merged cells confuse alignment; try a tighter selection.
- **Capture leaked / recording stuck:** `parker batch` and `parker toggle` recover; stale `~/.local/state/parker/recording` is auto-cleared.
- **Clipboard pastes old content:** under Wayland `wl-copy` keeps data until the provider exits — keep Parker running until pasted.
- **Self-update fails:** ensure `curl` + `tar` are installed and GitHub is reachable; see `ffmpeg.log`/`postprocess.log` beside recordings for encode failures.

## Privacy

All OCR, QR, and encoding is local (`tesseract`, `rqrr`, `ffmpeg`). No upload, no analytics, no cloud OCR. `curl` is only used for optional translation endpoints and `--self-update`. Temp captures are always deleted unless you set `PARKER_KEEP_OCR_CAPTURE=1`.
