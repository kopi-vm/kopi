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

# Kopi Installer Test Script for Windows Sandbox
# This script tests the Kopi bundle installer in various scenarios
# Supports both bundle (.exe) and MSI (.msi) installers

param(
    [string]$InstallerPath = ".\output\kopi-bundle-with-vcredist-0.0.3-windows-x64.exe",
    [switch]$SkipCleanup
)

# Set console encoding to UTF-8 to prevent character display issues
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8

$ErrorActionPreference = "Stop"

# Test results
$testResults = @()

function Write-TestHeader {
    param($TestName)
    Write-Host "`n========================================" -ForegroundColor Cyan
    Write-Host " $TestName" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
}

function Test-Result {
    param(
        [string]$TestName,
        [bool]$Success,
        [string]$Message = ""
    )
    
    $result = [PSCustomObject]@{
        Test = $TestName
        Success = $Success
        Message = $Message
    }
    
    $script:testResults += $result
    
    if ($Success) {
        Write-Host "[PASS] $TestName" -ForegroundColor Green
    } else {
        Write-Host "[FAIL] $TestName" -ForegroundColor Red
        if ($Message) {
            Write-Host "  $Message" -ForegroundColor Yellow
        }
    }
}

# Check if installer exists
Write-TestHeader "Pre-Test Validation"
if (-not (Test-Path $InstallerPath)) {
    Write-Error "Installer file not found at: $InstallerPath"
    Write-Host "Please build the bundle installer first with: .\build-bundle.ps1" -ForegroundColor Yellow
    Write-Host "Current directory: $(Get-Location)" -ForegroundColor Gray
    Write-Host "Available files:" -ForegroundColor Gray
    Get-ChildItem -Path ".\output\en-us" -Filter "*.exe" -ErrorAction SilentlyContinue | ForEach-Object { Write-Host "  $_" -ForegroundColor Gray }
    Get-ChildItem -Path ".\output\en-us" -Filter "*.msi" -ErrorAction SilentlyContinue | ForEach-Object { Write-Host "  $_" -ForegroundColor Gray }
    exit 1
}

$InstallerFullPath = (Resolve-Path $InstallerPath).Path
$InstallerSize = (Get-Item $InstallerFullPath).Length
Write-Host "Installer file found: $InstallerFullPath" -ForegroundColor Green
Write-Host "Installer file size: $([math]::Round($InstallerSize / 1MB, 2)) MB" -ForegroundColor Gray

# Determine installer type
$isBundle = $InstallerFullPath -like "*.exe"
$isMsi = $InstallerFullPath -like "*.msi"

if ($isBundle) {
    Write-Host "Installer type: Bundle (includes VC++ Runtime)" -ForegroundColor Cyan
    Write-Host "Note: VC++ installer will run automatically and skip if already installed" -ForegroundColor Gray
} elseif ($isMsi) {
    Write-Host "Installer type: MSI (requires VC++ Runtime pre-installed)" -ForegroundColor Cyan
    
    # Quick MSI validation
    try {
        $shell = New-Object -ComObject Shell.Application
        $folder = $shell.Namespace((Split-Path $InstallerFullPath))
        $file = $folder.ParseName((Split-Path $InstallerFullPath -Leaf))
        if ($file) {
            Write-Host "MSI package appears valid" -ForegroundColor Green
        } else {
            Write-Warning "Could not validate MSI package structure"
        }
        [System.Runtime.Interopservices.Marshal]::ReleaseComObject($shell) | Out-Null
    } catch {
        Write-Warning "Could not validate MSI package: $_"
    }
} else {
    Write-Error "Unknown installer type. Expected .exe (bundle) or .msi"
    exit 1
}

# Test 1: Silent Installation
Write-TestHeader "Test 1: Silent Installation"
Write-Host "Installing Kopi silently..." -ForegroundColor Yellow

$installLog = "install-silent.log"

if ($isBundle) {
    Write-Host "Running bundle installer..." -ForegroundColor Gray
    $result = Start-Process -FilePath $InstallerFullPath -ArgumentList "/quiet", "/log", $installLog -Wait -PassThru
} else {
    Write-Host "Running: msiexec /i `"$InstallerFullPath`" /quiet /l*v $installLog" -ForegroundColor Gray
    $result = Start-Process msiexec -ArgumentList "/i", "`"$InstallerFullPath`"", "/quiet", "/l*v", $installLog -Wait -PassThru
}

Test-Result "Silent Installation" ($result.ExitCode -eq 0) "Exit code: $($result.ExitCode)"

