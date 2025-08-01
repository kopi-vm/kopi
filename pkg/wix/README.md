# Kopi Windows Installer (MSI)

This directory contains the WiX Toolset v6 configuration files for building the Kopi MSI installer for Windows.

## Prerequisites

1. **.NET 6 SDK or later**
   - Required for WiX v6 MSBuild support
   - Download from: https://dotnet.microsoft.com/download
   - Verify installation: `dotnet --version`

2. **WiX Toolset v6** (optional but recommended)
   - Install as a .NET tool: `dotnet tool install --global wix`
   - The WiX SDK will be automatically restored via NuGet when building
   - Verify installation: `wix --version`

3. **Rust toolchain** (for building Kopi binaries)
   - Install from: https://rustup.rs/

4. **PowerShell** (for running the build script)

## Building the MSI

### Quick Build

Run the build script from this directory:

```powershell
.\build.ps1
```

### Build Options

```powershell
# Build with specific version
.\build.ps1 -Version "0.1.0"

# Skip Rust build (use existing binaries)
.\build.ps1 -SkipBuild

# Specify output directory
.\build.ps1 -OutputDir "C:\temp\kopi-installer"

# Debug configuration
.\build.ps1 -Configuration Debug
```

### Using MSBuild/dotnet directly

You can also build using standard .NET tooling:

```powershell
# Build MSI using dotnet
dotnet build Kopi.wixproj

# Or using MSBuild directly
msbuild Kopi.wixproj /p:Configuration=Release

# Using WiX CLI (if installed)
wix build Product.wxs -arch x64 -out output\kopi.msi
```

## Files

- `Product.wxs` - Main WiX configuration defining the MSI structure
- `Kopi.wixproj` - MSBuild project file for WiX v6
- `WixUI_en-us.wxl` - Localization strings for English
- `build.ps1` - PowerShell build script
- `License.rtf` - License file (generated from project LICENSE)

## MSI Features

The installer includes:

1. **Core Components**
   - `kopi.exe` - Main Kopi executable
   - `kopi-shim.exe` - Shim executable for transparent JDK switching
   - Documentation files

2. **System Integration**
   - Adds Kopi shims directory to system PATH
   - Creates desktop and Start Menu shortcuts
   - Sets up KOPI_HOME environment variable

3. **User Directories**
   - Creates `%LOCALAPPDATA%\kopi` structure:
     - `shims/` - Java executable shims
     - `jdks/` - Installed JDK versions
     - `cache/` - Metadata cache

4. **Custom Actions**
   - Post-install shim setup
   - Environment variable configuration

## Important GUIDs

**UpgradeCode**: `6503f7d2-998f-412b-8d34-b6b2073cf939`
- This GUID identifies Kopi across all versions
- **NEVER change this value after the first release**
- Used for proper upgrade/uninstall handling

**Component GUIDs** (auto-generated, do not change after release):
- KopiExe: `bf8637ec-6a0c-44fb-93a0-f31d266abf62`
- KopiShimExe: `5c2fa8d3-32b1-45f2-bd5a-fc9f2532a29b`
- Documentation: `8ca86153-c757-4616-b251-bf426aa6383d`
- EnvironmentVars: `58291e66-7453-4bee-9a33-51530fe39c44`

## WiX v6 Changes

This project uses WiX v6, which has several important changes from v3:

- **MSBuild SDK integration**: Uses SDK-style project files (.wixproj)
- **NuGet-based distribution**: WiX SDK is restored via NuGet packages
- **Standard .NET tooling**: Build with `dotnet build` or `msbuild`
- **New schema**: Uses `http://wixtoolset.org/schemas/v4/wxs` (v4 schema in v6 tools)
- **Package element**: Replaces the Product element in .wxs files
- **StandardDirectory**: Uses built-in directory references
- **Simplified project files**: Smart defaults reduce configuration needs

## Customization

### Changing Default Settings

Edit the property values in `Product.wxs`:

```xml
<Property Id="WIXUI_INSTALLDIR" Value="INSTALLFOLDER" />
```

### Adding New Components

1. Add component definition in `Product.wxs`
2. Reference in the appropriate Feature element
3. Rebuild the MSI

### Signing the MSI

To sign the installer, set environment variables before building:

```powershell
$env:KOPI_SIGN_CERT = "path\to\certificate.pfx"
$env:KOPI_SIGN_PASSWORD = "certificate-password"
.\build.ps1
```

## Troubleshooting

### WiX Not Found
If you get "WiX Toolset v6 is not installed" error:
1. Install WiX v6: `dotnet tool install --global wix`
2. Ensure .NET tools are in PATH
3. Restart PowerShell

### Build Failures
1. Check that Rust binaries exist in `target/release/`
2. Verify all source files are present
3. Check WiX output for specific errors
4. Run with verbose output: `wix build -v`

### Installation Issues
- Run installer with logging: `msiexec /i kopi-0.0.3-x64.msi /l*v install.log`
- Check Windows Event Viewer for MSI errors
- Ensure you have administrator privileges

## Distribution

The built MSI file will be in the output directory:
- Default: `.\output\kopi-{version}-x64.msi`
- File size: ~5-10 MB (depending on build configuration)

The MSI can be distributed through:
- Direct download from project website
- Microsoft Store (with additional packaging)
- Corporate software deployment tools
- Package managers (Chocolatey, WinGet)