$hostProc = Get-Process bettercpt-host -ErrorAction SilentlyContinue
$qtProc = Get-Process widget-qt -ErrorAction SilentlyContinue

$hostMB = 0
$qtMB = 0
$totalMB = 0

if ($hostProc) {
    $hostMB = [math]::Round($hostProc.WorkingSet64 / 1MB, 1)
    Write-Host "Host: ${hostMB}MB (Private Working Set)"
}
if ($qtProc) {
    $qtMB = [math]::Round($qtProc.WorkingSet64 / 1MB, 1)
    Write-Host "Qt:   ${qtMB}MB (Private Working Set)"
}
if ($hostProc -or $qtProc) {
    $totalMB = [math]::Round(($hostProc.WorkingSet64 + $qtProc.WorkingSet64) / 1MB, 1)
    Write-Host "Total: ${totalMB}MB"
    if ($totalMB -lt 25) {
        Write-Host "PASS: under 25MB target" -ForegroundColor Green
    } else {
        Write-Host "FAIL: exceeds 25MB target" -ForegroundColor Red
    }
} else {
    Write-Host "No BetterCPT processes found" -ForegroundColor Yellow
}
