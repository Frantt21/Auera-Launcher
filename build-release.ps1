<#
.SYNOPSIS
    Compila Auera en modo release (binario autónomo con el frontend embebido).

USO
    .\build-release.ps1

    Desde cmd.exe:
        powershell -NoProfile -ExecutionPolicy Bypass -File .\build-release.ps1
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$vcvarsCandidates = @(
    'C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat',
    'C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat'
)

$vcvars = $vcvarsCandidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (-not $vcvars) {
    throw 'No se encontro vcvars64.bat de Visual Studio 2026 (x64).'
}

Write-Host "Inicializando entorno MSVC: $vcvars" -ForegroundColor Cyan

$lines = & cmd.exe /d /c ('call "' + $vcvars + '" && set')
if ($LASTEXITCODE -ne 0) {
    throw "vcvars64.bat fallo con codigo de salida $LASTEXITCODE."
}

foreach ($line in $lines) {
    if ($line -notmatch '^([^=]+)=(.*)$') { continue }
    $name = $Matches[1]
    if ($name.StartsWith('=')) { continue }
    Set-Item -Path "Env:$name" -Value $Matches[2]
}

Write-Host "Compilando frontend (npm run build)..." -ForegroundColor Cyan
& npm run build
if ($LASTEXITCODE -ne 0) {
    Write-Error "npm run build fallo con codigo de salida $LASTEXITCODE."
    exit $LASTEXITCODE
}

Write-Host "Compilando Rust en release..." -ForegroundColor Cyan
& cargo build --release --manifest-path src-tauri/Cargo.toml --features custom-protocol
if ($LASTEXITCODE -ne 0) {
    Write-Error "cargo build --release fallo con codigo de salida $LASTEXITCODE."
    exit $LASTEXITCODE
}

Write-Host "Build release completado." -ForegroundColor Green
