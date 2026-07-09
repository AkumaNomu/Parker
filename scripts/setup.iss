; Parker GUI Installer — Inno Setup 6
; Build: ISCC.exe scripts\setup.iss

#define MyAppName "Parker"
#define MyAppPublisher "Akuma Nomu"
#define MyAppURL "https://github.com/AkumaNomu/Parker"
#define MyAppExeName "parker.exe"

#define FileHandle
#define FileLine
#define MyAppVersion "0.4.0"
#ifndef MyAppVersion
  #expr ParseVersion("..\dist\parker.exe", MyAppVersion, \)
#endif

[Setup]
AppId={{A3E7F13C-1E5D-4B2C-9A8D-6F0B2E4C8A1D}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
VersionInfoVersion={#MyAppVersion}
DefaultDirName={localappdata}\Parker
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
DisableDirPage=no
AllowNoIcons=yes
OutputDir=..\release
OutputBaseFilename=parker-setup-{#MyAppVersion}-windows-x64
SetupIconFile=..\assets\parker.ico
UninstallDisplayIcon={app}\parker.exe
Compression=lzma2/ultra64
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
CloseApplications=yes
RestartApplications=no
ShowComponentSizes=no
InfoBeforeFile=
WizardImageFile=
WizardSmallImageFile=
DisableWelcomePage=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Messages]
WelcomeLabel2=This will install [name/ver] on your computer.%n%nParker is a hotkey-first Windows capture utility. Select any screen region for smart OCR, QR decoding, table/code/text recognition, screen recording, clip recording, or scroll capture — all placed straight onto your clipboard.

[Types]
Name: "full"; Description: "Full installation"
Name: "custom"; Description: "Custom installation"; Flags: iscustom

[Components]
Name: "core"; Description: "Parker core application"; Types: full custom; Flags: fixed
Name: "ocr"; Description: "OCR support (Tesseract)"; Types: full; Flags: disablenouninstallwarning
Name: "ffmpeg"; Description: "FFmpeg runtime for recording"; Types: full; Flags: disablenouninstallwarning
Name: "web"; Description: "Webpage extraction (site retriever)"; Types: full; Flags: disablenouninstallwarning

[Tasks]
Name: "startup"; Description: "Start Parker when Windows starts"; GroupDescription: "Startup options:"; Components: core
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Additional shortcuts:"; Components: core

[Files]
Source: "..\dist\parker.exe"; DestDir: "{app}"; Flags: ignoreversion; Components: core
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion; Components: core
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion; Components: core
Source: "..\settings.env.example"; DestDir: "{app}"; DestName: "settings.env.example"; Flags: ignoreversion; Components: core
Source: "..\scripts\install-ffmpeg.ps1"; DestDir: "{app}\scripts"; Flags: ignoreversion; Components: ffmpeg
Source: "..\uninstall.ps1"; DestDir: "{app}"; Flags: ignoreversion; Components: core

[Dirs]
Name: "{app}\logs"; Components: core

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; Components: core
Name: "{group}\Open recordings"; Filename: "{app}\{#MyAppExeName}"; Parameters: "recordings"; WorkingDir: "{app}"; Components: core
Name: "{group}\Settings"; Filename: "{localappdata}\Parker\settings.env"; Components: core
Name: "{group}\Uninstall {#MyAppName}"; Filename: "{uninstallexe}"; Components: core
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; Tasks: desktopicon; Components: core
Name: "{userstartup}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; Tasks: startup; Components: core

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch {#MyAppName}"; Flags: postinstall nowait skipifsilent unchecked; Components: core
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -Command ""& '{app}\scripts\install-ffmpeg.ps1'"""; Flags: runhidden; Components: ffmpeg; StatusMsg: "Installing FFmpeg runtime..."
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -Command ""try {{ winget install --id tesseract-ocr.tesseract --exact --silent --accept-package-agreements --accept-source-agreements 2>$null }} catch {{}}"""; Flags: runhidden; Components: ocr; StatusMsg: "Installing Tesseract OCR..."

[UninstallRun]
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\uninstall.ps1"""; Flags: runhidden; RunOnceId: "UninstallParker"

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Uninstall\{#MyAppName}"; ValueType: string; ValueName: "DisplayName"; ValueData: "{#MyAppName}"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Uninstall\{#MyAppName}"; ValueType: string; ValueName: "DisplayVersion"; ValueData: "{#MyAppVersion}"
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Uninstall\{#MyAppName}"; ValueType: string; ValueName: "Publisher"; ValueData: "{#MyAppPublisher}"
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Uninstall\{#MyAppName}"; ValueType: string; ValueName: "DisplayIcon"; ValueData: "{app}\parker.exe"
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Uninstall\{#MyAppName}"; ValueType: string; ValueName: "UninstallString"; ValueData: """{uninstallexe}"""
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Uninstall\{#MyAppName}"; ValueType: dword; ValueName: "NoModify"; ValueData: "1"
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Uninstall\{#MyAppName}"; ValueType: dword; ValueName: "NoRepair"; ValueData: "1"

[Code]
procedure CurStepChanged(CurStep: TSetupStep);
var
  SettingsPath: string;
  SettingsContent: string;
begin
  if CurStep = ssPostInstall then
  begin
    SettingsPath := ExpandConstant('{localappdata}\Parker\settings.env');
    if not FileExists(SettingsPath) then
    begin
      SettingsContent :=
        '# Parker settings' + #13#10 +
        '# Lines use KEY=VALUE. Restart Parker after editing.' + #13#10 +
        '' + #13#10 +
        '# --- OCR ---' + #13#10 +
        'PARKER_OCR_LANG=eng' + #13#10 +
        'PARKER_OCR_PSM=6' + #13#10 +
        'PARKER_OCR_MODE=auto' + #13#10 +
        'PARKER_QR_AUTO_OPEN=1' + #13#10 +
        'PARKER_KEEP_OCR_CAPTURE=0' + #13#10 +
        '' + #13#10 +
        '# --- Recording ---' + #13#10 +
        'PARKER_RECORD_FPS=30' + #13#10 +
        'PARKER_COMPRESSION=balanced' + #13#10 +
        'PARKER_VIDEO_ENCODER=auto' + #13#10 +
        'PARKER_RING_SECONDS=45' + #13#10 +
        '# PARKER_MAX_WIDTH=1920' + #13#10 +
        '# PARKER_MAX_HEIGHT=1080' + #13#10 +
        '# PARKER_POST_CRF=24' + #13#10 +
        '# PARKER_POST_PRESET=medium' + #13#10 +
        '' + #13#10 +
        '# --- Custom hotkeys (F1-F12, or single letter) ---' + #13#10 +
        '# PARKER_HOTKEY_OCR=F8' + #13#10 +
        '# PARKER_HOTKEY_RECORD=F9' + #13#10 +
        '# PARKER_HOTKEY_CLIP=F7' + #13#10 +
        '# PARKER_HOTKEY_SCROLL=F11' + #13#10 +
        '# PARKER_HOTKEY_FOLDER=F10' + #13#10 +
        '# PARKER_HOTKEY_QUIT=F12' + #13#10 +
        '# PARKER_HOTKEY_WEB=F6';
      SaveStringToFile(SettingsPath, SettingsContent, False);
    end;
  end;
end;

procedure CurPageChanged(CurPageID: Integer);
begin
  if CurPageID = wpFinished then
  begin
    WizardForm.FinishedLabel.Caption :=
      'Parker is installed and ready.' + #13#10 + #13#10 +
      'Notification-area icon:' + #13#10 +
      '  Right-click for capture, recording, settings, and exit.' + #13#10 +
      '  Double-click to open recordings.' + #13#10 + #13#10 +
      'Hotkeys (press then drag a region):' + #13#10 +
      '  Ctrl+Shift+F6   Extract a webpage (copy URL first)' + #13#10 +
      '  Ctrl+Shift+F7   Record last 30-60s clip' + #13#10 +
      '  Ctrl+Shift+F8   Smart capture: QR, table, code, or text' + #13#10 +
      '  Ctrl+Shift+F9   Record a screen region' + #13#10 +
      '  Ctrl+Shift+F10  Open recordings folder' + #13#10 +
      '  Ctrl+Shift+F11  Scroll capture' + #13#10 +
      '  Ctrl+Shift+F12  Exit Parker' + #13#10 + #13#10 +
      'Esc or right-click cancels a region selector.' + #13#10 + #13#10 +
      'OCR (Tesseract) is required for text, code, and table' + #13#10 +
      'recognition. QR and recording work without it.';
  end;
end;
