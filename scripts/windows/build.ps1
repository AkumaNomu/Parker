$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $root

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Rust/Cargo was not found. Install Rust with: winget install --id Rustlang.Rustup --exact"
}

Write-Host "Building Parker..."
cargo build --release
if ($LASTEXITCODE -ne 0) {
    throw "cargo build --release failed with exit code $LASTEXITCODE"
}

$dist = Join-Path $root "dist"
New-Item -ItemType Directory -Force -Path $dist | Out-Null
Copy-Item (Join-Path $root "target\release\parker.exe") $dist -Force
Copy-Item (Join-Path $root "README.md") $dist -Force
Copy-Item (Join-Path $root "LICENSE") $dist -Force
Copy-Item (Join-Path $PSScriptRoot "install.ps1") $dist -Force
Copy-Item (Join-Path $PSScriptRoot "uninstall.ps1") $dist -Force
Copy-Item (Join-Path $PSScriptRoot "setup.cmd") $dist -Force
Copy-Item (Join-Path $PSScriptRoot "setup-gui.ps1") $dist -Force
Copy-Item (Join-Path $root "settings.env.example") $dist -Force

Write-Host "Built: $dist\parker.exe"
