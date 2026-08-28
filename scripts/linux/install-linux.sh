#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ./install-linux.sh [--prefix DIR] [--system] [--uninstall]

Options:
  --prefix DIR   Install to DIR/bin instead of ~/.local/bin
  --system       Install to /usr/local/bin (requires sudo)
  --uninstall    Remove installed files
EOF
}

prefix="${XDG_BIN_HOME:-$HOME/.local/bin}"
system=0
uninstall=0
while [ $# -gt 0 ]; do
  case "$1" in
    --prefix) prefix="$2"; shift 2 ;;
    --system) system=1; shift ;;
    --uninstall) uninstall=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) usage; exit 1 ;;
  esac
done

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)

if [ "$uninstall" = 1 ]; then
  target="$prefix"
  desktop="$HOME/.local/share/applications/io.github.akumanomu.Parker.desktop"
  if [ "$system" = 1 ]; then
    target="/usr/local/bin"
    desktop="/usr/share/applications/io.github.akumanomu.Parker.desktop"
    sudo rm -f "$target/parker" "$target/portal_capture.py" "$desktop"
  else
    rm -f "$target/parker" "$target/portal_capture.py" "$desktop"
  fi
  echo "Removed Parker from $target."
  exit 0
fi

if [ "$system" = 1 ]; then
  prefix="/usr/local/bin"
  mkdir -p /usr/share/applications 2>/dev/null || sudo mkdir -p /usr/share/applications
fi

mkdir -p "$prefix"
if [ "$system" = 1 ]; then
  sudo install -m 755 "$root/parker" "$prefix/parker"
  sudo install -m 755 "$root/portal_capture.py" "$prefix/portal_capture.py"
  sudo install -Dm644 "$root/parker.desktop" /usr/share/applications/io.github.akumanomu.Parker.desktop
else
  install -m 755 "$root/parker" "$prefix/parker"
  install -m 755 "$root/portal_capture.py" "$prefix/portal_capture.py"
  mkdir -p "$HOME/.local/share/applications"
  install -m 644 "$root/parker.desktop" "$HOME/.local/share/applications/io.github.akumanomu.Parker.desktop"
fi

detect_manager() {
  for manager in apt dnf pacman zypper apk; do
    if command -v "$manager" >/dev/null 2>&1; then
      echo "$manager"
      return 0
    fi
  done
  return 1
}

install_command() {
  case "$1" in
    apt) echo "sudo apt install grim slurp wf-recorder ffmpeg tesseract-ocr wl-clipboard libnotify-bin xclip python3-gi" ;;
    dnf) echo "sudo dnf install grim slurp wf-recorder ffmpeg tesseract wl-clipboard libnotify python3-gobject" ;;
    pacman) echo "sudo pacman -S --needed grim slurp wf-recorder ffmpeg tesseract tesseract-data-eng wl-clipboard libnotify xclip python-gobject" ;;
    zypper) echo "sudo zypper install grim slurp wf-recorder ffmpeg tesseract-ocr-traineddata-english wl-clipboard libnotify-tools xclip python3-gobject" ;;
    apk) echo "sudo apk add grim slurp wf-recorder ffmpeg tesseract-ocr wl-clipboard libnotify xclip py3-gobject" ;;
  esac
}

printf 'Installed Parker to %s\n' "$prefix/parker"

has_capture=0
for c in grim slurp spectacle gnome-screenshot maim scrot import; do
  if command -v "$c" >/dev/null 2>&1 || [ -x "/usr/bin/$c" ] || [ -x "/usr/local/bin/$c" ]; then has_capture=1; break; fi
done
has_clipboard=0
if command -v wl-copy >/dev/null 2>&1 || [ -x /usr/bin/wl-copy ] || command -v xclip >/dev/null 2>&1 || [ -x /usr/bin/xclip ]; then has_clipboard=1; fi

missing=""
for dep in ffmpeg tesseract; do
  if ! command -v "$dep" >/dev/null 2>&1 && [ ! -x "/usr/bin/$dep" ] && [ ! -x "/usr/local/bin/$dep" ]; then
    missing="$missing $dep"
  fi
done
if [ "$has_capture" = 0 ]; then missing="$missing capture-backend"; fi
if [ "$has_clipboard" = 0 ]; then missing="$missing wl-clipboard/xclip"; fi
case "${XDG_CURRENT_DESKTOP:-}:${XDG_SESSION_DESKTOP:-}" in
  *GNOME*)
    if [ -n "${WAYLAND_DISPLAY:-}" ] && ! python3 -c 'import gi' >/dev/null 2>&1; then
      missing="$missing python3-gobject"
    fi
    ;;
esac

if [ -z "$missing" ]; then
  printf 'All required dependencies are present.\n'
else
  manager=$(detect_manager || true)
  printf '\nSome runtime dependencies are missing:%s\n' "$missing"
  printf 'Fedora Wayland needs: grim slurp wf-recorder ffmpeg tesseract wl-clipboard libnotify; GNOME also needs python3-gobject.\n'
  if [ -n "${manager:-}" ]; then
    printf 'Install them with:\n\n  %s\n' "$(install_command "$manager")"
  fi
  printf 'If you already installed them, open a new terminal or run: hash -r; parker --version\n'
fi

case ":$PATH:" in
  *":$prefix:"*) ;;
  *) printf '\nNote: %s is not in your PATH. Add it to your shell profile.\n' "$prefix" ;;
esac

printf 'Bind desktop shortcuts to `parker capture` (F8), `parker shot` (F9), `parker open` (F10), and `parker toggle` (F11).\n'
