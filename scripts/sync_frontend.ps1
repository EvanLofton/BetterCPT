$dist = "C:\Program1\RUST\Projects\BetterCPT_V1\dashboard\dist"
$target = "C:\Program1\RUST\Projects\BetterCPT_V1\host\frontend"
Remove-Item -Recurse -Force $target -ErrorAction SilentlyContinue
robocopy $dist $target /E /NJH /NJS /NFL
Write-Host "Frontend synced OK"
