$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

& (Join-Path (Get-Location) "build.ps1")

$version = (Select-String -Path "Cargo.toml" -Pattern '^version\s*=\s*"([^\"]+)"').Matches[0].Groups[1].Value
$releaseDir = Join-Path (Get-Location) "release"
$staging = Join-Path $releaseDir "parker-$version-windows-x64"
$archive = Join-Path $releaseDir "parker-$version-windows-x64.zip"

Remove-Item $staging -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item $archive -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $staging | Out-Null

Copy-Item "dist\parker.exe" $staging
Copy-Item "README.md" $staging
Copy-Item "LICENSE" $staging
Copy-Item "install.ps1" $staging
Copy-Item "uninstall.ps1" $staging
Copy-Item "setup.cmd" $staging
Copy-Item "settings.env.example" $staging

Compress-Archive -Path "$staging\*" -DestinationPath $archive -Force
Remove-Item $staging -Recurse -Force
$hash = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLowerInvariant()
$checksum = "$archive.sha256"
"$hash  $(Split-Path $archive -Leaf)" | Set-Content -Path $checksum -Encoding ASCII
Write-Host "Created $archive"
Write-Host "Created $checksum"
