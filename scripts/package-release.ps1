$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

& (Join-Path (Get-Location) "build.ps1")

$version = (Select-String -Path "Cargo.toml" -Pattern '^version\s*=\s*"([^\"]+)"').Matches[0].Groups[1].Value
$releaseDir = Join-Path (Get-Location) "release"
$staging = Join-Path $releaseDir "parker-$version-windows-x64"
$archive = Join-Path $releaseDir "parker-$version-windows-x64.zip"
$portable = Join-Path $releaseDir "parker-$version-windows-x64.exe"
$installer = Join-Path $releaseDir "parker-setup-$version-windows-x64.exe"

Remove-Item $staging -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item $archive -Force -ErrorAction SilentlyContinue
Remove-Item $portable -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $staging | Out-Null

Copy-Item "dist\parker.exe" $staging
Copy-Item "dist\parker.exe" $portable
Copy-Item "README.md" $staging
Copy-Item "LICENSE" $staging
Copy-Item "install.ps1" $staging
Copy-Item "uninstall.ps1" $staging
Copy-Item "setup.cmd" $staging
Copy-Item "settings.env.example" $staging
Copy-Item "scripts\install-ffmpeg.ps1" (Join-Path $staging "scripts\install-ffmpeg.ps1")
$version | Set-Content -Path (Join-Path $staging "version.txt") -Encoding ASCII

Compress-Archive -Path "$staging\*" -DestinationPath $archive -Force

# Build GUI installer with Inno Setup
$iscc = Get-Command "ISCC.exe" -ErrorAction SilentlyContinue
if (-not $iscc) {
    $isccPath = "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe"
    if (-not (Test-Path $isccPath)) {
        throw "Inno Setup 6 (ISCC.exe) not found. Install with: winget install --id JRSoftware.InnoSetup --exact"
    }
    $iscc = $isccPath
} else {
    $iscc = $iscc.Source
}

Write-Host "Building GUI installer with Inno Setup..."
& $iscc /Q "scripts\setup.iss"
if ($LASTEXITCODE -ne 0) {
    throw "Inno Setup compilation failed with exit code $LASTEXITCODE"
}

Remove-Item $staging -Recurse -Force

foreach ($asset in @($archive, $portable, $installer)) {
    $hash = (Get-FileHash -Algorithm SHA256 $asset).Hash.ToLowerInvariant()
    $checksum = "$asset.sha256"
    "$hash  $(Split-Path $asset -Leaf)" | Set-Content -Path $checksum -Encoding ASCII
    Write-Host "Created $asset"
    Write-Host "Created $checksum"
}
