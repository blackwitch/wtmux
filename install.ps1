#Requires -Version 5.1
<#
.SYNOPSIS
    Install wtmux - Windows terminal multiplexer
.DESCRIPTION
    Builds from source (if in repo) or copies pre-built binaries to
    $env:LOCALAPPDATA\wtmux and adds the directory to the user PATH.
#>
param(
    [switch]$SkipBuild,
    [string]$InstallDir = "$env:LOCALAPPDATA\wtmux"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Write-Host "=== wtmux installer ===" -ForegroundColor Cyan

# --- Determine source of binaries ---
$repoRoot = $PSScriptRoot
$releaseBin = Join-Path $repoRoot "target\release"
$clientExe = Join-Path $releaseBin "wtmux-client.exe"
$serverExe = Join-Path $releaseBin "wtmux-server.exe"

if (-not $SkipBuild) {
    # Check if we're in the source tree
    $cargoToml = Join-Path $repoRoot "Cargo.toml"
    if (Test-Path $cargoToml) {
        Write-Host "Building release binaries..." -ForegroundColor Yellow
        $cargo = Get-Command cargo -ErrorAction SilentlyContinue
        if (-not $cargo) {
            # Try common location
            $cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
            if (-not (Test-Path $cargo)) {
                Write-Error "cargo not found. Install Rust from https://rustup.rs/ or use -SkipBuild with pre-built binaries."
                exit 1
            }
        } else {
            $cargo = $cargo.Source
        }
        Push-Location $repoRoot
        try {
            & $cargo build --release
            if ($LASTEXITCODE -ne 0) {
                Write-Error "Build failed."
                exit 1
            }
        } finally {
            Pop-Location
        }
    }
}

# Verify binaries exist
if (-not (Test-Path $clientExe) -or -not (Test-Path $serverExe)) {
    Write-Error "Binaries not found at $releaseBin. Build first or place binaries there."
    exit 1
}

# --- Create install directory ---
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Write-Host "Created $InstallDir" -ForegroundColor Green
}

# --- Copy binaries ---
Write-Host "Copying binaries to $InstallDir..." -ForegroundColor Yellow

# wtmux-client.exe -> wtmux.exe (convenience alias)
Copy-Item -Path $clientExe -Destination (Join-Path $InstallDir "wtmux.exe") -Force
# Also keep original name
Copy-Item -Path $clientExe -Destination (Join-Path $InstallDir "wtmux-client.exe") -Force
Copy-Item -Path $serverExe -Destination (Join-Path $InstallDir "wtmux-server.exe") -Force

Write-Host "  wtmux.exe (client)" -ForegroundColor Green
Write-Host "  wtmux-client.exe" -ForegroundColor Green
Write-Host "  wtmux-server.exe" -ForegroundColor Green

# --- Add to user PATH ---
$currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($currentPath -split ';' | Where-Object { $_ -eq $InstallDir }) {
    Write-Host "PATH already contains $InstallDir" -ForegroundColor Gray
} else {
    $newPath = "$currentPath;$InstallDir"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    # Also update current session
    $env:Path = "$env:Path;$InstallDir"
    Write-Host "Added $InstallDir to user PATH" -ForegroundColor Green
}

# --- Create default config if missing ---
$configPath = Join-Path $env:USERPROFILE ".wtmux.conf"
if (-not (Test-Path $configPath)) {
    @"
# wtmux configuration
# See https://github.com/petermn2/wtmux for documentation

# Prefix key (default: Ctrl-b)
# prefix = "C-b"

# Default shell
# shell = "powershell.exe"
"@ | Set-Content -Path $configPath -Encoding UTF8
    Write-Host "Created default config at $configPath" -ForegroundColor Green
} else {
    Write-Host "Config already exists at $configPath" -ForegroundColor Gray
}

# --- Done ---
Write-Host ""
Write-Host "=== Installation complete ===" -ForegroundColor Cyan
Write-Host "Run 'wtmux' to start. You may need to restart your terminal for PATH changes." -ForegroundColor White
