param(
    [switch]$NoStartup,
    [switch]$SkipDependencies,
    [switch]$NoLaunch
)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

function Write-Step([string]$Message) {
    Write-Host "[Parker] $Message" -ForegroundColor Cyan
}

function Find-Tesseract {
    $command = Get-Command tesseract.exe -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }

    $candidates = @()
    if ($env:ProgramFiles) {
        $candidates += Join-Path $env:ProgramFiles "Tesseract-OCR\tesseract.exe"
    }
    if (${env:ProgramFiles(x86)}) {
        $candidates += Join-Path ${env:ProgramFiles(x86)} "Tesseract-OCR\tesseract.exe"
    }
    if ($env:LOCALAPPDATA) {
        $candidates += Join-Path $env:LOCALAPPDATA "Programs\Tesseract-OCR\tesseract.exe"
        $candidates += Join-Path $env:LOCALAPPDATA "Microsoft\WinGet\Links\tesseract.exe"
    }

    return $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
}

function Resolve-ParkerExecutable {
    $candidates = @(
        (Join-Path $PSScriptRoot "parker.exe"),
        (Join-Path $PSScriptRoot "dist\parker.exe")
    )
    $existing = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    if ($existing) { return $existing }

    if ((Test-Path (Join-Path $PSScriptRoot "Cargo.toml")) -and (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Step "Building the release executable from source..."
        & (Join-Path $PSScriptRoot "build.ps1")
        return Join-Path $PSScriptRoot "dist\parker.exe"
    }

    throw "parker.exe was not found. Use a GitHub release package or install Rust and run this script from the repository root."
}

function Resolve-ParkerVersion {
    $versionFile = Join-Path $PSScriptRoot "version.txt"
    if (Test-Path $versionFile) {
        return (Get-Content $versionFile -Raw).Trim()
    }

    $cargoFile = Join-Path $PSScriptRoot "Cargo.toml"
    if (Test-Path $cargoFile) {
        $match = Select-String -Path $cargoFile -Pattern '^version\s*=\s*"([^\"]+)"' | Select-Object -First 1
        if ($match) { return $match.Matches[0].Groups[1].Value }
    }

    return "0.4.3"
}

function New-Shortcut(
    [string]$Path,
    [string]$Target,
    [string]$WorkingDirectory,
    [string]$Description
) {
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($Path)
    $shortcut.TargetPath = $Target
    $shortcut.WorkingDirectory = $WorkingDirectory
    $shortcut.Description = $Description
    $shortcut.IconLocation = "$Target,0"
    $shortcut.Save()
}

Write-Step "Preparing a clean per-user installation..."
$sourceExe = Resolve-ParkerExecutable
$version = Resolve-ParkerVersion
$installDir = Join-Path $env:LOCALAPPDATA "Parker"
$installedExe = Join-Path $installDir "parker.exe"

Get-Process parker -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 200
New-Item -ItemType Directory -Force -Path $installDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $installDir "logs") | Out-Null
Copy-Item $sourceExe $installedExe -Force
foreach ($file in @("README.md", "LICENSE", "uninstall.ps1")) {
    $source = Join-Path $PSScriptRoot $file
    if (Test-Path $source) { Copy-Item $source $installDir -Force }
}

$settingsPath = Join-Path $installDir "settings.env"
if (-not (Test-Path $settingsPath)) {
@'
# Parker settings
# Lines use KEY=VALUE. Restart Parker after editing.

PARKER_OCR_LANG=eng
PARKER_OCR_PSM=6
PARKER_OCR_MODE=auto
PARKER_QR_AUTO_OPEN=1
PARKER_KEEP_OCR_CAPTURE=0

PARKER_RECORD_FPS=30
PARKER_COMPRESSION=balanced
PARKER_VIDEO_ENCODER=auto
# Optional FFmpeg DirectShow audio device, for example:
# PARKER_AUDIO_DEVICE=Microphone Array (Realtek(R) Audio)
# PARKER_MAX_WIDTH=1920
# PARKER_MAX_HEIGHT=1080

# Advanced overrides
# PARKER_POST_CRF=24
# PARKER_POST_PRESET=medium
# PARKER_USE_GPU=1
# PARKER_HOTKEY_OCR=F8
# PARKER_HOTKEY_RECORD=F9
# PARKER_HOTKEY_FOLDER=F10
# PARKER_HOTKEY_QUIT=F12
'@ | Set-Content -Path $settingsPath -Encoding UTF8
}

