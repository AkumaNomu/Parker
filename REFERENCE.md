# Parker Reference

Complete module-by-module documentation of all features, APIs, and gaps.

---

## Modules

### Core
| Module | File | Purpose |
|---|---|---|
| main | `src/main.rs` | Application entry, message loop, hotkey dispatch, action routing |
| win | `src/win.rs` | Raw Win32 FFI bindings (all `extern "system"` functions, structs, constants) |
| settings | `src/settings.rs` | Config file (INI-style), env var fallback, data dir initialization |

### Capture & Recording
| Module | File | Purpose |
|---|---|---|
| screenshot | `src/screenshot.rs` | GDI screen capture → BMP file. Also exports `write_bmp()` |
| selector | `src/selector.rs` | Interactive region selection via fullscreen overlay window |
| ocr | `src/ocr.rs` | Tesseract EXE invocation: text, code, table OCR modes |
| qr | `src/qr.rs` | QR/barcode detection via rqrr crate |
| recorder | `src/recorder.rs` | FFmpeg screen region recording + post-processing |
| scroll_capture | `src/scroll_capture.rs` | Auto/scroll capture with frame-diff stability detection |
| recording_indicator | `src/recording_indicator.rs` | On-screen recording boundary indicator window |
| input_capture | `src/input_capture.rs` | Passive low-level keyboard/mouse hook → ring buffer |

### Input Control (NEW)
| Module | File | Purpose |
|---|---|---|
| **input_controller** | `src/input_controller.rs` | Active input simulation via `SendInput` |
| **screen_controller** | `src/screen_controller.rs` | In-memory screen capture, pixel search, OCR find, template matching |

### Assistant Features
| Module | File | Purpose |
|---|---|---|
| activity | `src/activity.rs` | JSONL activity log, clipboard history, capture logging |
| clipboard | `src/clipboard.rs` | Clipboard read/write (text, file drop), `AddClipboardFormatListener` monitor |
| virtual_desktop | `src/virtual_desktop.rs` | COM `IVirtualDesktopManager` → current desktop GUID |

### UI & Orchestration
| Module | File | Purpose |
|---|---|---|
| tray | `src/tray.rs` | System tray icon + context menu |
| toast | `src/toast.rs` | Windows toast notifications (balanced multi-monitor) |
| config_ui | `src/config_ui.rs` | Interactive hotkey/CRF settings via `parker config` |
| scheduler | `src/scheduler.rs` | Timer-based recurring capture scheduling |
| signals | `src/signals.rs` | Sound cues (beeps) for events |
| updater | `src/updater.rs` | GitHub release self-update |
| site_retriever | `src/site_retriever.rs` | Webpage extraction (optional, feature-gated) |

---

## Input Controller (`input_controller.rs`)

### Functions
| Function | Description |
|---|---|
| `send_input(inputs)` | Low-level `SendInput` wrapper |
| `move_mouse(x, y)` | Move cursor to absolute screen position |
| `move_mouse_relative(dx, dy)` | Move cursor relative to current position |
| `get_cursor_pos()` → `(i32, i32)` | Current cursor screen position |
| `click_left()` | Click at current cursor position |
| `click_left_at(pos)` | Move to (x,y) then click |
| `click_right()` | Right-click at current position |
| `double_click()` | Two left clicks with 50ms delay |
| `scroll_wheel(delta)` | Scroll wheel (positive=up, negative=down) |
| `drag(from, to)` | Mouse drag from one point to another (10-step interpolation) |
| `type_text(text)` | Type a string character-by-character with US keyboard mapping |
| `type_char(c)` | Single character type with shift handling |
| `press_key(vk)` | Press and release a virtual key |
| `hold_key_down(vk)` | Press a key down without releasing |
| `release_key(vk)` | Release a held key |
| `key_combo(keys)` | Press multiple keys in sequence, release in reverse |
| `hotkey(ctrl, alt, shift, vk)` | Convenience: modifier key combo |
| `type_from_clipboard()` | Read clipboard text and type it |
| `focus_window(hwnd)` | Bring window to foreground and focus |
| `find_window(title)` | Find HWND by window title |
| `activate_window(title)` | Find and focus window by title |
| `wait_ms(ms)` | `thread::sleep` wrapper |

