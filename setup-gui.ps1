Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

function Add-LogLine {
    param(
        [System.Windows.Forms.TextBox]$LogBox,
        [string]$Line
    )

    if ([string]::IsNullOrWhiteSpace($Line)) { return }
    $append = {
        param($box, $text)
        $box.AppendText($text + [Environment]::NewLine)
        $box.SelectionStart = $box.TextLength
        $box.ScrollToCaret()
    }
    if ($LogBox.InvokeRequired) {
        $LogBox.BeginInvoke($append, $LogBox, $Line) | Out-Null
    } else {
        & $append $LogBox $Line
    }
}

function Set-ControlText {
    param(
        [System.Windows.Forms.Control]$Control,
        [string]$Text
    )

    $set = {
        param($target, $value)
        $target.Text = $value
    }
    if ($Control.InvokeRequired) {
        $Control.BeginInvoke($set, $Control, $Text) | Out-Null
    } else {
        & $set $Control $Text
    }
}

function Set-ControlEnabled {
    param(
        [System.Windows.Forms.Control]$Control,
        [bool]$Enabled
    )

    $set = {
        param($target, $value)
        $target.Enabled = $value
    }
    if ($Control.InvokeRequired) {
        $Control.BeginInvoke($set, $Control, $Enabled) | Out-Null
    } else {
        & $set $Control $Enabled
    }
}

$installDir = Join-Path $env:LOCALAPPDATA "Parker"
$versionFile = Join-Path $PSScriptRoot "version.txt"
$version = if (Test-Path $versionFile) { (Get-Content $versionFile -Raw).Trim() } else { "0.4.3" }

$form = New-Object System.Windows.Forms.Form
$form.Text = "Parker Setup"
$form.StartPosition = "CenterScreen"
$form.FormBorderStyle = "FixedDialog"
$form.MaximizeBox = $false
$form.MinimizeBox = $false
$form.ClientSize = New-Object System.Drawing.Size(640, 520)
$form.Font = New-Object System.Drawing.Font("Segoe UI", 9)

$iconPath = Join-Path $PSScriptRoot "parker.exe"
if (Test-Path $iconPath) {
    try { $form.Icon = [System.Drawing.Icon]::ExtractAssociatedIcon($iconPath) } catch {}
}

$title = New-Object System.Windows.Forms.Label
$title.Text = "Install Parker $version"
$title.Font = New-Object System.Drawing.Font("Segoe UI", 19, [System.Drawing.FontStyle]::Bold)
$title.AutoSize = $true
$title.Location = New-Object System.Drawing.Point(28, 22)
$form.Controls.Add($title)

$subtitle = New-Object System.Windows.Forms.Label
$subtitle.Text = "Local screen capture, QR/OCR, region recording, optional audio, and clipboard handoff."
$subtitle.AutoSize = $true
$subtitle.Location = New-Object System.Drawing.Point(32, 66)
$form.Controls.Add($subtitle)

$destination = New-Object System.Windows.Forms.Label
$destination.Text = "Install location: $installDir"
$destination.AutoSize = $true
$destination.Location = New-Object System.Drawing.Point(32, 96)
$form.Controls.Add($destination)

$panel = New-Object System.Windows.Forms.GroupBox
$panel.Text = "Setup options"
$panel.Location = New-Object System.Drawing.Point(28, 126)
$panel.Size = New-Object System.Drawing.Size(584, 134)
$form.Controls.Add($panel)

$startup = New-Object System.Windows.Forms.CheckBox
$startup.Text = "Start Parker with Windows"
$startup.Checked = $true
$startup.AutoSize = $true
$startup.Location = New-Object System.Drawing.Point(18, 28)
$panel.Controls.Add($startup)

$dependencies = New-Object System.Windows.Forms.CheckBox
$dependencies.Text = "Install FFmpeg and OCR helper dependencies when missing"
$dependencies.Checked = $true
$dependencies.AutoSize = $true
$dependencies.Location = New-Object System.Drawing.Point(18, 58)
$panel.Controls.Add($dependencies)

$launch = New-Object System.Windows.Forms.CheckBox
$launch.Text = "Launch Parker after setup"
$launch.Checked = $true
$launch.AutoSize = $true
$launch.Location = New-Object System.Drawing.Point(18, 88)
$panel.Controls.Add($launch)

$tips = New-Object System.Windows.Forms.Label
$tips.Text = "Tip: audio recording is opt-in. Set PARKER_AUDIO_DEVICE in settings when you want microphone/system audio through FFmpeg."
$tips.AutoSize = $false
$tips.Location = New-Object System.Drawing.Point(32, 272)
$tips.Size = New-Object System.Drawing.Size(580, 36)
$form.Controls.Add($tips)

