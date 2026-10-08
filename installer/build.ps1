# Builds target\installer\Jellyfin-MDL-RPC-Setup.exe. Used by the release workflow
# and for local builds:
#   pip install pyinstaller -r mdl\requirements.txt
#   .\installer\build.ps1 -Version 1.4.0
# Needs Rust, a Python with PyInstaller + curl_cffi, and Inno Setup 6.
param(
    # Defaults to the version in jellyfin-rpc-cli/Cargo.toml.
    [string]$Version = "",
    [string]$Python = "python",
    [string]$Iscc = ""
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (-not $Version) {
    $match = Select-String -Path jellyfin-rpc-cli\Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"' |
        Select-Object -First 1
    if (-not $match) { throw "No version in jellyfin-rpc-cli\Cargo.toml; pass -Version" }
    $Version = $match.Matches[0].Groups[1].Value
}
Write-Host "Building version $Version"

function Invoke-Step([string]$Name, [scriptblock]$Command) {
    Write-Host "==> $Name"
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$Name failed with exit code $LASTEXITCODE" }
}

function Find-Iscc {
    $onPath = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($onPath) { return $onPath.Source }

    # Inno Setup records where it was installed, per user or machine-wide.
    $uninstallKeys = @(
        "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Inno Setup 6_is1",
        "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Inno Setup 6_is1",
        "HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Inno Setup 6_is1"
    )
    $candidates = @(
        $uninstallKeys | ForEach-Object {
            $location = (Get-ItemProperty $_ -ErrorAction SilentlyContinue).InstallLocation
            if ($location) { Join-Path $location "ISCC.exe" }
        }
        "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe"
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
        "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
    )
    return $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
}

if (-not $Iscc) {
    $Iscc = Find-Iscc
    if (-not $Iscc) {
        throw ("Inno Setup 6 not found. Install it with:`n" +
            "    winget install --id JRSoftware.InnoSetup -e --scope user`n" +
            "or pass -Iscc <path to ISCC.exe>")
    }
}
Write-Host "Using $Iscc"

Invoke-Step "cargo build" { cargo build --workspace --locked --release }

$work = Join-Path $env:TEMP "jellyfin-mdl-rpc-pyinstaller"
Invoke-Step "PyInstaller mdl_fetch.exe" {
    & $Python -m PyInstaller --noconfirm --onefile --name mdl_fetch --collect-all curl_cffi `
        --distpath target\mdl_fetch --workpath $work --specpath $work mdl\mdl_fetch.py
}

Invoke-Step "Inno Setup" { & $Iscc "/DAppVersion=$Version" installer\jellyfin-mdl-rpc.iss }

Write-Host "Built target\installer\Jellyfin-MDL-RPC-Setup.exe"
