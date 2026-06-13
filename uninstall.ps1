param([switch]$RemoveSettings)

$ErrorActionPreference = "Stop"
$installDir = Join-Path $env:LOCALAPPDATA "Parker"
Get-Process parker -ErrorAction SilentlyContinue | Stop-Process -Force

$startup = Join-Path ([Environment]::GetFolderPath("Startup")) "Parker.lnk"
Remove-Item $startup -Force -ErrorAction SilentlyContinue
$startMenuFolder = Join-Path ([Environment]::GetFolderPath("Programs")) "Parker"
Remove-Item $startMenuFolder -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Parker" -Recurse -Force -ErrorAction SilentlyContinue

if ($RemoveSettings) {
    Remove-Item $installDir -Recurse -Force -ErrorAction SilentlyContinue
} else {
    Get-ChildItem $installDir -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -notin @("settings.env", "logs") } |
        Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host "Parker was uninstalled. Recordings in Videos\Parker were left untouched."
if (-not $RemoveSettings) {
    Write-Host "Settings were preserved in $installDir. Run with -RemoveSettings to remove them."
}