# Provide more details on failure
if ($result.ExitCode -ne 0) {
    Write-Host "Installation failed. Common error codes:" -ForegroundColor Yellow
    switch ($result.ExitCode) {
        2 { Write-Host "  2 = ERROR_FILE_NOT_FOUND - MSI file not found or corrupted" -ForegroundColor Red }
        1603 { Write-Host "  1603 = Fatal error during installation" -ForegroundColor Red }
        1619 { Write-Host "  1619 = MSI package could not be opened" -ForegroundColor Red }
        1638 { Write-Host "  1638 = Another version is already installed" -ForegroundColor Red }
        default { Write-Host "  $($result.ExitCode) = Unknown error" -ForegroundColor Red }
    }
    
    # Try to extract key errors from log
    if (Test-Path $installLog) {
        Write-Host "`nChecking installation log for details..." -ForegroundColor Yellow
        try {
            # Try Unicode encoding first (common for MSI logs)
            $logContent = Get-Content $installLog -Encoding Unicode -ErrorAction SilentlyContinue
            if (-not $logContent -or $logContent -match '\x00') {
                $logContent = Get-Content $installLog -ErrorAction SilentlyContinue
            }
            
            if ($logContent) {
                $errorLines = $logContent | Select-String -Pattern "Error|Failed|returning [^0]" | Select-Object -First 5
                if ($errorLines) {
                    Write-Host "Key log entries:" -ForegroundColor Yellow
                    $errorLines | ForEach-Object { Write-Host "  $_" -ForegroundColor Gray }
                }
            }
        } catch {
            Write-Warning "Could not read log file"
        }
    }
}

# Test 2: Verify File Installation
Write-TestHeader "Test 2: File Installation Verification"

$filesToCheck = @(
    "$env:ProgramFiles\Kopi\bin\kopi.exe",
    "$env:ProgramFiles\Kopi\bin\kopi-shim.exe",
    "$env:ProgramFiles\Kopi\docs\README.md",
    "$env:ProgramFiles\Kopi\docs\LICENSE",
    "$env:ProgramFiles\Kopi\docs\reference.md"
)

$allFilesExist = $true
foreach ($file in $filesToCheck) {
    $exists = Test-Path $file
    Test-Result "File: $(Split-Path $file -Leaf)" $exists
    if (-not $exists) { $allFilesExist = $false }
}

# Test 3: Environment Variables
Write-TestHeader "Test 3: Environment Variables"

# Check KOPI_HOME
$kopiHome = [System.Environment]::GetEnvironmentVariable("KOPI_HOME", "Machine")
$expectedKopiHome = "$env:USERPROFILE\.kopi"
$kopiHomeCorrect = $kopiHome -eq $expectedKopiHome
Test-Result "KOPI_HOME set correctly" $kopiHomeCorrect "Expected: $expectedKopiHome, Got: $kopiHome"

# Check PATH
$machinePath = [System.Environment]::GetEnvironmentVariable("PATH", "Machine")
$pathHasBin = $machinePath -like "*$env:ProgramFiles\Kopi\bin*"
$pathHasShims = ($machinePath -like "*%USERPROFILE%\.kopi\shims*") -or ($machinePath -like "*$env:USERPROFILE\.kopi\shims*")

Test-Result "PATH contains Kopi\bin" $pathHasBin
Test-Result "PATH contains Kopi\shims" $pathHasShims

# Test 4: Command Execution
Write-TestHeader "Test 4: Command Execution"

# Refresh environment for current session
$env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")
$env:KOPI_HOME = [System.Environment]::GetEnvironmentVariable("KOPI_HOME", "Machine")

