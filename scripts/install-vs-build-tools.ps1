<#
Unattended installer for Visual Studio Build Tools (Windows).
This script downloads the bootstrapper and runs it with the C++ workload.
Run with elevated privileges. If not elevated, the script will attempt to relaunch as admin.

Usage:
  PowerShell -ExecutionPolicy Bypass -File .\scripts\install-vs-build-tools.ps1
  PowerShell -ExecutionPolicy Bypass -File .\scripts\install-vs-build-tools.ps1 -InstallPath "C:\BuildTools" -Quiet
#>

[CmdletBinding()]
param(
    [string]$InstallPath = "C:\BuildTools",
    [switch]$Quiet
)

function Assert-Admin {
    $isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltinRole]::Administrator)
    if (-not $isAdmin) {
        Write-Host "Not elevated. Relaunching as administrator..."
        $arg = "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`""
        if ($InstallPath) { $arg += " -InstallPath `"$InstallPath`"" }
        if ($Quiet) { $arg += " -Quiet" }
        Start-Process -FilePath powershell -ArgumentList $arg -Verb RunAs
        exit
    }
}

function LinkExists {
    $link = Get-Command link.exe -ErrorAction SilentlyContinue
    return $null -ne $link
}

Assert-Admin

if (LinkExists) {
    Write-Host "MSVC linker 'link.exe' already available. Skipping installation."
    exit 0
}

$url = 'https://aka.ms/vs/17/release/vs_buildtools.exe'
$out = Join-Path -Path $env:TEMP -ChildPath 'vs_buildtools.exe'

Write-Host "Downloading Visual Studio Build Tools bootstrapper to $out"
try {
    Invoke-WebRequest -Uri $url -OutFile $out -UseBasicParsing -ErrorAction Stop
} catch {
    Write-Error "Download failed: $_"
    exit 1
}

$args = "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --installPath `"$InstallPath`""

if ($Quiet) { Write-Host "Starting silent installation (this may take several minutes)..." } else { Write-Host "Starting installation (interactive progress hidden due to --quiet)." }

try {
    $p = Start-Process -FilePath $out -ArgumentList $args -Wait -Passthru
    if ($p.ExitCode -eq 0) { Write-Host "Installation finished."; exit 0 } else { Write-Error "Installer exited with code $($p.ExitCode)"; exit $p.ExitCode }
} catch {
    Write-Error "Installer failed to start: $_"
    exit 1
}
