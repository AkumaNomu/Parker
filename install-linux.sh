#!/usr/bin/env bash
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
install_dir="${XDG_BIN_HOME:-$HOME/.local/bin}"
mkdir -p "$install_dir"
install -m 755 "$root/parker" "$install_dir/parker"
mkdir -p "$HOME/.local/share/applications"
install -m 644 "$root/parker.desktop" "$HOME/.local/share/applications/io.github.akumanomu.Parker.desktop"
printf 'Installed Parker to %s\n' "$install_dir/parker"
printf 'Install runtime: sudo dnf install grim slurp wf-recorder ffmpeg tesseract wl-clipboard libnotify\n'
printf 'Add %s to PATH if needed.\n' "$install_dir"
