#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
./scripts/linux/check-linux.sh
version=$(sed -nE 's/^version = "([^"]+)"/\1/p' Cargo.toml | head -1)
release=release
stage="$release/parker-$version-linux-x64"
archive="$release/parker-$version-linux-x64.tar.gz"
rm -rf "$stage" "$archive" "$archive.sha256"
mkdir -p "$stage"
install -m 755 target/release/parker "$stage/parker"
install -m 755 scripts/linux/install-linux.sh "$stage/install-linux.sh"
install -m 644 packaging/linux/parker.desktop "$stage/parker.desktop"
install -m 644 README.md LICENSE settings.env.example "$stage/"
tar -C "$release" -czf "$archive" "$(basename "$stage")"
sha256sum "$archive" > "$archive.sha256"
rm -rf "$stage"
