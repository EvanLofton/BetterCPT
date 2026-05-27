$file = "dist/index.html"
$content = Get-Content $file -Raw
$content = $content -replace 'type="module" crossorigin ', ''
Set-Content $file -Value $content -NoNewline
Write-Host "stripped type=module from index.html"
