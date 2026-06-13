# Setup and initialization

## Release installation

Extract a GitHub release and double-click `setup.cmd`. It invokes `install.ps1`
with a temporary PowerShell execution-policy bypass.

The installer is per-user and supports a prebuilt release or a source checkout.
It finds `parker.exe` beside the script, under `dist`, or builds it with Cargo.
It then:

1. stops an existing Parker process;
2. installs files under `%LOCALAPPDATA%\Parker`;
3. preserves an existing `settings.env`;
4. downloads FFmpeg when missing;
5. attempts a silent Tesseract installation through `winget`;
6. creates Start menu and startup shortcuts using the embedded application icon;
7. creates an HKCU uninstall entry;
8. starts Parker unless `-NoLaunch` is supplied.

Options:

```powershell
.\install.ps1 -NoStartup
.\install.ps1 -SkipDependencies
.\install.ps1 -NoLaunch
```

## First application launch

Parker independently initializes its data directory and settings file. This
means the portable executable remains usable even when it was not installed by
the script. Process environment variables override values from `settings.env`.

## Updating

Run the newer release's `setup.cmd`. Existing settings are preserved. The
executable and support files are replaced after the running Parker process is
stopped.

## Uninstalling

```powershell
.\uninstall.ps1
```

This removes the executable, shortcuts, and uninstall registration while
preserving settings by default. Use `-RemoveSettings` for a full configuration
cleanup. Recordings are never deleted automatically.