### VK Constants
All standard Windows virtual-key codes defined in `win.rs` as `WORD`: `VK_A..VK_Z`, `VK_0..VK_9`, `VK_F1..VK_F12`, `VK_SHIFT`, `VK_CONTROL`, `VK_MENU(Alt)`, `VK_LWIN`, `VK_LEFT/UP/RIGHT/DOWN`, `VK_RETURN`, `VK_TAB`, `VK_SPACE`, etc.

### char_to_key() Lookup Table
Maps printable ASCII to `(vk, scan_code, shift_needed)` for accurate keyboard simulation via scan codes.

---

## Screen Controller (`screen_controller.rs`)

### Types
| Type | Description |
|---|---|
| `ScreenImage` | In-memory `BGRA 32bpp` pixel buffer: `width`, `height`, `pixels` |
| `Point` | `{ x: i32, y: i32 }` |
| `MonitorInfo` | `{ index, rect, is_primary, device_name }` |

### Capture Functions
| Function | Description |
|---|---|
| `capture_screen()` | Capture entire virtual desktop (all monitors) |
| `capture_primary()` | Capture primary monitor only |
| `capture_monitor(index)` | Capture specific monitor by index |
| `capture_region(x, y, w, h)` | Capture arbitrary screen rectangle via GDI `BitBlt` + `GetDIBits` |

### Pixel/Color Operations
| Function | Description |
|---|---|
| `ScreenImage::pixel(x, y)` → `(R, G, B)` | Get pixel at coordinates |
| `ScreenImage::color_at(x, y)` → `u32 RGB` | Get pixel as RGB integer |
| `ScreenImage::save_bmp(path)` | Write pixel buffer to BMP file |
| `find_color(img, target, tolerance)` → `Vec<Point>` | Find all pixels of a color |
| `find_first_color(img, target, tolerance)` → `Option<Point>` | Find first matching pixel |
| `get_pixel_color(x, y)` → `Option<u32>` | Get color at live screen coordinate |

### OCR / Text Search
| Function | Description |
|---|---|
| `find_text_on_screen(text)` → `Vec<Point>` | Capture screen, OCR it, search for text. Returns approximate center point |
| `find_text_in_image(img, text)` → `Vec<Point>` | Search existing ScreenImage for text via Tesseract |
| `wait_for_text(text, timeout_ms)` → `Point` | Poll screen until text appears or timeout |

### Template Matching
| Function | Description |
|---|---|
| `find_template(haystack, needle, threshold)` → `Vec<Point>` | SAD (sum of absolute differences) pattern matching. Returns top-left match positions where avg pixel diff < threshold |

---

## Activity Assistant (`activity.rs`)

### Functions
| Function | Description |
|---|---|
| `log_capture(kind, detail)` | Log capture event to `activity.jsonl` |
| `log_clipboard(text)` | Log clipboard change to `clipboard.json` |
| `get_clipboard_history()` → `Vec<String>` | Read clipboard history (last 20 items) |

### Settings (env vars)
- `PARKER_ACTIVITY_LOG` — set to `0` to disable activity logging (default: `1`)
- `PARKER_CLIPBOARD_HISTORY` — max clipboard entries to keep (default: `50`)

### Files
- `{data_dir}/activity/activity.jsonl` — JSONL event log with `ts`, `kind`, `detail`, `desktop`
- `{data_dir}/activity/clipboard.json` — JSON array of `ClipboardEntry { ts, text, desktop }`

---

## Virtual Desktop (`virtual_desktop.rs`)

### Functions
| Function | Description |
|---|---|
| `initialize()` | Initialize COM and create VirtualDesktopManager singleton |
| `current_desktop_id()` → `Option<String>` | Returns short GUID prefix (`XXXXXX-XXXX-XXXX`) of current desktop |
| `desktop_label()` → `String` | Returns desktop ID or custom name from `PARKER_DESKTOP_{GUID}` env var |

