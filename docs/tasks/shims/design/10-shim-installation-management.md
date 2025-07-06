# Shim Installation and Management

## Overview

This document covers the complete lifecycle of shim installation, setup, and management in Kopi. Shims are created during initial setup and dynamically managed throughout the tool's usage.

## Initial Installation and Setup

### Shim Installation Strategy

When `kopi setup` is run or during first use, the system:

1. **Creates the shim directory** at `~/.kopi/shims`
2. **Installs the shim binary** that will handle all tool invocations
3. **Creates individual shims** for each JDK tool based on configuration
4. **Configures the shell PATH** to include the shim directory

### Tool Selection During Setup

The system determines which tools to create shims for by:

1. **Starting with standard JDK tools** (see the tool registry in [Distribution-Specific Tools](./09-distribution-specific-tools.md))
2. **Adding distribution-specific tools** if a default distribution is configured
3. **Including user-configured additional tools** from the configuration file
4. **Excluding any tools** that the user has explicitly opted out of

This flexible approach ensures that users get all the tools they need while avoiding unnecessary shims.

## Platform-Specific Installation

### Unix Systems (Linux, macOS)

On Unix systems, the installation process:

1. **Copies the kopi-shim binary** from the Kopi installation directory to `~/.kopi/shims/`
2. **Sets executable permissions** (755) on the shim binary
3. **Creates symbolic links** for each tool pointing to the main kopi-shim binary
   - This approach saves disk space as all tools share the same binary
   - The shim determines which tool was invoked by examining the command name

### Windows Systems

On Windows, the process differs due to platform limitations:

1. **Copies kopi-shim.exe** to the shims directory as a template
2. **Creates individual .exe files** for each tool by copying the template
   - Windows doesn't support symbolic links without admin privileges
   - Each tool needs its own .exe file (e.g., java.exe, javac.exe)
3. **Maintains the same behavior** as Unix despite the different implementation

## PATH Configuration

Users need to add the shim directory to their PATH:

### Unix Shells

```bash
# .bashrc / .zshrc
export PATH="$HOME/.kopi/shims:$PATH"

# Fish shell
set -x PATH $HOME/.kopi/shims $PATH

# For system-wide installation
echo 'export PATH="$HOME/.kopi/shims:$PATH"' >> /etc/profile.d/kopi.sh
```

### Windows

```powershell
# PowerShell profile
$env:Path = "$env:USERPROFILE\.kopi\shims;$env:Path"

# Permanent PATH update (requires admin)
[Environment]::SetEnvironmentVariable(
    "Path", 
    "$env:USERPROFILE\.kopi\shims;$([Environment]::GetEnvironmentVariable('Path', 'User'))",
    "User"
)

# Command Prompt
setx PATH "%USERPROFILE%\.kopi\shims;%PATH%"
```

### Shell Detection and Configuration

The setup process automatically detects the user's shell and configures it appropriately:

1. **Detects the current shell** (Bash, Zsh, Fish, PowerShell, or CMD)
2. **Adds the shim directory to PATH** in the appropriate configuration file
3. **Provides instructions** for applying the changes immediately

The system updates the following files based on the detected shell:

- **Bash**: Modifies `~/.bashrc` or `~/.bash_profile`
- **Zsh**: Updates `~/.zshrc`
- **Fish**: Modifies `~/.config/fish/config.fish`
- **PowerShell**: Updates the PowerShell profile
- **CMD**: Uses `setx` to permanently modify the user's PATH

## Ongoing Shim Management

### When Shims are Created/Updated

#### 1. JDK Installation (`kopi install`)

After successfully installing a JDK, Kopi should:
- Verify existing shims
- Create missing shims based on:
  - Standard tool list
  - Distribution-specific tools (if known)
  - User configuration
- Report any new shims created

**User Experience Example**:
```bash
$ kopi install graalvm@21
Downloading GraalVM 21.0.2...
Installing to ~/.kopi/jdks/graalvm-21.0.2...
✓ Installation complete

Verifying shims...
Created 3 new shims:
  - gu
  - native-image
  - polyglot

GraalVM 21.0.2 is now available
```

#### 2. Kopi Updates (`kopi self-update`)

- Update kopi-shim binary if changed
- No need to recreate symlinks on Unix
- On Windows, may need to recopy .exe files

