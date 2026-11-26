<#
.SYNOPSIS
    Compila Auera inicializando antes el entorno de MSVC (x64) de Visual Studio.

USO
    .\build.ps1                  -> cargo build
    .\build.ps1 --release        -> cargo build --release
    .\build.ps1 --target x86_64-pc-windows-msvc

    Desde cmd.exe:
        powershell -NoProfile -ExecutionPolicy Bypass -File .\build.ps1 --release

NOTA
    Se prefiere Build Tools 2026 porque la instalacion de Community tiene el
    toolset 14.52 incompleto (falta vcruntime.h y lib\x64\msvcrt.lib).
#>
[CmdletBinding()]
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$CargoArgs = @()
)

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

# vcvars64.bat es un .bat: se ejecuta en cmd.exe y se importa el entorno resultante.
$lines = & cmd.exe /d /c ('call "' + $vcvars + '" && set')
if ($LASTEXITCODE -ne 0) {
    throw "vcvars64.bat fallo con codigo de salida $LASTEXITCODE."
}

foreach ($line in $lines) {
    if ($line -notmatch '^([^=]+)=(.*)$') { continue }
    $name = $Matches[1]
    if ($name.StartsWith('=')) { continue } # variables internas de cmd (=C:, =ExitCode, ...)
    Set-Item -Path "Env:$name" -Value $Matches[2]
}

# Por defecto se compila el crate de Tauri (la interfaz principal).
if (-not ($CargoArgs -match '--manifest-path')) {
    $CargoArgs = @('--manifest-path', 'src-tauri/Cargo.toml') + $CargoArgs
}

$detalle = if ($CargoArgs.Count) { ' ' + ($CargoArgs -join ' ') } else { '' }
Write-Host "cargo build$detalle" -ForegroundColor Cyan

& cargo build @CargoArgs
if ($LASTEXITCODE -ne 0) {
    Write-Error "cargo build fallo con codigo de salida $LASTEXITCODE."
    exit $LASTEXITCODE
}