### Notes
- Requires Windows 10 v1903+ (IVirtualDesktopManager was added in 10.0.18362)
- Uses COM `CoInitializeEx(APARTMENTTHREADED)` + `CoCreateInstance`
- Passing `null HWND` to `GetWindowDesktopId` returns the desktop of the foreground window

---

## Tray Menu Items

| Item | Action |
|---|---|
| Smart capture | `SmartCapture` |
| Record a region / Stop recording | `ToggleRecording` |
| Record 30-60s clip / Stop clip | `ToggleClipRecording` |
| Scroll capture / Stop scroll | `ToggleScrollCapture` |
| Open recordings | `OpenRecordings` |
| Copy last file path | `CopyLastPath` |
| Settings | `OpenSettings` |
| Clipboard history | `ClipboardHistory` — opens JSON + shows preview toast |
| Activity log | `ActivityLog` — opens JSONL file |
| Type clipboard text | `TypeClipboard` — reads clipboard and types it out |
| Click at cursor | `ClickHere` — left-click at current position |
| Find & click text from clipboard | `FindTextOnScreen` — OCR screen → find text → click it |
| Save screenshot | `SaveScreenshot` — capture primary monitor to BMP |
| Extract webpage | `ExtractWebpage` (feature-gated) |
| Exit | `Exit` |

---

## Hotkeys (default, configurable via env vars)

| Env Var | Default | Action |
|---|---|---|
| `PARKER_HOTKEY_OCR` | `Ctrl+Shift+F8` | Smart capture (OCR/QR) |
| `PARKER_HOTKEY_RECORD` | `Ctrl+Shift+F9` | Toggle recording |
| `PARKER_HOTKEY_CLIP` | `Ctrl+Shift+F7` | Toggle clip recording |
| `PARKER_HOTKEY_SCROLL` | `Ctrl+Shift+F11` | Toggle scroll capture |
| `PARKER_HOTKEY_FOLDER` | `Ctrl+Shift+F10` | Open recordings folder |
| `PARKER_HOTKEY_QUIT` | `Ctrl+Shift+F12` | Exit |
| `PARKER_HOTKEY_WEB` | `Ctrl+Shift+F6` | Extract webpage (if built with feature) |

---

## All Environment Variables

### OCR
- `PARKER_OCR_LANG` — Tesseract language (default: `eng`)
- `PARKER_OCR_PSM` — Tesseract page segmentation mode (default: `6`)
- `PARKER_OCR_MODE` — `auto`, `text`, `code`, `table` (default: `auto`)
- `PARKER_OCR_PREPROCESS` — `1` to enable contrast+sharpen preprocessing
- `PARKER_OCR_PREPROCESS_CONTRAST` — Contrast adjustment factor (default: `1.3`)
- `PARKER_OCR_PREPROCESS_SHARPEN` — Sharpening strength (default: `0.3`)
- `PARKER_QR_AUTO_OPEN` — `1` to auto-open QR URLs (default: `1`)
- `PARKER_KEEP_OCR_CAPTURE` — `1` to keep OCR temp captures (default: `0`)

### Recording
- `PARKER_RECORD_FPS` — Recording FPS (default: `30`)
- `PARKER_COMPRESSION` — `balanced`, `size`, `quality` (default: `balanced`)
- `PARKER_VIDEO_ENCODER` — `auto`, `h264_nvenc`, `h264_amf`, etc.
- `PARKER_RING_SECONDS` — Clip recording duration (default: `45`)
- `PARKER_RECORD_AUDIO` — `none`, `mic`, `system`, `both`
- `PARKER_MAX_WIDTH` — Max output width (default: `1920`)
- `PARKER_MAX_HEIGHT` — Max output height (default: `1080`)

