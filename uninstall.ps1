#Requires -Version 5.1
<#
.SYNOPSIS
    Uninstall wtmux
.DESCRIPTION
    Removes wtmux binaries and removes the install directory from user PATH.
    Configuration files are preserved.
#>
param(
    [string]$InstallDir = "$env:LOCALAPPDATA\wtmux"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Write-Host "=== wtmux uninstaller ===" -ForegroundColor Cyan

# --- Remove binaries ---
if (Test-Path $InstallDir) {
    $binaries = @("wtmux.exe", "wtmux-client.exe", "wtmux-server.exe")
    foreach ($bin in $binaries) {
        $path = Join-Path $InstallDir $bin
        if (Test-Path $path) {
            Remove-Item -Path $path -Force
            Write-Host "Removed $bin" -ForegroundColor Yellow
        }
    }

    # Remove directory if empty
    $remaining = Get-ChildItem -Path $InstallDir -ErrorAction SilentlyContinue
    if (-not $remaining) {
        Remove-Item -Path $InstallDir -Force
        Write-Host "Removed $InstallDir" -ForegroundColor Yellow
    } else {
        Write-Host "$InstallDir not empty, keeping directory" -ForegroundColor Gray
    }
} else {
    Write-Host "Install directory $InstallDir not found" -ForegroundColor Gray
}

# --- Remove from user PATH ---
$currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
$parts = $currentPath -split ';' | Where-Object { $_ -ne $InstallDir -and $_ -ne "" }
$newPath = $parts -join ';'

if ($newPath -ne $currentPath) {
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "Removed $InstallDir from user PATH" -ForegroundColor Green
} else {
    Write-Host "PATH did not contain $InstallDir" -ForegroundColor Gray
}

# --- Config note ---
$configPath = Join-Path $env:USERPROFILE ".wtmux.conf"
if (Test-Path $configPath) {
    Write-Host ""
    Write-Host "Note: Configuration file preserved at $configPath" -ForegroundColor Gray
    Write-Host "Delete it manually if no longer needed." -ForegroundColor Gray
}

Write-Host ""
Write-Host "=== Uninstall complete ===" -ForegroundColor Cyan
