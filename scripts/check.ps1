$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
cargo build --release

Write-Host "Rust checks completed."
Write-Host "Run Parker on Windows to manually test hotkeys, QR opening, OCR classification, recording, post-processing, toasts, and clipboard behavior."