# Try to run kopi
try {
    # First check if the executable exists
    $kopiPath = "$env:ProgramFiles\Kopi\bin\kopi.exe"
    if (Test-Path $kopiPath) {
        Write-Host "Found kopi.exe at: $kopiPath" -ForegroundColor Gray
        
        # Try to run kopi with --version (clap provides this automatically)
        Write-Host "Attempting to run: $kopiPath --version" -ForegroundColor Gray
        $kopiOutput = & $kopiPath --version 2>&1
        $exitCode = $LASTEXITCODE
        $kopiRuns = $exitCode -eq 0
        
        if ($kopiRuns) {
            Test-Result "kopi.exe runs" $kopiRuns "Version: $kopiOutput"
        } else {
            Test-Result "kopi.exe runs" $false "Exit code: $exitCode, Output: $kopiOutput"
            
            # Try with help if version failed
            Write-Host "Trying with help command..." -ForegroundColor Yellow
            $helpOutput = & $kopiPath help 2>&1
            Write-Host "Help output: $helpOutput" -ForegroundColor Gray
            Write-Host "Help exit code: $LASTEXITCODE" -ForegroundColor Gray
        }
        
        # If version failed, try just running the exe
        if (-not $kopiRuns) {
            Write-Host "Trying to run kopi.exe without arguments..." -ForegroundColor Yellow
            $plainOutput = & $kopiPath 2>&1
            Write-Host "Plain run output: $plainOutput" -ForegroundColor Gray
            Write-Host "Plain run exit code: $LASTEXITCODE" -ForegroundColor Gray
            
            # Check for missing dependencies (common in Windows Sandbox)
            Write-Host "`nChecking for potential dependency issues..." -ForegroundColor Yellow
            
            # Check if Visual C++ Runtime is installed
            $vcRedistKeys = @(
                "HKLM:\SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64",
                "HKLM:\SOFTWARE\WOW6432Node\Microsoft\VisualStudio\14.0\VC\Runtimes\x64",
                "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{*}",
                "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\{*}"
            )
            $vcInstalled = $false
            foreach ($key in $vcRedistKeys) {
                if ($key -like "*{*}*") {
                    Get-ChildItem $key -ErrorAction SilentlyContinue | Where-Object {
                        $_.GetValue("DisplayName") -like "*Visual C++*Redistributable*"
                    } | Select-Object -First 1 | ForEach-Object {
                        $vcInstalled = $true
                        Write-Host "  Found: $($_.GetValue('DisplayName'))" -ForegroundColor Green
                    }
                } elseif (Test-Path $key) {
                    $vcInstalled = $true
                    break
                }
            }
            if (-not $vcInstalled) {
                Write-Host "  Visual C++ Redistributable may not be installed" -ForegroundColor Yellow
                Write-Host "  This is often required for Rust applications" -ForegroundColor Yellow
            }
            
            # Try to get more details about the error using Windows Event Log
            Write-Host "`nChecking Windows Event Log for application errors..." -ForegroundColor Yellow
            try {
                $events = Get-WinEvent -LogName Application -MaxEvents 10 -ErrorAction SilentlyContinue |
                    Where-Object { $_.TimeCreated -gt (Get-Date).AddMinutes(-5) -and $_.LevelDisplayName -eq "Error" }
                if ($events) {
                    Write-Host "Recent application errors:" -ForegroundColor Yellow
                    $events | Select-Object -First 2 | ForEach-Object {
                        Write-Host "  $($_.TimeCreated): $($_.Message -split "`n" | Select-Object -First 1)" -ForegroundColor Gray
                    }
                }
            } catch {
                Write-Host "  Could not access Event Log" -ForegroundColor Gray
            }
            
            # Check file properties
            $fileInfo = Get-Item $kopiPath
            Write-Host "  File size: $($fileInfo.Length) bytes" -ForegroundColor Gray
            Write-Host "  File version: $($fileInfo.VersionInfo.FileVersion)" -ForegroundColor Gray
            Write-Host "  Product version: $($fileInfo.VersionInfo.ProductVersion)" -ForegroundColor Gray
            
            # Check for common runtime DLLs
            Write-Host "`nChecking for runtime DLLs..." -ForegroundColor Yellow
            $systemDlls = @(
                "vcruntime140.dll",
                "msvcp140.dll",
                "api-ms-win-crt-runtime-l1-1-0.dll"
            )
            foreach ($dll in $systemDlls) {
                $dllPath = "$env:SystemRoot\System32\$dll"
                if (Test-Path $dllPath) {
                    Write-Host "  [OK] Found: $dll" -ForegroundColor Green
                } else {
                    Write-Host "  [X] Missing: $dll" -ForegroundColor Red
                }
            }
            
            # Try running with explicit error capture
            Write-Host "`nTrying to capture detailed error..." -ForegroundColor Yellow
            try {
                $pinfo = New-Object System.Diagnostics.ProcessStartInfo
                $pinfo.FileName = $kopiPath
                $pinfo.Arguments = "--version"
                $pinfo.RedirectStandardOutput = $true
                $pinfo.RedirectStandardError = $true
                $pinfo.UseShellExecute = $false
                $pinfo.CreateNoWindow = $true
                
                $p = New-Object System.Diagnostics.Process
                $p.StartInfo = $pinfo
                $started = $p.Start()
                $p.WaitForExit()
                
                Write-Host "  Process started: $started" -ForegroundColor Gray
                Write-Host "  Exit code: $($p.ExitCode)" -ForegroundColor Gray
                Write-Host "  StdOut: $($p.StandardOutput.ReadToEnd())" -ForegroundColor Gray
                Write-Host "  StdErr: $($p.StandardError.ReadToEnd())" -ForegroundColor Gray
            } catch {
                Write-Host "  Process start exception: $_" -ForegroundColor Red
            }
        }
    } else {
        Test-Result "kopi.exe runs" $false "Executable not found at: $kopiPath"
    }
} catch {
    Test-Result "kopi.exe runs" $false $_.Exception.Message
    Write-Host "Exception details: $($_.Exception)" -ForegroundColor Red
}

# Test 5: Post-Installation Setup
Write-TestHeader "Test 5: Post-Installation Setup"

# Note: kopi setup is no longer run automatically during installation
# Users must run it manually after installation
Write-Host "Note: 'kopi setup' must be run manually after installation" -ForegroundColor Yellow

# Check if .kopi directory exists (it shouldn't until user runs kopi setup)
$kopiDirExists = Test-Path "$env:USERPROFILE\.kopi"
Test-Result ".kopi directory should NOT exist yet" (-not $kopiDirExists) "Directory exists: $kopiDirExists (should be False until 'kopi setup' is run)"

# Test 6: Uninstallation
if (-not $SkipCleanup) {
    Write-TestHeader "Test 6: Uninstallation"
    Write-Host "Uninstalling Kopi..." -ForegroundColor Yellow
    
    $uninstallLog = "uninstall.log"
    
    if ($isBundle) {
        Write-Host "Running bundle uninstaller..." -ForegroundColor Gray
        # Bundle uninstall - need to find the uninstaller in Add/Remove Programs
        # For now, use the bundle exe with uninstall switch
        $result = Start-Process -FilePath $InstallerFullPath -ArgumentList "/uninstall", "/quiet", "/log", $uninstallLog -Wait -PassThru
    } else {
        $result = Start-Process msiexec -ArgumentList "/x", "`"$InstallerFullPath`"", "/quiet", "/l*v", $uninstallLog -Wait -PassThru
    }
    
    Test-Result "Silent Uninstallation" ($result.ExitCode -eq 0) "Exit code: $($result.ExitCode)"
    
    # Verify removal
    Start-Sleep -Seconds 2
    $kopiExeRemoved = -not (Test-Path "$env:ProgramFiles\Kopi\bin\kopi.exe")
    Test-Result "kopi.exe removed" $kopiExeRemoved
}

# Test Summary
Write-TestHeader "Test Summary"

$passed = ($testResults | Where-Object { $_.Success }).Count
$failed = ($testResults | Where-Object { -not $_.Success }).Count
$total = $testResults.Count

Write-Host "Total Tests: $total" -ForegroundColor White
Write-Host "Passed: $passed" -ForegroundColor Green
Write-Host "Failed: $failed" -ForegroundColor $(if ($failed -eq 0) { "Green" } else { "Red" })

# Export results
$testResults | Export-Csv -Path "test-results.csv" -NoTypeInformation
Write-Host "`nTest results saved to: test-results.csv" -ForegroundColor Gray

# Check logs for errors
Write-Host "`nChecking installation log for errors..." -ForegroundColor Yellow
if (Test-Path $installLog) {
    # Try to read log with different encodings
    $logContent = $null
    try {
        # First try Unicode (UTF-16) which is common for MSI logs
        $logContent = Get-Content $installLog -Encoding Unicode -ErrorAction SilentlyContinue
        if (-not $logContent -or $logContent -match '\x00') {
            # Try default encoding
            $logContent = Get-Content $installLog -ErrorAction SilentlyContinue
        }
    } catch {
        Write-Warning "Could not read log file: $_"
    }
    
    if ($logContent) {
        $errors = $logContent | Select-String -Pattern "Error \d+|Failed|Exception|returning [^0]" | Select-Object -First 5
        if ($errors) {
            Write-Host "Found errors in log:" -ForegroundColor Red
            $errors | ForEach-Object { Write-Host $_.Line -ForegroundColor Yellow }
        } else {
            Write-Host "No obvious errors found in installation log" -ForegroundColor Green
        }
    }
}

# Exit code
exit $(if ($failed -eq 0) { 0 } else { 1 })
