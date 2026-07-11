# Parker — User Guide

Parker is a hotkey-first Windows capture utility. Select any screen region and
get the useful result straight onto your clipboard — no cloud, no accounts.

---

## Quick start

| Key | What it does |
|---|---|
| `Ctrl+Shift+F8` | Smart capture — QR, table, code, or text |
| `Ctrl+Shift+F9` | Record a screen region to MP4 |
| `Ctrl+Shift+F7` | Record the last 30–60 seconds as a clip |
| `Ctrl+Shift+F11` | Stitch scrolling screenshots into one image |
| `Ctrl+Shift+F6` | Save a webpage locally (copy URL first) |
| `Ctrl+Shift+F10` | Open the recordings folder |
| `Ctrl+Shift+F12` | Exit Parker |

Press `Esc` or right-click to cancel any region selector.

---

## Smart capture — `Ctrl+Shift+F8`

Drag over any screen region. Parker classifies the content and copies the result
automatically.

### QR codes

Parker decodes all QR codes in the region. It copies every decoded value to the
clipboard and opens the first HTTP/HTTPS link in your browser. Set
`PARKER_QR_AUTO_OPEN=0` in settings to copy without opening.

### Tables

Parker detects aligned rows and columns and copies **tab-separated values (TSV)**
that paste directly into Excel, Google Sheets, and text editors.

### Code

Parker identifies common programming syntax and preserves line structure and
inferred indentation.

### Text

Standard Unicode OCR — clean, flattened text from any on-screen content.

> **Note:** Automatic classification uses one Tesseract TSV pass to determine the
> content type before reconstructing the result. QR decoding runs a fast first
> pass and only retries with upscaling when nothing is found.

---

## Region recording — `Ctrl+Shift+F9`

1. Press `Ctrl+Shift+F9` and drag over the region to record.
2. A draggable timer and stop control appears. Parker places it outside the
   recorded region when possible and asks Windows to exclude it from capture.
3. Click stop or press `Ctrl+Shift+F9` again to finish.
4. Parker optimizes the capture:
   - Finalizes a resilient temporary Matroska (MKV) file.
   - Detects NVENC, Intel Quick Sync, and AMD AMF encoders once per run.
   - Uses hardware encoding when available, falls back to x264 otherwise.
   - Compresses and downscales oversized captures based on your profile.
   - Strips metadata, audio, subtitle, and data streams.
   - Normalizes dimensions to broadly compatible H.264 `yuv420p`.
   - Writes MP4 fast-start metadata for immediate playback.
   - Removes the intermediate MKV.
   - Copies the MP4 as a Windows file-clipboard entry (`CF_HDROP`).

The mouse cursor is never included in the output.

---

## Clip recording — `Ctrl+Shift+F7`

Records a **rolling buffer** of the last 30–60 seconds instead of recording from
the moment you press the hotkey. Useful for capturing something that just
happened.

1. Press `Ctrl+Shift+F7` and drag over the clip region.
2. Parker starts buffering frames silently. No file is written yet.
3. Press `Ctrl+Shift+F7` again (or click stop on the indicator) to save only
   the buffered tail as an MP4.

Adjust the buffer length:

```ini
PARKER_RING_SECONDS=45
```

Minimum is 5 seconds.

---

## Audio recording

Record microphone, system audio (loopback), or both alongside video capture.

| `PARKER_RECORD_AUDIO` | Behavior |
|---|---|
| `none` (default) | No audio, video only |
| `mic` | Capture from default microphone |
| `system` | Capture system audio (what you hear) |
| `both` | Mix microphone and system audio |

Audio is encoded as AAC 192 kbps and muxed into the final MP4. Requires
FFmpeg's dshow devices to be available (Windows default).

```ini
PARKER_RECORD_AUDIO=mic
```

---

## Scroll capture — `Ctrl+Shift+F11`

Stitches multiple screenshots into one long image.

1. Press `Ctrl+Shift+F11` and drag over the region to capture.
2. Scroll the page, conversation, or document naturally.
3. Press `Ctrl+Shift+F11` again when done.
4. Parker stitches the screenshots and copies the result as a file-clipboard
   entry.