#### 3. Manual Management

- `kopi shim add <tool>`: Add specific tool shim
- `kopi shim remove <tool>`: Remove specific tool shim
- `kopi shim verify`: Check and repair shims

### Shim Verification Command

The `kopi shim verify` command should check and repair the shim installation by:

1. **Checking shim directory**
   - Verify the shim directory exists
   - Confirm it's in the PATH environment variable

2. **Checking kopi-shim binary**
   - Verify the main kopi-shim binary exists
   - On Windows, check for the .exe extension

3. **Checking standard tool shims**
   - Verify all standard Java tools have shims
   - Track which shims are missing

4. **Reporting and fixing issues**
   - Display all found issues clearly
   - List missing shims
   - Offer to create missing shims automatically
   - Show success message when all shims are properly installed

### Listing Available Tools

The `kopi shim list --available` command should show tools available in installed JDKs without creating shims:

1. **Scan installed JDKs**
   - Look in ~/.kopi/jdks/ directory
   - For each JDK, check the bin directory

2. **Identify executable tools**
   - Find all executable files in each JDK's bin directory
   - Group tools by JDK version/distribution

3. **Display tool availability**
   - Show tools grouped by JDK
   - Indicate which tools already have shims
   - Help users understand what tools they could add shims for

Example output:
```
Available tools in installed JDKs:

temurin-17.0.2:
  java (shim exists)
  javac (shim exists)
  jar (shim exists)
  jshell
  jpackage

graalvm-21.0.2:
  java (shim exists)
  javac (shim exists)
  gu
  native-image
  polyglot
```

## Configuration Options

Users should be able to configure shim behavior in ~/.kopi/config.toml:

```toml
[shims]
# Additional tools to create shims for
additional_tools = ["gu", "native-image", "polyglot"]

# Tools to exclude from shim creation
exclude_tools = ["unpack200", "pack200"]  # Deprecated tools

# Auto-install settings
auto_install = true  # Default: true
auto_install_prompt = false  # Default: false
install_timeout = 300  # Seconds
```

## Auto-Installation Behavior

### Design Principle

The shim binary is designed to be lightweight and fast for normal operations. Auto-installation is an exceptional case that occurs only when:

1. A requested JDK version is not installed
2. Auto-installation is enabled in configuration
3. The user invokes a Java tool that requires the missing JDK

### Implementation Strategy

When auto-installation is triggered, the shim will:

1. **Spawn the main kopi binary as a subprocess** to handle the installation
2. **Pass appropriate arguments** to install the required JDK version
3. **Wait for the installation to complete**
4. **Retry locating the JDK** after successful installation
5. **Proceed with normal execution** using the newly installed JDK

This approach keeps the shim binary small (< 1MB) while delegating complex installation logic to the main kopi binary. The process overhead is acceptable in this case because:

- Auto-installation is an infrequent operation
- The time spent spawning a subprocess is negligible compared to downloading and extracting a JDK
- It allows the shim to remain lightweight for the common case (JDK already installed)

Example subprocess invocation:
```rust
// When auto-installation is needed
Command::new("kopi")
    .arg("install")
    .arg(format!("{}@{}", distribution, version))
    .arg("--auto")
    .status()?;
```

## Shim Management Commands

| Command | Description |
|---------|-------------|
| `kopi shim verify` | Check and repair shim installation |
| `kopi shim add <tool>` | Create shim for specific tool |
| `kopi shim remove <tool>` | Remove shim for specific tool |
| `kopi shim list` | List all installed shims |
| `kopi shim list --available` | Show tools available in JDKs |

## Implementation Considerations

1. **Performance**: Shim verification should be fast since it may run frequently
2. **Safety**: Never remove or overwrite shims without user confirmation
3. **Compatibility**: Handle platform differences (Unix symlinks vs Windows copies)
4. **User Experience**: Provide clear feedback about what's happening
5. **Error Recovery**: Gracefully handle partial installations or corrupted shims
6. **Post-Setup Instructions**: After configuration, users are instructed to either:
   - Restart their shell for changes to take effect
   - Source their shell configuration file to apply changes immediately

## Next: [Creating and Maintaining Curated Tool Lists](./11-tool-discovery.md)