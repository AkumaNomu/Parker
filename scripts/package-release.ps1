$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

& (Join-Path (Get-Location) "build.ps1")

$version = (Select-String -Path "Cargo.toml" -Pattern '^version\s*=\s*"([^\"]+)"').Matches[0].Groups[1].Value
$releaseDir = Join-Path (Get-Location) "release"
$staging = Join-Path $releaseDir "parker-$version-windows-x64"
$archive = Join-Path $releaseDir "parker-$version-windows-x64.zip"
$portable = Join-Path $releaseDir "parker-$version-windows-x64.exe"
$installer = Join-Path $releaseDir "parker-setup-$version-windows-x64.exe"
$sed = Join-Path $releaseDir "parker-setup-$version.sed"

Remove-Item $staging -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item $archive -Force -ErrorAction SilentlyContinue
Remove-Item $portable -Force -ErrorAction SilentlyContinue
Remove-Item $installer -Force -ErrorAction SilentlyContinue
Remove-Item $sed -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $staging | Out-Null

Copy-Item "dist\parker.exe" $staging
Copy-Item "dist\parker.exe" $portable
Copy-Item "README.md" $staging
Copy-Item "LICENSE" $staging
Copy-Item "install.ps1" $staging
Copy-Item "uninstall.ps1" $staging
Copy-Item "setup.cmd" $staging
Copy-Item "setup-gui.ps1" $staging
Copy-Item "settings.env.example" $staging
$version | Set-Content -Path (Join-Path $staging "version.txt") -Encoding ASCII

Compress-Archive -Path "$staging\*" -DestinationPath $archive -Force

$sourceDir = "$staging\"
$installerPath = $installer
$sourcePath = $sourceDir
$sedContent = @"
[Version]
Class=IEXPRESS
SEDVersion=3

[Options]
PackagePurpose=InstallApp
ShowInstallProgramWindow=1
HideExtractAnimation=0
UseLongFileName=1
InsideCompressed=0
CAB_FixedSize=0
CAB_ResvCodeSigning=0
RebootMode=N
InstallPrompt=
DisplayLicense=
FinishMessage=
TargetName=$installerPath
FriendlyName=Parker $version Setup
AppLaunched=setup.cmd
PostInstallCmd=<None>
AdminQuietInstCmd=setup.cmd
UserQuietInstCmd=setup.cmd
SourceFiles=SourceFiles

[SourceFiles]
SourceFiles0=$sourcePath

[SourceFiles0]
%FILE0%=
%FILE1%=
%FILE2%=
%FILE3%=
%FILE4%=
%FILE5%=
%FILE6%=
%FILE7%=
%FILE8%=

[Strings]
FILE0="parker.exe"
FILE1="README.md"
FILE2="LICENSE"
FILE3="install.ps1"
FILE4="uninstall.ps1"
FILE5="setup.cmd"
FILE6="settings.env.example"
FILE7="version.txt"
FILE8="setup-gui.ps1"
"@
$sedContent | Set-Content -Path $sed -Encoding ASCII
$sedArgument = Resolve-Path -Relative $sed
& "$env:SystemRoot\System32\iexpress.exe" /N /Q $sedArgument
$lastLength = -1
$stableSamples = 0
for ($attempt = 0; $attempt -lt 240 -and $stableSamples -lt 8; $attempt++) {
    if (Test-Path $installer) {
        $length = (Get-Item $installer).Length
        if ($length -gt 0 -and $length -eq $lastLength) {
            $stableSamples++
        } else {
            $stableSamples = 0
            $lastLength = $length
        }
    }
    Start-Sleep -Milliseconds 250
}
if (-not (Test-Path $installer) -or $stableSamples -lt 8) {
    throw "IExpress could not create $installer"
}

Remove-Item $staging -Recurse -Force
Remove-Item $sed -Force

foreach ($asset in @($archive, $portable, $installer)) {
    $hash = (Get-FileHash -Algorithm SHA256 $asset).Hash.ToLowerInvariant()
    $checksum = "$asset.sha256"
    "$hash  $(Split-Path $asset -Leaf)" | Set-Content -Path $checksum -Encoding ASCII
    Write-Host "Created $asset"
    Write-Host "Created $checksum"
}