### Scroll Capture
- `PARKER_SCROLL_MODE` — `manual` or `auto` (default: `manual`)
- `PARKER_SCROLL_SPEED` — Mouse wheel delta per scroll (default: `120`)
- `PARKER_SCROLL_STABLE_FRAMES` — Frames of stability before auto-stop (default: `5`)

### Scheduling
- `PARKER_SCHEDULE_ENABLED` — `1` to enable (default: `0`)
- `PARKER_SCHEDULE_MODE` — `smart-capture`, `recording`, `clip`, `scroll-capture`
- `PARKER_SCHEDULE_INTERVAL` — Interval in minutes (default: `30`)
- `PARKER_SCHEDULE_START` — `HH:MM` format
- `PARKER_SCHEDULE_END` — `HH:MM` format
- `PARKER_SCHEDULE_COUNT` — Max captures per day (default: `10`)

### Video Overrides
- `PARKER_POST_CRF` — CRF value (default: determined by compression profile)
- `PARKER_POST_PRESET` — FFmpeg preset (default: determined by compression profile)

### Activity Assistant
- `PARKER_ACTIVITY_LOG` — `1` to enable activity logging (default: `1`)
- `PARKER_CLIPBOARD_HISTORY` — Max clipboard history entries (default: `50`)

### Virtual Desktop
- `PARKER_DESKTOP_{GUID}` — Custom label for a desktop GUID

---

## Data Directory

Location: `%APPDATA%/Parker` (Windows) or `~/Library/Application Support/Parker` (macOS, not yet supported)

```
%APPDATA%/Parker/
├── parker.conf              # Settings file (INI-like KEY=VALUE)
├── recordings/             # Captures, screenshots, scroll captures
│   └── ...
├── activity/
│   ├── activity.jsonl    # Activity event log (JSONL format)
│   └── clipboard.json     # Clipboard history (JSON array)
```

---

## Features & Gaps

### Implemented
- [x] OCR (text, code, table, auto-detect)
- [x] QR/barcode scanning
- [x] Screen region recording (FFmpeg)
- [x] Scroll capture (manual + auto mode)
- [x] Audio recording (mic/system/both)
- [x] Hotkey rebinding (env var + interactive UI)
- [x] Capture scheduling
- [x] System tray + context menu
- [x] Multi-monitor toasts
- [x] Virtual desktop awareness
- [x] Clipboard history monitoring
- [x] Activity logging (JSONL)
- [x] Input simulation (mouse + keyboard via SendInput)
- [x] In-memory screen capture (GDI GetDIBits)
- [x] Pixel/color search
- [x] Screen text search (OCR-based)
- [x] Template/pattern matching (SAD)
- [x] Self-updater
- [x] Webpage extraction (feature-gated)
- [x] Single-instance guard
- [x] Interactive config UI

### Known Gaps
- [ ] **No image template matching** `find_template` uses simple SAD — no scale/rotation invariance
- [ ] **No programmatic scroll capture** — scrolling still requires user wheel or auto-mouse. Scroll capture via `SendInput` wheel events is possible but not wired
- [ ] **No UIA automation** — no MSAA/UIA window traversal, no element tree, no `AccEvent` listener
- [ ] **No image-to-image comparison** — could add perceptual hashing or SSIM for diff detection
- [ ] **No remote control** — no HTTP/pipe listener for external commands
- [ ] **No scripting/automation pipeline** — no way to chain actions (e.g., "find text → click → type → screenshot")
- [ ] **No hotkey for automation actions** — TypeClipboard/ClickHere/FindText/SaveScreenshot only in tray menu, not mappable to hotkeys
- [ ] **No accessibility** — Parker window has no accessible name, could interfere with screen readers
- [ ] **No LLM integration** — could use clipboard as bridge (copy text → paste into LLM → copy result → type back)

### Potential Future
- Scriptable automation pipeline (YAML/TOML-based)
- Image-based UI element detection (match icon/button templates)
- Accessibility tree walking for reliable element finding
- OCR-based click automation ("find the 'Save' button and click it")
- Foreground window tracking (auto-labelling captures by app name)
- Replay engine for input_capture ring buffer