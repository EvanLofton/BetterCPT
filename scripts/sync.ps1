$src = "C:\Program1\RUST\Projects\BetterCPT_V1\dashboard\dist"
$dst = "C:\Program1\RUST\Projects\BetterCPT_V1\host\frontend"

Remove-Item -Recurse -Force $dst -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $dst | Out-Null

Get-ChildItem $src | ForEach-Object {
    Copy-Item -Recurse -Force $_.FullName "$dst\"
}

Write-Host "=== Synced files ==="
Get-ChildItem -Recurse $dst | ForEach-Object { Write-Host $_.FullName }
