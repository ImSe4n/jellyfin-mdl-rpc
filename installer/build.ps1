# Builds target\installer\Jellyfin-MDL-RPC-Setup.exe. Used by the release workflow
# and for local builds:
#   pip install pyinstaller -r mdl\requirements.txt
#   .\installer\build.ps1 -Version 1.4.0
# Needs Rust, a Python with PyInstaller + curl_cffi, and Inno Setup 6.
param(
    [Parameter(Mandatory = $true)][string]$Version,
    [string]$Python = "python",
    [string]$Iscc = ""
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

function Invoke-Step([string]$Name, [scriptblock]$Command) {
    Write-Host "==> $Name"
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$Name failed with exit code $LASTEXITCODE" }
}

if (-not $Iscc) {
    $candidates = @(
        "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
        "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
    )
    $Iscc = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $Iscc) { throw "Inno Setup 6 not found; pass -Iscc <path to ISCC.exe>" }
}

Invoke-Step "cargo build" { cargo build --workspace --locked --release }

$work = Join-Path $env:TEMP "jellyfin-mdl-rpc-pyinstaller"
Invoke-Step "PyInstaller mdl_fetch.exe" {
    & $Python -m PyInstaller --noconfirm --onefile --name mdl_fetch --collect-all curl_cffi `
        --distpath target\mdl_fetch --workpath $work --specpath $work mdl\mdl_fetch.py
}

Invoke-Step "Inno Setup" { & $Iscc "/DAppVersion=$Version" installer\jellyfin-mdl-rpc.iss }

Write-Host "Built target\installer\Jellyfin-MDL-RPC-Setup.exe"
