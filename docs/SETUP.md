# Setup and initialization

## Linux installation

Extract the release tarball and run the installer:

```bash
tar -xzf parker-<version>-linux-x64.tar.gz
cd parker-<version>-linux-x64
./install-linux.sh
```

The script installs `parker` to `~/.local/bin` (override with `--prefix DIR`
or use `--system` for `/usr/local/bin`), registers desktop actions in your
application launcher, checks dependencies, and prints a distro-specific
install command when something is missing. Re-run with `--uninstall` to
remove it.

`./install-linux.sh --help` lists every option.

### Dependencies by distribution

Install the dependencies for your distribution:

| Distro | Command |
|---|---|
| Debian/Ubuntu/Mint | `sudo apt install grim slurp wf-recorder ffmpeg tesseract-ocr wl-clipboard libnotify-bin xclip` |
| Fedora/RHEL | `sudo dnf install grim slurp wf-recorder ffmpeg tesseract wl-clipboard libnotify python3-gobject` |
| Arch/Manjaro | `sudo pacman -S --needed grim slurp wf-recorder ffmpeg tesseract tesseract-data-eng wl-clipboard libnotify xclip` |
| openSUSE | `sudo zypper install grim slurp wf-recorder ffmpeg tesseract-ocr-traineddata-english wl-clipboard libnotify-tools xclip` |
| Alpine/postmarketOS | `sudo apk add grim slurp wf-recorder ffmpeg tesseract-ocr wl-clipboard libnotify xclip` |

What each tool does:

- `grim` + `slurp` — Wayland region capture and selection on compositors that
  support them.
- GNOME screenshot portal via the bundled `portal_capture.py` helper — the
  preferred GNOME Wayland region picker (requires `python3-gobject` and
  `xdg-desktop-portal-gnome`).
- `spectacle` / `gnome-screenshot` — optional desktop-specific fallbacks.
- `maim`, `scrot`, or ImageMagick `import` — X11 capture with selection.
- `wf-recorder` — Wayland region recording. Requires wlr-screencopy support;
  verify compositor support before relying on it. GNOME Wayland does not
  support recording through this backend.
- `ffmpeg` — finalization, hardware encoding, compression.
- `tesseract` — OCR text extraction.
- `wl-copy` (Wayland) or `xclip` (X11) — clipboard delivery for text, images,
  and file URIs.
- `libnotify`/`notify-send` — desktop notifications.
- `curl` — self-update downloads.

### Shortcuts

Wayland apps cannot register global shortcuts. Bind desktop commands instead
(screenshot `F9` is primary; video `F11` is de-prioritized):

| Shortcut | Command |
|---|---|
| `Ctrl+Shift+F8` | `parker capture` |
| `Ctrl+Shift+F9` | `parker shot` |
| `Ctrl+Shift+F10` | `parker open` |
| `Ctrl+Shift+F11` | `parker toggle` |

KDE: System Settings → Keyboard → Shortcuts → Add Command. Sway/Hyprland:
bind in the compositor config.

### Settings

Linux settings live at `~/.config/parker/settings.env`. Parker loads them on
startup without overriding process environment variables. Run `parker gui` for
action buttons and common settings, or `parker config` to open the full file.
See `settings.env.example` for the full annotated list.

## Windows installation

Download and open `parker-setup-<version>-windows-x64.exe` from GitHub
Releases. It extracts the release payload and opens the Parker setup GUI. The
GUI lets you choose startup, dependency installation, and launch options, then
invokes `install.ps1` with a temporary PowerShell execution-policy bypass. The
ZIP remains available for manual setup.

The installer is per-user and supports a prebuilt release or a source checkout.
It finds `parker.exe` beside the script, under `dist`, or builds it with Cargo.
It then:

1. stops an existing Parker process;
2. installs files under `%LOCALAPPDATA%\Parker`;
3. preserves an existing `settings.env`;
4. downloads FFmpeg when missing;
5. attempts a silent Tesseract installation through `winget`;
6. creates Start menu and startup shortcuts using the embedded application icon;
7. creates an HKCU uninstall entry;
8. starts Parker unless `-NoLaunch` is supplied.

Options:

```powershell
.\install.ps1 -NoStartup
.\install.ps1 -SkipDependencies
.\install.ps1 -NoLaunch
```

## First application launch

Parker independently initializes its data directory and settings file. This
means the executable from the manual-install ZIP remains usable even when it was
not installed by the script. Process environment variables override values from
`settings.env`.

## Updating

### Windows

Open the newer release's setup EXE. Existing settings are preserved. The
executable and support files are replaced after the running Parker process is
stopped.

`parker.exe --self-update` also works: it downloads the matching release ZIP
and SHA-256 asset via curl, verifies the archive, then swaps in the executable
with rollback on replacement failure.

### Linux

```bash
parker --self-update
```

This fetches the latest release metadata from GitHub, downloads
`parker-<version>-linux-x64.tar.gz` and its SHA-256 asset, verifies both, and
replaces the installed binary with rollback on replacement failure. Re-run
`./install-linux.sh` from a newer tarball if you prefer manual updates.

## Uninstalling

### Windows

```powershell
.\uninstall.ps1
```

This removes the executable, shortcuts, and uninstall registration while
preserving settings by default. Use `-RemoveSettings` for a full configuration
cleanup. Recordings are never deleted automatically.

### Linux

```bash
./install-linux.sh --uninstall
```

Removes the binary and desktop entry. Recordings under `~/Videos/Parker` are
preserved. Delete `~/.config/parker` manually if you want settings gone too.