The mouse cursor is excluded from each frame.

### Auto-scroll mode

Set `PARKER_SCROLL_MODE=auto` to have Parker scroll automatically. It sends
mouse wheel events and stops when content stops changing (configurable via
`PARKER_SCROLL_STABLE_FRAMES`, default 5). Adjust scroll amount with
`PARKER_SCROLL_SPEED` (default 120 = one wheel notch per frame).

```ini
PARKER_SCROLL_MODE=auto
PARKER_SCROLL_SPEED=120
PARKER_SCROLL_STABLE_FRAMES=5
```

---

## Webpage extraction — `Ctrl+Shift+F6`

Saves a webpage and its assets to a local folder.

1. Copy a URL (starts with `http://` or `https://`).
2. Press `Ctrl+Shift+F6`.
3. Parker downloads the page HTML, CSS, JavaScript, images, and fonts into
   `%USERPROFILE%\Videos\Parker\parker-web\`.

This feature requires Parker built with the `site_retriever` feature (included
in default builds). Running `parker web <url>` from the command line works
without the clipboard step.

---

## Tray (notification area) controls

Parker's icon lives in the Windows notification area.

| Action | Behavior |
|---|---|
| **Right-click** | Context menu: Smart capture, Record region, Record clip, Scroll capture, Open recordings, Copy last file path, Settings, Extract webpage, Exit |
| **Double-click** | Opens the recordings folder |
| **Restart Explorer** | Parker restores its icon automatically |

The tray tooltip shows the current state: ready, recording, optimizing, or
stitching.

---

## Settings

Parker stores settings in:

```
%LOCALAPPDATA%\Parker\settings.env
```

Open it from the tray menu (`Settings`). Lines use `KEY=VALUE`. Process-level
environment variables take precedence over the file. Restart Parker after
editing.

### OCR

| Key | Default | Notes |
|---|---|---|
| `PARKER_OCR_LANG` | `eng` | Tesseract language, e.g. `eng+fra`, `eng+ara` |
| `PARKER_OCR_PSM` | `6` | Page-segmentation mode, `0`–`13` |
| `PARKER_OCR_MODE` | `auto` | `auto`, `text`, `code`, or `table` |
| `PARKER_QR_AUTO_OPEN` | `1` | `0` to copy QR URLs without opening |
| `PARKER_KEEP_OCR_CAPTURE` | `0` | `1` to retain the captured BMP |
| `PARKER_OCR_PREPROCESS` | `0` | `1` to enable image enhancement before OCR |
| `PARKER_OCR_PREPROCESS_CONTRAST` | `1.3` | Contrast multiplier |
| `PARKER_OCR_PREPROCESS_SHARPEN` | `0.3` | Sharpening strength |

### Recording

| Key | Default | Notes |
|---|---|---|
| `PARKER_RECORD_FPS` | `30` | Frames per second, `1`–`120` |
| `PARKER_COMPRESSION` | `balanced` | `compact`, `balanced`, or `quality` |
| `PARKER_VIDEO_ENCODER` | `auto` | `auto`, `nvenc`, `qsv`, `amf`, `libx264` |
| `PARKER_RING_SECONDS` | `45` | Rolling-buffer length for clip recording |
| `PARKER_RECORD_AUDIO` | `none` | `none`, `mic`, `system`, or `both` |
| `PARKER_MAX_WIDTH` | *profile* | Limit output width (`0` = unlimited) |
| `PARKER_MAX_HEIGHT` | *profile* | Limit output height (`0` = unlimited) |
| `PARKER_POST_CRF` | *profile* | x264/NVENC quality override, `0`–`51` |
| `PARKER_POST_PRESET` | *profile* | x264 speed/compression override |

### Scroll capture

| Key | Default | Notes |
|---|---|---|
| `PARKER_SCROLL_MODE` | `manual` | `manual` or `auto` |
| `PARKER_SCROLL_SPEED` | `120` | Wheel ticks per auto-scroll step |
| `PARKER_SCROLL_STABLE_FRAMES` | `5` | Consecutive unchanged frames before stopping |

### Scheduling

| Key | Default | Notes |
|---|---|---|
| `PARKER_SCHEDULE_ENABLED` | `0` | `1` to enable recurring captures |
| `PARKER_SCHEDULE_MODE` | `smart-capture` | `smart-capture`, `recording`, `clip`, `scroll-capture` |
| `PARKER_SCHEDULE_INTERVAL` | `30` | Minutes between captures |
| `PARKER_SCHEDULE_START` | `00:00` | Earliest time (24h format) |
| `PARKER_SCHEDULE_END` | `23:59` | Latest time (24h format) |
| `PARKER_SCHEDULE_COUNT` | unlimited | Maximum captures per session |

### Compression profiles

| Profile | CRF/CQ | Max output | Use case |
|---|---|---|---|
| `compact` | 28 | 1600×900 | Small files, fast sharing |
| `balanced` | 24 | 1920×1080 | Default |
| `quality` | 20 | 2560×1440 | Higher fidelity |

### Custom hotkeys

Uncomment and change the key in `settings.env`:

```ini
PARKER_HOTKEY_OCR=F8
PARKER_HOTKEY_RECORD=F9
PARKER_HOTKEY_CLIP=F7
PARKER_HOTKEY_SCROLL=F11
PARKER_HOTKEY_FOLDER=F10
PARKER_HOTKEY_QUIT=F12
PARKER_HOTKEY_WEB=F6
```

Keys can be `F1`–`F12` or a single letter. Parker prepresses `Ctrl+Shift+`
automatically.

### Paths

Auto-detected if not set:

```ini
PARKER_FFMPEG=C:\tools\ffmpeg.exe
PARKER_TESSERACT=C:\Program Files\Tesseract-OCR\tesseract.exe
PARKER_OUTPUT=%USERPROFILE%\Videos\Parker
```

---

## OCR preprocessing

Enable `PARKER_OCR_PREPROCESS=1` to apply image enhancement before sending
captures to Tesseract. This improves recognition on low-contrast, blurry, or
poorly lit source material at the cost of a small latency increase.

| Setting | Default | Effect |
|---|---|---|
| `PARKER_OCR_PREPROCESS` | `0` (off) | Set to `1` to enable |
| `PARKER_OCR_PREPROCESS_CONTRAST` | `1.3` | Contrast multiplier (1.0 = no change) |
| `PARKER_OCR_PREPROCESS_SHARPEN` | `0.3` | Sharpening strength (0.0 = no sharpen) |

```ini
PARKER_OCR_PREPROCESS=1
PARKER_OCR_PREPROCESS_CONTRAST=1.5
PARKER_OCR_PREPROCESS_SHARPEN=0.4
```

---

## Hotkey remapping UI

Run `parker config` from a terminal to open an interactive menu where you can
reassign all hotkeys without editing `settings.env` by hand. The menu also
lets you configure video quality (CRF + preset) and view current settings.

```
parker config
```

Hotkeys can be set to any F1–F12 key or a single letter. Changes take effect
after restarting Parker.

---

## Capture scheduling

Parker can run captures automatically on a recurring schedule. Configure via
`settings.env`:

```ini
PARKER_SCHEDULE_ENABLED=1
PARKER_SCHEDULE_MODE=smart-capture
PARKER_SCHEDULE_INTERVAL=30
PARKER_SCHEDULE_START=09:00
PARKER_SCHEDULE_END=23:00
PARKER_SCHEDULE_COUNT=10
```

| Setting | Default | Notes |
|---|---|---|
| `PARKER_SCHEDULE_ENABLED` | `0` | Set to `1` to enable |
| `PARKER_SCHEDULE_MODE` | `smart-capture` | `smart-capture`, `recording`, `clip`, `scroll-capture` |
| `PARKER_SCHEDULE_INTERVAL` | `30` | Minutes between captures |
| `PARKER_SCHEDULE_START` | `00:00` | Earliest time to capture (24h) |
| `PARKER_SCHEDULE_END` | `23:59` | Latest time to capture (24h) |
| `PARKER_SCHEDULE_COUNT` | unlimited | Maximum captures per session |

Scheduling respects Parker's busy state — it will skip a scheduled capture if
a recording or optimization is in progress.

---

## Command-line usage

```
parker                         Normal startup (tray + hotkeys)
parker config                  Open terminal configuration UI
parker batch [directory]       Batch-process .capture.mkv files
parker web <url> [outdir]      Extract a webpage to a folder
parker --self-update           Check for updates
```

---

## Clipboard behavior

- **Videos and scroll captures** use `CF_HDROP` — compatible apps receive a file
  reference rather than raw bytes. The file must remain at its saved path until
  it is pasted.
- **QR, table, code, and text results** use `CF_UNICODETEXT`.
- **Copy last file path** from the tray menu copies the path of the most recent
  saved capture.

## Files and directories

| Path | Purpose |
|---|---|
| `%LOCALAPPDATA%\Parker\parker.exe` | Installed executable |
| `%LOCALAPPDATA%\Parker\settings.env` | User configuration |
| `%LOCALAPPDATA%\Parker\logs\` | Runtime logs |
| `%LOCALAPPDATA%\Parker\ffmpeg.exe` | Bundled FFmpeg runtime |
| `%USERPROFILE%\Videos\Parker\` | Final recordings |
| `%USERPROFILE%\Videos\Parker\parker-web\` | Extracted web pages |

## Prerequisites

- **OCR:** Tesseract (`winget install tesseract-ocr.tesseract`)
- **Recording:** FFmpeg (installed automatically by the setup)
- **Webpage extraction:** Built with `--features site_retriever`

QR detection and screen capture work without any external dependencies.

## Limitations

- **Dense borderless tables** may not be classified correctly by the auto-detect
  heuristic.
- **Protected video (DRM)** and hardware overlays (e.g. video players) may
  appear blank due to Windows graphics pipeline protections.

## Future directions

These are new capabilities that would expand Parker's scope into adjacent
territory. They are not implemented yet.

### Screen reader / accessibility mode

Extract on-screen text, control labels, and UI element hierarchy using
Windows Automation API (UIA) or `AccEvent`. Parker could read dialog content,
error messages, or form labels aloud or output them as structured text —
useful for accessibility, automated UI testing, and hands-free monitoring.

### Remote capture relay

Broadcast a selected screen region as a real-time H.264 stream to a local
network or the internet via RTMP/SRT. Combined with the existing tray
controls, Parker could function as a lightweight streaming source for
presentations, demos, or surveillance without launching OBS.

### Version history / undo for captures

Maintain a short-term version history of clipboard captures so users can
recall the last N items (OCR results, file paths, QR URLs) from the tray
menu. Each capture would be timestamped and previewable before pasting.

### Plugin system / scriptable pipeline

Allow external scripts or binaries to hook into capture events. For example:
run a PowerShell script after every smart capture to post-process the text,
upload the result, or trigger a notification. A simple JSON event file or
stdin pipe would suffice.

### Multi-language mixed OCR

Run Tesseract with multiple language packs simultaneously on the same capture
and merge the results using per-word confidence scoring. Useful for documents
or screenshots that mix English with CJK, Cyrillic, or Arabic script.

### Overlay annotations

After selecting a region and before the result is copied, show a brief overlay
where the user can draw highlights, arrows, or redactions on the captured
image. These annotations become part of the final clipboard result.

### Virtual desktop awareness

Detect which Windows virtual desktop the captured region belongs to and tag
recordings with the desktop name. Useful for users who organize work across
multiple desktops (e.g. "Dev", "Design", "Comms").

### Watch folder integration

Monitor a folder for new files (e.g. screenshots from other tools) and
automatically process them through Parker's OCR pipeline — classify, extract
tables, decode QR codes, and push results to the clipboard or a log file.
