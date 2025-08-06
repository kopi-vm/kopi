# Copyright 2025 dentsusoken
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# Build Kopi Bundle with Visual C++ Runtime
# This script builds a WiX Bundle that includes both VC++ Runtime and Kopi MSI
#
# Parameters:
#   -KopiMsiPath: Optional path to pre-built MSI file (e.g., from CI/CD pipeline)
#   -SkipMsiBuild: Skip building the MSI (useful when using -KopiMsiPath)
#   -SkipVCDownload: Skip downloading VC++ redistributable

param(
    [string]$Configuration = "Release",
    [string]$Version = "0.0.3",
    [string]$OutputDir = (Join-Path $PSScriptRoot "output"),
    [string]$KopiMsiPath = "",  # Optional: path to pre-built MSI
    [switch]$SkipMsiBuild,
    [switch]$SkipVCDownload
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host " Kopi Bundle Build Script" -ForegroundColor Cyan  
Write-Host "========================================" -ForegroundColor Cyan

# Get script directory
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ProjectRoot = Join-Path $ScriptDir ".." | Join-Path -ChildPath ".." | Resolve-Path

# Step 1: Download VC++ Redistributable
if (-not $SkipVCDownload) {
    Write-Host "`nStep 1: Downloading VC++ Redistributable..." -ForegroundColor Yellow
    $vcRedistOutput = Join-Path $ScriptDir "redist"
    & "$ScriptDir\download-vcredist.ps1" -OutputPath $vcRedistOutput
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to download VC++ Redistributable"
    }
} else {
    Write-Host "`nStep 1: Skipping VC++ download" -ForegroundColor Yellow
}

# Step 2: Build Kopi MSI
if (-not $SkipMsiBuild) {
    Write-Host "`nStep 2: Building Kopi MSI..." -ForegroundColor Yellow
    & "$ScriptDir\build.ps1" -Version $Version
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to build Kopi MSI"
    }
} else {
    Write-Host "`nStep 2: Skipping MSI build" -ForegroundColor Yellow
}

# Step 3: Build Bundle
Write-Host "`nStep 3: Building Bundle..." -ForegroundColor Yellow

# Check for required files
# Use provided MSI path or default to expected location
if ($KopiMsiPath) {
    $msiPath = $KopiMsiPath
    Write-Host "Using provided MSI path: $msiPath" -ForegroundColor Cyan
} else {
    $msiPath = Join-Path $ScriptDir "output\en-us\kopi-$Version-windows-x64.msi"
}
$vcRedistPath = Join-Path $ScriptDir "redist\vc_redist.x64.exe"

if (-not (Test-Path $msiPath)) {
    Write-Error "MSI not found at: $msiPath"
    if (-not $KopiMsiPath) {
        Write-Host "Tip: You can specify a custom MSI path with -KopiMsiPath parameter" -ForegroundColor Yellow
    }
    exit 1
}

if (-not (Test-Path $vcRedistPath)) {
    Write-Error "VC++ Redistributable not found at: $vcRedistPath"
    Write-Host "Run download-vcredist.ps1 first" -ForegroundColor Yellow
    exit 1
}

Write-Host "Found MSI: $msiPath" -ForegroundColor Green
Write-Host "Found VC++ Redist: $vcRedistPath" -ForegroundColor Green

# Build bundle using static project file
Write-Host "`nBuilding bundle with MSBuild..." -ForegroundColor Cyan

# Check if we can use dotnet build
if (Get-Command "dotnet" -ErrorAction SilentlyContinue) {
    $bundleProjectPath = Join-Path $ScriptDir "kopi-bundle.wixproj"
    
    # Build with MSBuild properties
    $buildArgs = @(
        $bundleProjectPath,
        "-c", $Configuration,
        "-p:Version=$Version",
        "-p:VCRedistPath=$vcRedistPath",
        "-p:KopiMsiPath=$msiPath"
    )
    
    dotnet build @buildArgs
        
    # Note: dotnet build may return warnings but still succeed
    # Check if the output file exists instead of relying only on exit code
} else {
    Write-Error "dotnet SDK not found. Please install .NET SDK."
    exit 1
}

# Wait a moment for the file to be written
Start-Sleep -Milliseconds 500

$bundleOutput = Join-Path $OutputDir "kopi-bundle-with-vcredist-$Version-windows-x64.exe"
if (Test-Path $bundleOutput) {
    Write-Host "`n✓ Bundle built successfully!" -ForegroundColor Green
    Write-Host "  Output: $bundleOutput" -ForegroundColor Gray
    
    $bundleSize = (Get-Item $bundleOutput).Length
    Write-Host "  Size: $([math]::Round($bundleSize / 1MB, 2)) MB" -ForegroundColor Gray
    
    Write-Host "`nThe bundle includes:" -ForegroundColor Cyan
    Write-Host "  - Visual C++ 2015-2022 Redistributable (x64)" -ForegroundColor White
    Write-Host "  - Kopi $Version MSI installer" -ForegroundColor White
    Write-Host "`nVC++ Runtime will be installed automatically if needed." -ForegroundColor Green
} else {
    Write-Error "Bundle output not found at expected location"
}

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host " Bundle Build Complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
