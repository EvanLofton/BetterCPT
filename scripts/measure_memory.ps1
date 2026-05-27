# BetterCPT Phase 0: 10 Widget Memory Measurement
# Usage: powershell -File scripts/measure_memory.ps1

param(
    [string]$QtExe = "experiments/qt-spike/build/qt-spike.exe",
    [int]$WidgetCount = 10,
    [int]$TargetMB = 60
)

$ErrorActionPreference = "Stop"

Write-Host "=== BetterCPT Memory Measurement ===" -ForegroundColor Cyan
Write-Host "Widget count: $WidgetCount"
Write-Host "Target: < ${TargetMB}MB (Private Working Set)"
Write-Host ""

$QtFullPath = (Resolve-Path $QtExe).Path
if (-not (Test-Path $QtFullPath)) {
    Write-Host "ERROR: qt-spike.exe not found" -ForegroundColor Red
    exit 1
}

# Use .NET Process API for real stdin/stdout pipe control
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $QtFullPath
$psi.RedirectStandardInput = $true
$psi.RedirectStandardOutput = $true
$psi.UseShellExecute = $false
$psi.CreateNoWindow = $true

$process = [System.Diagnostics.Process]::Start($psi)
Write-Host "Qt process started (PID: $($process.Id))"

# Wait for Qt ready signal
Write-Host "Waiting for Qt ready signal..." -ForegroundColor Yellow
while (-not $process.HasExited) {
    $line = $process.StandardOutput.ReadLine()
    if ($line -match '"ready"') {
        Write-Host "  Qt ready signal received"
        break
    }
}

# Send widget creation commands via stdin pipe
Write-Host "Sending createIcon commands for $WidgetCount widgets..." -ForegroundColor Yellow
for ($i = 0; $i -lt $WidgetCount; $i++) {
    $x = $i * 58
    $json = "{`"jsonrpc`":`"2.0`",`"method`":`"createIcon`",`"params`":{`"id`":`"icon_$i`",`"x`":$x,`"y`":16,`"size`":48,`"src`":`"icon.png`"}`"id`":$i}"
    $process.StandardInput.WriteLine($json)
}
$process.StandardInput.Flush()

# Wait for render
Write-Host "Waiting for render..."
Start-Sleep -Seconds 2

# Measure memory
Write-Host "Measuring memory..." -ForegroundColor Yellow

$process.Refresh()

$ws = [math]::Round($process.WorkingSet64 / 1MB, 2)
Write-Host "  WorkingSet64:        ${ws}MB"

$pm = [math]::Round($process.PrivateMemorySize64 / 1MB, 2)
Write-Host "  PrivateMemorySize64: ${pm}MB"

$paged = [math]::Round($process.PagedMemorySize64 / 1MB, 2)
Write-Host "  PagedMemorySize64:   ${paged}MB"

Write-Host ""

# Acceptance check
if ($pm -lt $TargetMB) {
    Write-Host "=== PASS: ${pm}MB < ${TargetMB}MB ===" -ForegroundColor Green
}
else {
    Write-Host "=== FAIL: ${pm}MB >= ${TargetMB}MB ===" -ForegroundColor Red
}

# Cleanup
Write-Host "Stopping Qt process..."
$process.Kill()
$null = $process.WaitForExit(3000)

Write-Host "Done."
