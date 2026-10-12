$ErrorActionPreference = 'Continue'
$tmp = $env:TEMP
$init = Join-Path $tmp 'rustup-init.exe'

Write-Host "== downloading rustup-init.exe"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
Invoke-WebRequest -Uri 'https://win.rustup.rs/x86_64' -OutFile $init -UseBasicParsing
Write-Host "   saved: $init  ($((Get-Item $init).Length) bytes)"

Write-Host "== running rustup-init -y (stable, default profile)"
& $init -y --default-toolchain stable --profile default
Write-Host "   rustup-init exit code: $LASTEXITCODE"

$cargo = Join-Path $env:USERPROFILE '.cargo\bin'
Write-Host "== versions"
& (Join-Path $cargo 'rustc.exe') --version
& (Join-Path $cargo 'cargo.exe') --version

Write-Host "== toolchain list"
& (Join-Path $cargo 'rustup.exe') toolchain list
Write-Host "DONE"
