# Kopi MSI Build Script
# This script builds the Kopi MSI installer using WiX Toolset v6
# 
# NOTE: This builds a standalone MSI that requires Visual C++ Runtime.
# For a complete installer that includes VC++ Runtime, use build-bundle.ps1 instead.

param(
    [string]$Configuration = "Release",
    [string]$Version = "0.0.3",
    [string]$OutputDir = ".\output",
    [switch]$SkipBuild
)

# Set error action preference
$ErrorActionPreference = "Stop"

# Function to check if a command exists
function Test-Command {
    param($Command)
    try {
        if (Get-Command $Command -ErrorAction Stop) {
            return $true
        }
    }
    catch {
        return $false
    }
}

# Check for required tools
Write-Host "Checking for required tools..." -ForegroundColor Yellow

# Check for .NET SDK
if (-not (Test-Command "dotnet")) {
    Write-Error ".NET SDK is required. Please install from: https://dotnet.microsoft.com/download"
    exit 1
}

# Check for WiX Toolset v6 (either as .NET tool or MSBuild SDK)
$hasWixCli = Test-Command "wix.exe"
$hasDotnetBuild = $null -ne (dotnet --list-sdks | Select-String "6\." -Quiet)

if (-not $hasWixCli -and -not $hasDotnetBuild) {
    Write-Error @"
WiX Toolset v6 is not installed.
Install WiX CLI: dotnet tool install --global wix
Or ensure .NET 6+ SDK is installed for MSBuild support.
"@
    exit 1
}

# Get script directory
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ProjectRoot = Join-Path $ScriptDir ".."
$ProjectRoot = Join-Path $ProjectRoot ".."
$ProjectRoot = Resolve-Path $ProjectRoot

Write-Host "Project root: $ProjectRoot" -ForegroundColor Cyan
Write-Host "Configuration: $Configuration" -ForegroundColor Cyan
Write-Host "Version: $Version" -ForegroundColor Cyan

# Build Rust binaries if not skipped
if (-not $SkipBuild) {
    Write-Host "`nBuilding Rust binaries..." -ForegroundColor Yellow
    
    Push-Location $ProjectRoot
    try {
        # Build main kopi executable
        Write-Host "Building kopi.exe..." -ForegroundColor Cyan
        cargo auditable build --release
        if ($LASTEXITCODE -ne 0) {
            throw "Failed to build kopi.exe"
        }
        
        # Build kopi-shim with optimized profile
        Write-Host "Building kopi-shim.exe..." -ForegroundColor Cyan
        cargo auditable build --profile release-shim --bin kopi-shim
        if ($LASTEXITCODE -ne 0) {
            throw "Failed to build kopi-shim.exe"
        }
    }
    finally {
        Pop-Location
    }
}
else {
    Write-Host "`nSkipping Rust build (using existing binaries)" -ForegroundColor Yellow
}

# Verify binaries exist
$KopiExe = Join-Path $ProjectRoot "target\release\kopi.exe"
$KopiShimExe = Join-Path $ProjectRoot "target\release-shim\kopi-shim.exe"

if (-not (Test-Path $KopiExe)) {
    Write-Error "kopi.exe not found at: $KopiExe"
    exit 1
}

if (-not (Test-Path $KopiShimExe)) {
    Write-Error "kopi-shim.exe not found at: $KopiShimExe"
    exit 1
}

Write-Host "`nBinaries verified:" -ForegroundColor Green
Write-Host "  - $KopiExe" -ForegroundColor Gray
Write-Host "  - $KopiShimExe" -ForegroundColor Gray

# Resolve output directory relative to script directory
if (-not [System.IO.Path]::IsPathRooted($OutputDir)) {
    $OutputDir = Join-Path $ScriptDir $OutputDir
}

# Create output directory
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}

# Normalize the path to remove .\ and other redundancies
$OutputDir = [System.IO.Path]::GetFullPath($OutputDir)

# Verify LICENSE file exists
$LicenseFile = Join-Path $ProjectRoot "LICENSE"
$LicenseRtf = Join-Path $ScriptDir "License.rtf"

if (-not (Test-Path $LicenseFile)) {
    Write-Error "LICENSE file not found at: $LicenseFile"
    Write-Error "The LICENSE file is required for building the installer."
    exit 1
}

