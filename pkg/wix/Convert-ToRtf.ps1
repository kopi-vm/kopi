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

<#
.SYNOPSIS
    Converts a plain text file to RTF format.

.DESCRIPTION
    This script converts a plain text file (such as a LICENSE file) to RTF format
    suitable for use in WiX installers. It preserves line breaks and handles
    special RTF characters appropriately.

.PARAMETER InputFile
    The path to the input text file to convert.

.PARAMETER OutputFile
    The path where the RTF file should be saved.

.EXAMPLE
    .\Convert-ToRtf.ps1 -InputFile "..\..\LICENSE" -OutputFile "License.rtf"

.NOTES
    This script is designed specifically for converting license files for WiX installers.
#>

param(
    [Parameter(Mandatory=$true)]
    [ValidateScript({Test-Path $_ -PathType Leaf})]
    [string]$InputFile,
    
    [Parameter(Mandatory=$true)]
    [string]$OutputFile
)

# Set error action preference
$ErrorActionPreference = "Stop"

try {
    Write-Host "Converting text file to RTF format..." -ForegroundColor Yellow
    Write-Host "  Input: $InputFile" -ForegroundColor Gray
    Write-Host "  Output: $OutputFile" -ForegroundColor Gray
    
    # Read the input file
    $textLines = Get-Content $InputFile -ErrorAction Stop
    
    # Build RTF content line by line to preserve formatting
    $rtfContentBuilder = New-Object System.Text.StringBuilder
    
    # RTF header with proper formatting
    $rtfContentBuilder.AppendLine('{\rtf1\ansi\ansicpg1252\deff0\nouicompat\deflang1033{\fonttbl{\f0\fmodern\fprq1\fcharset0 Courier New;}}') | Out-Null
    $rtfContentBuilder.AppendLine('{\colortbl ;\red0\green0\blue0;}') | Out-Null
    $rtfContentBuilder.AppendLine('{\*\generator Kopi Installer}\viewkind4\uc1') | Out-Null
    $rtfContentBuilder.AppendLine('\pard\sa0\sl276\slmult1\f0\fs20\lang9') | Out-Null
    
    # Process each line preserving indentation
    foreach ($line in $textLines) {
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
    
    # Ensure output directory exists
    $outputDir = Split-Path -Parent $OutputFile
    if ($outputDir -and -not (Test-Path $outputDir)) {
        New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
    }
    
    # Write the RTF content
    $rtfContent = $rtfContentBuilder.ToString()
    Set-Content -Path $OutputFile -Value $rtfContent -Encoding ASCII -NoNewline
    
    Write-Host "RTF file created successfully!" -ForegroundColor Green
    
    # Verify the file was created
    if (Test-Path $OutputFile) {
        $fileInfo = Get-Item $OutputFile
        Write-Host "  Size: $($fileInfo.Length) bytes" -ForegroundColor Gray
    }
}
catch {
    Write-Error "Failed to convert file to RTF: $_"
    exit 1
}