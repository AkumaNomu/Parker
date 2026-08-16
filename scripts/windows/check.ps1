$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { throw "cargo fmt failed with exit code $LASTEXITCODE" }
cargo clippy --all-targets
if ($LASTEXITCODE -ne 0) { throw "cargo clippy failed with exit code $LASTEXITCODE" }
cargo test
if ($LASTEXITCODE -ne 0) { throw "cargo test failed with exit code $LASTEXITCODE" }
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "cargo build failed with exit code $LASTEXITCODE" }

Write-Host "Rust checks completed."
Write-Host "Run Parker on Windows to manually test hotkeys, QR opening, OCR classification, recording, post-processing, toasts, and clipboard behavior."
