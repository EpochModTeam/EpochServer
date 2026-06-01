<#
.SYNOPSIS
    Builds a clean EpochServer release containing ONLY the x64 variants.

.DESCRIPTION
    This script ensures that only epochserver_x64.dll (Windows) or
    libepochserver_x64.so (Linux) artifacts are produced.
    Any non-x64 named files are deleted before the final output.

    Run from the EpochServer/ directory.

.EXAMPLE
    ./scripts/build-x64-release.ps1
#>

[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

Write-Host "=== Building clean x64-only EpochServer release ===" -ForegroundColor Cyan

# Build in release mode
cargo build --release

if ($LASTEXITCODE -ne 0) {
    Write-Error "Build failed"
    exit 1
}

$releaseDir = "target/release"

Write-Host "`nCleaning any non-x64 artifacts..." -ForegroundColor Yellow

# Remove any non-x64 named files that Cargo might have produced
$nonX64Patterns = @(
    "epochserver.dll",
    "epochserver.exe",
    "epochserver.so",
    "libepochserver.so",
    "epochserver_x86*.dll",
    "*epochserver*[!xX]64*"
)

foreach ($pattern in $nonX64Patterns) {
    Get-ChildItem -Path $releaseDir -Filter $pattern -ErrorAction SilentlyContinue | 
        Where-Object { $_.Name -notmatch 'x64' -and $_.Name -notmatch 'X64' } |
        Remove-Item -Force -ErrorAction SilentlyContinue
}

Write-Host "`nFinal x64 artifacts:" -ForegroundColor Green
Get-ChildItem -Path $releaseDir -Filter "*epochserver_x64*" -ErrorAction SilentlyContinue |
    Select-Object Name, Length, LastWriteTime |
    Format-Table -AutoSize

Write-Host "`n=== Done. Only x64 variants remain. ===" -ForegroundColor Cyan
Write-Host "Main file: $releaseDir\epochserver_x64.dll" -ForegroundColor Green