if (-not $SkipDependencies) {
    $ffmpegTarget = Join-Path $installDir "ffmpeg.exe"
    if (-not (Test-Path $ffmpegTarget)) {
        Write-Step "Installing the optimized FFmpeg runtime..."
        $temp = Join-Path $env:TEMP ("parker-ffmpeg-" + [guid]::NewGuid())
        $zip = "$temp.zip"
        New-Item -ItemType Directory -Force -Path $temp | Out-Null
        try {
            Invoke-WebRequest -Uri "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip" -OutFile $zip
            Expand-Archive -Path $zip -DestinationPath $temp -Force
            $ffmpeg = Get-ChildItem -Path $temp -Filter ffmpeg.exe -Recurse | Select-Object -First 1
            if (-not $ffmpeg) {
                throw "The downloaded FFmpeg archive did not contain ffmpeg.exe."
            }
            Copy-Item $ffmpeg.FullName $ffmpegTarget -Force
        } finally {
            Remove-Item $zip -Force -ErrorAction SilentlyContinue
            Remove-Item $temp -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    $tesseract = Find-Tesseract
    if (-not $tesseract) {
        $winget = Get-Command winget.exe -ErrorAction SilentlyContinue
        if ($winget) {
            Write-Step "Installing local OCR support..."
            foreach ($package in @("tesseract-ocr.tesseract", "UB-Mannheim.TesseractOCR")) {
                & winget install --id $package --exact --silent --accept-package-agreements --accept-source-agreements
                if ($LASTEXITCODE -eq 0) { break }
            }
            $tesseract = Find-Tesseract
        }
        if (-not $tesseract) {
            Write-Warning "Tesseract was not installed. QR detection and recording work, but text/code/table OCR needs Tesseract."
        }
    }
}

Write-Step "Creating Start menu and startup entries..."
$programs = [Environment]::GetFolderPath("Programs")
$startMenuFolder = Join-Path $programs "Parker"
New-Item -ItemType Directory -Force -Path $startMenuFolder | Out-Null
New-Shortcut (Join-Path $startMenuFolder "Parker.lnk") $installedExe $installDir "Parker capture utility"

$startup = [Environment]::GetFolderPath("Startup")
$startupShortcut = Join-Path $startup "Parker.lnk"
if ($NoStartup) {
    Remove-Item $startupShortcut -Force -ErrorAction SilentlyContinue
} else {
    New-Shortcut $startupShortcut $installedExe $installDir "Start Parker with Windows"
}

$uninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Parker"
New-Item -Path $uninstallKey -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name DisplayName -Value "Parker" -PropertyType String -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name DisplayVersion -Value $version -PropertyType String -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name Publisher -Value "Akuma Nomu" -PropertyType String -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name InstallLocation -Value $installDir -PropertyType String -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name DisplayIcon -Value $installedExe -PropertyType String -Force | Out-Null
$uninstallScript = Join-Path $installDir "uninstall.ps1"
New-ItemProperty -Path $uninstallKey -Name UninstallString -Value "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$uninstallScript`"" -PropertyType String -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name NoModify -Value 1 -PropertyType DWord -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name NoRepair -Value 1 -PropertyType DWord -Force | Out-Null

if (-not $NoLaunch) {
    Write-Step "Starting Parker..."
    Start-Process $installedExe -WorkingDirectory $installDir
}

Write-Host ""
Write-Host "Parker is installed." -ForegroundColor Green
Write-Host "Notification-area icon: right-click for capture, recording, settings, and exit."
Write-Host "Ctrl+Shift+F8   Smart capture: QR, table, code, or text"
Write-Host "Ctrl+Shift+F9   Select/start region recording; press again to optimize and copy"
Write-Host "Ctrl+Shift+F10  Open recordings"
Write-Host "Ctrl+Shift+F12  Stop recording and exit"
Write-Host "Settings: $settingsPath"
