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

$form = New-Object System.Windows.Forms.Form
$form.Text = "Parker Setup"
$form.StartPosition = "CenterScreen"
$form.FormBorderStyle = "FixedDialog"
$form.MaximizeBox = $false
$form.MinimizeBox = $false
$form.ClientSize = New-Object System.Drawing.Size(560, 430)
$form.Font = New-Object System.Drawing.Font("Segoe UI", 9)

$iconPath = Join-Path $PSScriptRoot "parker.exe"
if (Test-Path $iconPath) {
    try { $form.Icon = [System.Drawing.Icon]::ExtractAssociatedIcon($iconPath) } catch {}
}

$title = New-Object System.Windows.Forms.Label
$title.Text = "Install Parker"
$title.Font = New-Object System.Drawing.Font("Segoe UI", 18, [System.Drawing.FontStyle]::Bold)
$title.AutoSize = $true
$title.Location = New-Object System.Drawing.Point(24, 20)
$form.Controls.Add($title)

$subtitle = New-Object System.Windows.Forms.Label
$subtitle.Text = "Screen capture, QR/OCR, and region recording for Windows."
$subtitle.AutoSize = $true
$subtitle.Location = New-Object System.Drawing.Point(28, 62)
$form.Controls.Add($subtitle)

$startup = New-Object System.Windows.Forms.CheckBox
$startup.Text = "Start Parker with Windows"
$startup.Checked = $true
$startup.AutoSize = $true
$startup.Location = New-Object System.Drawing.Point(32, 105)
$form.Controls.Add($startup)

$dependencies = New-Object System.Windows.Forms.CheckBox
$dependencies.Text = "Install FFmpeg and OCR helper dependencies when missing"
$dependencies.Checked = $true
$dependencies.AutoSize = $true
$dependencies.Location = New-Object System.Drawing.Point(32, 134)
$form.Controls.Add($dependencies)

$launch = New-Object System.Windows.Forms.CheckBox
$launch.Text = "Launch Parker after setup"
$launch.Checked = $true
$launch.AutoSize = $true
$launch.Location = New-Object System.Drawing.Point(32, 163)
$form.Controls.Add($launch)

$status = New-Object System.Windows.Forms.Label
$status.Text = "Ready to install."
$status.AutoSize = $true
$status.Location = New-Object System.Drawing.Point(28, 205)
$form.Controls.Add($status)

$log = New-Object System.Windows.Forms.TextBox
$log.Multiline = $true
$log.ReadOnly = $true
$log.ScrollBars = "Vertical"
$log.BackColor = [System.Drawing.Color]::FromArgb(250, 250, 250)
$log.Location = New-Object System.Drawing.Point(28, 232)
$log.Size = New-Object System.Drawing.Size(504, 126)
$form.Controls.Add($log)

$install = New-Object System.Windows.Forms.Button
$install.Text = "Install"
$install.Location = New-Object System.Drawing.Point(326, 378)
$install.Size = New-Object System.Drawing.Size(98, 32)
$form.Controls.Add($install)

$close = New-Object System.Windows.Forms.Button
$close.Text = "Close"
$close.Location = New-Object System.Drawing.Point(434, 378)
$close.Size = New-Object System.Drawing.Size(98, 32)
$close.Add_Click({ $form.Close() })
$form.Controls.Add($close)

$install.Add_Click({
    Set-ControlEnabled $install $false
    Set-ControlEnabled $close $false
    Set-ControlText $status "Installing..."
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
        if ($process.ExitCode -eq 0) {
            Set-ControlText $status "Parker installed successfully."
        } else {
            Set-ControlText $status "Setup failed. Review the log above."
        }
        Set-ControlEnabled $install $true
        Set-ControlEnabled $close $true
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
        Set-ControlText $status "Setup could not start."
        Set-ControlEnabled $install $true
        Set-ControlEnabled $close $true
    }
})

[void]$form.ShowDialog()
