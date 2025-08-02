# Download Visual C++ Redistributable for bundling with installer
param(
    [string]$OutputPath = ".\redist"
)

$ErrorActionPreference = "Stop"

Write-Host "Downloading Visual C++ Redistributable..." -ForegroundColor Cyan

# Create output directory
if (-not (Test-Path $OutputPath)) {
    New-Item -ItemType Directory -Path $OutputPath | Out-Null
}

# Visual C++ 2015-2022 Redistributable (x64)
$vcRedistUrl = "https://aka.ms/vs/17/release/vc_redist.x64.exe"
$vcRedistPath = Join-Path $OutputPath "vc_redist.x64.exe"

if (Test-Path $vcRedistPath) {
    Write-Host "VC++ Redistributable already exists at: $vcRedistPath" -ForegroundColor Yellow
    $response = Read-Host "Download again? (y/N)"
    if ($response -ne 'y') {
        Write-Host "Using existing file." -ForegroundColor Green
        exit 0
    }
}

Write-Host "Downloading from: $vcRedistUrl" -ForegroundColor Gray

try {
    $ProgressPreference = 'SilentlyContinue'
    Invoke-WebRequest -Uri $vcRedistUrl -OutFile $vcRedistPath -UseBasicParsing
    $ProgressPreference = 'Continue'
    
    $fileInfo = Get-Item $vcRedistPath
    Write-Host "✓ Downloaded successfully!" -ForegroundColor Green
    Write-Host "  File: $vcRedistPath" -ForegroundColor Gray
    Write-Host "  Size: $([math]::Round($fileInfo.Length / 1MB, 2)) MB" -ForegroundColor Gray
    
    # Get file version
    $version = $fileInfo.VersionInfo.FileVersion
    Write-Host "  Version: $version" -ForegroundColor Gray
    
    Write-Host "`nNext steps:" -ForegroundColor Cyan
    Write-Host "1. The VC++ Redistributable is now available for bundling" -ForegroundColor White
    Write-Host "2. Build the bundle with: .\build-bundle.ps1" -ForegroundColor White
    
    exit 0
    
} catch {
    Write-Error "Failed to download VC++ Redistributable: $_"
    exit 1
}
