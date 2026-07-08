param(
    [string]$Destination = 'C:\ffmpeg'
)

Write-Host "Downloading ffmpeg (release essentials) and installing to $Destination"
$zip = Join-Path $env:TEMP 'ffmpeg.zip'
$url = 'https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip'

Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing -ErrorAction Stop
Write-Host "Extracting..."
Expand-Archive -Path $zip -DestinationPath "$env:TEMP\ffmpeg_tmp" -Force
$dir = Get-ChildItem "$env:TEMP\ffmpeg_tmp" | Where-Object {$_.PSIsContainer} | Select-Object -First 1
if (Test-Path $Destination) { Remove-Item -Recurse -Force $Destination }
Move-Item -Path $dir.FullName -Destination $Destination -Force
Remove-Item -Recurse -Force "$env:TEMP\ffmpeg_tmp"
Remove-Item -Force $zip
Write-Host "ffmpeg installed to" (Join-Path $Destination 'bin\ffmpeg.exe')