$status = New-Object System.Windows.Forms.Label
$status.Text = "Ready to install."
$status.Font = New-Object System.Drawing.Font("Segoe UI", 9, [System.Drawing.FontStyle]::Bold)
$status.AutoSize = $true
$status.Location = New-Object System.Drawing.Point(32, 318)
$form.Controls.Add($status)

$progress = New-Object System.Windows.Forms.ProgressBar
$progress.Location = New-Object System.Drawing.Point(32, 344)
$progress.Size = New-Object System.Drawing.Size(580, 18)
$progress.Style = "Blocks"
$progress.Value = 0
$form.Controls.Add($progress)

$log = New-Object System.Windows.Forms.TextBox
$log.Multiline = $true
$log.ReadOnly = $true
$log.ScrollBars = "Vertical"
$log.BackColor = [System.Drawing.Color]::FromArgb(250, 250, 250)
$log.Location = New-Object System.Drawing.Point(32, 374)
$log.Size = New-Object System.Drawing.Size(580, 92)
$form.Controls.Add($log)

$openFolder = New-Object System.Windows.Forms.Button
$openFolder.Text = "Open install folder"
$openFolder.Location = New-Object System.Drawing.Point(270, 478)
$openFolder.Size = New-Object System.Drawing.Size(130, 32)
$openFolder.Enabled = $false
$openFolder.Add_Click({
    if (Test-Path $installDir) { Start-Process explorer.exe $installDir }
})
$form.Controls.Add($openFolder)

$install = New-Object System.Windows.Forms.Button
$install.Text = "Install"
$install.Location = New-Object System.Drawing.Point(406, 478)
$install.Size = New-Object System.Drawing.Size(96, 32)
$form.Controls.Add($install)

$close = New-Object System.Windows.Forms.Button
$close.Text = "Close"
$close.Location = New-Object System.Drawing.Point(512, 478)
$close.Size = New-Object System.Drawing.Size(96, 32)
$close.Add_Click({ $form.Close() })
$form.Controls.Add($close)

$install.Add_Click({
    Set-ControlEnabled $install $false
    Set-ControlEnabled $close $false
    Set-ControlEnabled $openFolder $false
    Set-ControlText $status "Installing..."
    $progress.Style = "Marquee"
    $log.Clear()

    $arguments = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "`"$PSScriptRoot\install.ps1`"")
    if (-not $startup.Checked) { $arguments += "-NoStartup" }
    if (-not $dependencies.Checked) { $arguments += "-SkipDependencies" }
    if (-not $launch.Checked) { $arguments += "-NoLaunch" }

    $process = New-Object System.Diagnostics.Process
    $process.StartInfo.FileName = "powershell.exe"
    $process.StartInfo.Arguments = ($arguments -join " ")
    $process.StartInfo.UseShellExecute = $false
    $process.StartInfo.RedirectStandardOutput = $true
    $process.StartInfo.RedirectStandardError = $true
    $process.StartInfo.CreateNoWindow = $true
    $process.EnableRaisingEvents = $true

    $outputHandler = [System.Diagnostics.DataReceivedEventHandler]{
        param($sender, $event)
        if ($event.Data) { Add-LogLine $log $event.Data }
    }
    $errorHandler = [System.Diagnostics.DataReceivedEventHandler]{
        param($sender, $event)
        if ($event.Data) { Add-LogLine $log $event.Data }
    }
    $exitHandler = {
        $done = {
            param($processRef)
            $progress.Style = "Blocks"
            if ($processRef.ExitCode -eq 0) {
                $progress.Value = 100
                $status.Text = "Parker installed successfully."
                $openFolder.Enabled = $true
            } else {
                $progress.Value = 0
                $status.Text = "Setup failed. Review the log above."
            }
            $install.Enabled = $true
            $close.Enabled = $true
        }
        if ($form.InvokeRequired) {
            $form.BeginInvoke($done, $process) | Out-Null
        } else {
            & $done $process
        }
    }

    $process.add_OutputDataReceived($outputHandler)
    $process.add_ErrorDataReceived($errorHandler)
    $process.add_Exited($exitHandler)

    try {
        Add-LogLine $log "Starting Parker installer..."
        [void]$process.Start()
        $process.BeginOutputReadLine()
        $process.BeginErrorReadLine()
    } catch {
        Add-LogLine $log $_.Exception.Message
        $progress.Style = "Blocks"
        $progress.Value = 0
        Set-ControlText $status "Setup could not start."
        Set-ControlEnabled $install $true
        Set-ControlEnabled $close $true
    }
})

[void]$form.ShowDialog()