# Create RTF version of license
Write-Host "`nCreating License.rtf from LICENSE file..." -ForegroundColor Yellow
$licenseLines = Get-Content $LicenseFile

# Build RTF content line by line to preserve formatting
$rtfContentBuilder = New-Object System.Text.StringBuilder

# RTF header with proper formatting
$rtfContentBuilder.AppendLine('{\rtf1\ansi\ansicpg1252\deff0\nouicompat\deflang1033{\fonttbl{\f0\fmodern\fprq1\fcharset0 Courier New;}}') | Out-Null
$rtfContentBuilder.AppendLine('{\colortbl ;\red0\green0\blue0;}') | Out-Null
$rtfContentBuilder.AppendLine('{\*\generator Kopi Installer}\viewkind4\uc1') | Out-Null
$rtfContentBuilder.AppendLine('\pard\sa0\sl276\slmult1\f0\fs20\lang9') | Out-Null

# Process each line preserving indentation
foreach ($line in $licenseLines) {
    # Escape special RTF characters
    $escapedLine = $line `
        -replace '\\', '\\\\' `
        -replace '\{', '\\\{' `
        -replace '\}', '\\\}'
    
    # RTF doesn't require escaping regular quotes, but we'll handle smart quotes
    # Convert smart quotes to regular quotes for consistency
    $escapedLine = $escapedLine `
        -replace '["""]', '"'
    
    # Add the line with \par for line break
    $rtfContentBuilder.Append($escapedLine) | Out-Null
    $rtfContentBuilder.AppendLine('\par') | Out-Null
}

# Close RTF document
$rtfContentBuilder.AppendLine('}') | Out-Null

# Write the RTF content
$rtfContent = $rtfContentBuilder.ToString()
Set-Content -Path $LicenseRtf -Value $rtfContent -Encoding ASCII -NoNewline

# Build MSI using WiX v6
Write-Host "`nBuilding MSI installer..." -ForegroundColor Yellow

# Define MSI output file path
$MsiFile = Join-Path $OutputDir "en-us\kopi-$Version-windows-x64.msi"


# Change to WiX directory for relative paths to work
Push-Location $ScriptDir
try {
    # Prefer dotnet build if available (more reliable)
    if (Test-Command "dotnet") {
        Write-Host "Using dotnet build with Kopi.wixproj..." -ForegroundColor Cyan
        
        # Build using MSBuild via dotnet
        $BuildArgs = @(
            "build"
            "Kopi.wixproj"
            "-c", $Configuration
            "-p:Version=$Version"
            "-v:normal"
        )
        
        Write-Host "Running: dotnet $($BuildArgs -join ' ')" -ForegroundColor DarkGray
        
        & dotnet $BuildArgs
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Failed to create MSI"
            exit 1
        }
    }
}
finally {
    Pop-Location
}

# Verify MSI was created

if (Test-Path $MsiFile) {
    $MsiInfo = Get-Item $MsiFile
    Write-Host "`nMSI installer created successfully!" -ForegroundColor Green
    Write-Host "  File: $MsiFile" -ForegroundColor Gray
    Write-Host "  Size: $([math]::Round($MsiInfo.Length / 1MB, 2)) MB" -ForegroundColor Gray
    
    # Optional: Sign the MSI if certificate is available
    if ($env:KOPI_SIGN_CERT) {
        Write-Host "`nSigning MSI installer..." -ForegroundColor Yellow
        & signtool.exe sign /f $env:KOPI_SIGN_CERT /p $env:KOPI_SIGN_PASSWORD /d "Kopi - JDK Version Management Tool" /t http://timestamp.digicert.com $MsiFile
        if ($LASTEXITCODE -eq 0) {
            Write-Host "MSI signed successfully!" -ForegroundColor Green
        }
        else {
            Write-Warning "Failed to sign MSI (continuing anyway)"
        }
    }
}
else {
    Write-Error "MSI file was not created"
    exit 1
}

Write-Host "`nBuild complete!" -ForegroundColor Green
Write-Host "To install: msiexec /i `"$MsiFile`"" -ForegroundColor Cyan
Write-Host "To install silently: msiexec /i `"$MsiFile`" /quiet" -ForegroundColor Cyan
