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

# Open Windows Sandbox with pkg\wix Directory
# This script creates a Windows Sandbox environment with the pkg\wix directory mounted
# You can manually run test-installer.ps1 or other scripts in the sandbox

param()

# Check if running as administrator
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "This script requires administrator privileges to run Windows Sandbox." -ForegroundColor Yellow
    Write-Host "Requesting elevation..." -ForegroundColor Yellow
    
    # Build arguments for the elevated process
    $arguments = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $MyInvocation.MyCommand.Definition)
    
    # Start elevated process
    try {
        Start-Process powershell.exe -Verb RunAs -ArgumentList $arguments
    } catch {
        Write-Error "Failed to elevate: $_"
        exit 1
    }
    
    # Exit current non-elevated process
    exit 0
}

Write-Host "Running with administrator privileges" -ForegroundColor Green

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host " Kopi Windows Sandbox Launcher" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# Check if Windows Sandbox is available
Write-Host "`nChecking Windows Sandbox availability..." -ForegroundColor Yellow

# Try to check if Windows Sandbox executable exists (doesn't require admin rights)
$sandboxExe = Join-Path $env:SystemRoot "System32\WindowsSandbox.exe"
if (-not (Test-Path $sandboxExe)) {
    Write-Error "Windows Sandbox appears to not be installed."
    Write-Host "To enable: Settings > Apps > Optional features > More Windows features > Windows Sandbox" -ForegroundColor Yellow
    Write-Host "Note: Windows Sandbox requires Windows 10 Pro/Enterprise or Windows 11 Pro/Enterprise" -ForegroundColor Yellow
    exit 1
}

# Try to verify sandbox is enabled by checking if we can get the process (non-admin method)
try {
    $sandboxProcess = Get-Process "WindowsSandbox" -ErrorAction SilentlyContinue
    if ($sandboxProcess) {
        Write-Host "Windows Sandbox is currently running" -ForegroundColor Green
    }
} catch {
    # Process not running is fine, we just checked if we could query it
}

Write-Host "Windows Sandbox executable found" -ForegroundColor Green

# Get script directory
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ProjectRoot = Join-Path $ScriptDir ".." | Join-Path -ChildPath ".." | Resolve-Path

Write-Host "`nWill mount pkg\wix directory in sandbox: $ScriptDir" -ForegroundColor Gray

# Create temp directory for sandbox configuration
$TempDir = Join-Path $env:TEMP "kopi-sandbox-$(Get-Date -Format 'yyyyMMdd-HHmmss')"
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null


# Create sandbox configuration
$SandboxConfig = @"
<Configuration>
    <MappedFolders>
        <MappedFolder>
            <HostFolder>$ScriptDir</HostFolder>
            <SandboxFolder>C:\HostShare</SandboxFolder>
            <ReadOnly>false</ReadOnly>
        </MappedFolder>
    </MappedFolders>
    <LogonCommand>
        <Command>cmd.exe /c start powershell.exe -NoExit -ExecutionPolicy Bypass -Command "Set-Location C:\HostShare; .\test-installer.ps1"</Command>
    </LogonCommand>
    <Networking>Default</Networking>
    <MemoryInMB>16384</MemoryInMB>
    <VGpu>Disable</VGpu>
</Configuration>
"@

$SandboxConfigPath = Join-Path $TempDir "kopi-test.wsb"
$SandboxConfig | Out-File -FilePath $SandboxConfigPath -Encoding UTF8

Write-Host "`nSandbox configuration created: $SandboxConfigPath" -ForegroundColor Gray

# Launch Windows Sandbox
Write-Host "`nLaunching Windows Sandbox..." -ForegroundColor Cyan
Write-Host "A PowerShell prompt will open in the sandbox with pkg\wix mounted." -ForegroundColor Yellow

# Start sandbox
Start-Process "WindowsSandbox.exe" -ArgumentList $SandboxConfigPath -Wait
