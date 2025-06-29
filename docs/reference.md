# Kopi Reference Manual

## Overview

Kopi is a JDK version management tool that integrates with your shell to seamlessly switch between different Java Development Kit versions. It fetches JDK metadata from foojay.io and provides a simple, fast interface similar to tools like volta, nvm, and pyenv.

## Installation & Setup Commands

### `kopi install`

Install a specific JDK version.

**Usage:**
```bash
kopi install <version>                    # Install a specific JDK version
kopi install <distribution>@<version>     # Install specific distribution
```

**Examples:**
```bash
kopi install 21                          # Latest Java 21 (Eclipse Temurin by default)
kopi install 21.0.1                      # Specific version (Eclipse Temurin by default)
kopi install temurin@17.0.2              # Specific distribution and version
kopi install corretto@21                 # Latest Java 21 from Amazon Corretto
kopi install zulu@11.0.15                # Zulu JDK version 11.0.15
```

**Options:**
- `--force`: Reinstall even if already installed
- `--dry-run`: Show what would be installed without actually installing
- `--no-progress`: Disable progress indicators
- `--timeout <seconds>`: Download timeout in seconds (default: 120)
- `--javafx-bundled`: Include packages regardless of JavaFX bundled status

### `kopi uninstall`

Remove an installed JDK version.

**Usage:**
```bash
kopi uninstall <version>                 # Remove an installed JDK version
kopi uninstall <distribution>@<version>  # Remove specific distribution
```

## Version Management Commands

### `kopi use`

Switch to a JDK version in current shell.

**Usage:**
```bash
kopi use <version>                       # Switch to a JDK version in current shell
```

### `kopi shell`

Launch new shell with JDK environment configured.

**Usage:**
```bash
kopi shell                               # Launch new shell with JDK environment configured
```

### `kopi global`

Set default JDK version globally.

**Usage:**
```bash
kopi global <version>                    # Set default JDK version globally
```

### `kopi local`

Set JDK version for current project.

**Usage:**
```bash
kopi local <version>                     # Set JDK version for current project
```

### `kopi pin`

Pin JDK version in project config.

**Usage:**
```bash
kopi pin <version>                       # Pin JDK version in project config
```

## Information Commands

### `kopi list`

List installed JDK versions.

**Usage:**
```bash
kopi list                                # List installed JDK versions
kopi list --remote                       # List available versions from foojay.io
```

### `kopi current`

Show current JDK version and details.

**Usage:**
```bash
kopi current                             # Show current JDK version and details
```

### `kopi which`

Show path to current java executable.

**Usage:**
```bash
kopi which                               # Show path to current java executable
```

## Project Configuration Commands

### `kopi init`

Initialize kopi in current project.

**Usage:**
```bash
kopi init                                # Initialize kopi in current project
```

### `kopi env`

Show JDK environment variables.

**Usage:**
```bash
kopi env                                 # Show JDK environment variables
```

## Advanced Features

### `kopi default`

Set default distribution for installations.

**Usage:**
```bash
kopi default <distribution>              # Set default distribution for installations
```

**Examples:**
```bash
kopi default temurin                     # Set Eclipse Temurin as default
kopi default corretto                    # Set Amazon Corretto as default
```

### `kopi refresh`

Update metadata cache from foojay.io. This is an alias for `kopi cache refresh`.

**Usage:**
```bash
kopi refresh                             # Update metadata cache from foojay.io
kopi refresh --javafx-bundled            # Include JavaFX bundled packages
```

### `kopi search`

Search for available JDK versions. This is an alias for `kopi cache search`.

**Usage:**
```bash
kopi search <query>                      # Search for JDK versions
kopi search <query> --compact            # Minimal display (default)
kopi search <query> --detailed           # Full information display
kopi search <query> --json               # JSON output for programmatic use
kopi search <query> --lts-only           # Filter to show only LTS versions
```

**Examples:**
```bash
kopi search 21                           # Find all Java 21 versions
kopi search corretto                     # List all Corretto versions
kopi search latest                       # Show latest version of each distribution
kopi search 21 --detailed                # Show full details
kopi search 21 --lts-only                # Only show LTS versions
```

### `kopi prune`

Remove unused JDK versions.

**Usage:**
```bash
kopi prune                               # Remove unused JDK versions
```

### `kopi doctor`

Diagnose kopi installation issues.

**Usage:**
```bash
kopi doctor                              # Diagnose kopi installation issues
```

## Cache Management Commands

### `kopi cache`

Manage the JDK metadata cache used for searching and installing JDK versions.

#### `kopi cache refresh`

Update the metadata cache from foojay.io API.

**Usage:**
```bash
kopi cache refresh                       # Refresh metadata for all distributions
kopi cache refresh --javafx-bundled      # Include JavaFX bundled packages
```

#### `kopi cache search`

Search for available JDK versions in the cache with enhanced display options.

**Usage:**
```bash
kopi cache search <query>                # Search for JDK versions
kopi cache search <query> --compact      # Minimal display (default)
kopi cache search <query> --detailed     # Full information display
kopi cache search <query> --json         # JSON output for programmatic use
kopi cache search <query> --lts-only     # Filter to show only LTS versions
```

**Examples:**
```bash
# Search by version
kopi cache search 21                     # Find all Java 21 versions
kopi cache search 17.0.9                 # Find specific version

# Search by distribution
kopi cache search corretto               # List all Corretto versions
kopi cache search temurin@21             # Find Temurin Java 21 versions

# Special queries
kopi cache search latest                 # Show latest version of each distribution
kopi cache search jre@17                 # Search for JRE packages only

# Display options
kopi cache search 21 --detailed          # Show full details (OS/Arch, Status, Size)
kopi cache search 21 --json              # Output as JSON
kopi cache search 21 --lts-only          # Only show LTS versions
```

**Display Modes:**
- **Compact (default)**: Shows Distribution, Version, and LTS status
- **Detailed**: Includes Status (GA/EA), Type (JDK/JRE), OS/Arch, LibC, Size, and JavaFX
- **JSON**: Machine-readable format with all available fields

**Color Coding:**
- LTS versions are highlighted in green
- STS versions are shown in yellow
- GA releases are marked in green
- EA releases are dimmed yellow

#### `kopi cache list-distributions`

List all available distributions in the cache.

**Usage:**
```bash
kopi cache list-distributions            # Show all cached distributions
```

**Output includes:**
- Distribution ID (e.g., "temurin", "corretto")
- Display name (e.g., "Eclipse Temurin", "Amazon Corretto")
- Number of versions available for current platform

#### `kopi cache info`

Show information about the cache.

**Usage:**
```bash
kopi cache info                          # Display cache details
```

**Shows:**
- Cache file location
- File size
- Last update time
- Number of distributions
- Total JDK packages

#### `kopi cache clear`

Remove all cached metadata.

**Usage:**
```bash
kopi cache clear                         # Delete the cache file
```

## Supported Distributions

- `temurin` - Eclipse Temurin (formerly AdoptOpenJDK)
- `corretto` - Amazon Corretto
- `zulu` - Azul Zulu
- `oracle` - Oracle JDK
- `graalvm` - GraalVM
- `liberica` - BellSoft Liberica
- `sapmachine` - SAP Machine
- `semeru` - IBM Semeru
- `dragonwell` - Alibaba Dragonwell

## Configuration Files

### Global Config: `~/.kopi/config.toml`

Stores default distribution preference and global settings.

Example configuration:

```toml
# Default JDK distribution for installations
default_distribution = "temurin"

[storage]
# Minimum required disk space in MB for JDK installation (default: 500)
min_disk_space_mb = 1024
```

### Project Config: `.kopi-version` or `.java-version`

Simple text file with single line specifying version:
- Can specify version as `21` or `temurin@21`
- Kopi supports `.java-version` for compatibility with other tools

### Project Metadata: `kopi.toml` (optional)

Advanced project settings:

```toml
[java]
version = "21.0.1"                    # JDK version
distribution = "temurin"              # JDK distribution

[java.env]
# Additional environment variables
JAVA_TOOL_OPTIONS = "-Xmx2g"
MAVEN_OPTS = "-Xmx1g"

[project]
# Project-specific JVM options
jvm_args = ["-XX:+UseG1GC", "-XX:MaxGCPauseMillis=200"]

[tools]
# Pin specific tool versions that come with JDK
javac = { min_version = "21.0.0" }

[fallback]
# Fallback options if primary version unavailable
allow_higher_patch = true             # Allow 21.0.2 if 21.0.1 not found
allow_lts_fallback = true             # Fall back to nearest LTS version
distributions = ["temurin", "corretto", "zulu"]  # Try distributions in order
```

## Shell Integration

Kopi uses shims for transparent version management:
- Add `~/.kopi/bin` to PATH
- Creates shims for `java`, `javac`, `jar`, etc.
- Automatic version switching based on project configuration

The `kopi shell` command provides an alternative approach:
- Launches a new shell subprocess with JDK environment variables properly configured
- Sets `JAVA_HOME`, updates `PATH` to include JDK bin directory
- Useful for isolated environments or when shim approach isn't suitable
- Respects project-specific JDK versions if launched within a project directory

## Version Specification Format

Kopi supports flexible version specifications:

- `21` - Latest Java 21 (uses default distribution)
- `21.0.1` - Specific version (uses default distribution)
- `temurin@17.0.2` - Specific distribution and version
- `corretto@21` - Latest Java 21 from Amazon Corretto
- `latest` - Latest available version
- `latest --lts` - Latest LTS version

## Debugging and Logging

Kopi provides flexible logging controls for troubleshooting and debugging:

### Verbosity Levels

Use the `-v/--verbose` flag (can be specified multiple times) with any command:

```bash
kopi install 21              # Default: warnings and errors only
kopi install 21 -v           # Info level: show major operations
kopi install 21 -vv          # Debug level: detailed flow information
kopi install 21 -vvv         # Trace level: very detailed debugging
```

The verbose flag is global and works with all commands:

```bash
kopi list -v                 # Show info logs for list command
kopi use 21 -vv              # Debug version switching
kopi current -vvv            # Trace current version detection
```

### Environment Variable Control

For persistent logging or module-specific debugging, use the `RUST_LOG` environment variable:

```bash
# Set logging level for entire session
export RUST_LOG=debug
kopi install 21

# Debug specific modules
RUST_LOG=kopi::download=debug kopi install 21        # Debug downloads only
RUST_LOG=kopi::api=trace kopi list --remote          # Trace API calls
RUST_LOG=kopi::storage=debug kopi prune              # Debug storage operations

# Multiple module filters
RUST_LOG=kopi::download=debug,kopi::security=trace kopi install 21
```

### Common Debugging Scenarios

**Installation Issues:**
```bash
kopi install 21 -vv          # See download URLs, checksums, extraction paths
```

**Version Resolution Problems:**
```bash
RUST_LOG=kopi::version=debug kopi install temurin@21  # Debug version parsing
```

**API Communication:**
```bash
RUST_LOG=kopi::api=debug kopi list --remote           # Debug foojay.io API calls
```

**Storage and Disk Space:**
```bash
RUST_LOG=kopi::storage=debug kopi install 21          # Debug installation paths
```

## Environment Variables

Kopi respects the following environment variables:

- `KOPI_HOME` - Override default kopi home directory (default: `~/.kopi`)
- `JAVA_HOME` - Set by kopi when switching JDK versions
- `PATH` - Modified by kopi to include JDK bin directory
- `RUST_LOG` - Control logging verbosity (see Debugging and Logging section)

Note: Minimum disk space requirement is configured via `~/.kopi/config.toml` (see Global Config section above)

## Troubleshooting

### Enhanced Error Messages

Kopi provides comprehensive error messages with helpful suggestions when something goes wrong:

```bash
# Example: Version not found
$ kopi install 999
Error: JDK version 'temurin 999' is not available

Details: Version lookup failed: temurin 999 not found

Suggestion: Run 'kopi cache search' to see available versions or 'kopi cache refresh' to update the list.
```

### Common Issues and Solutions

**1. Version Not Available**
```bash
Error: JDK version 'X' is not available
```
**Solution:** 
- Run `kopi cache refresh` to update the metadata
- Use `kopi cache search <version>` to find available versions
- Check if you're using the correct distribution name

**2. Already Installed**
```bash
Error: temurin 21 is already installed
```
**Solution:** Use `--force` flag to reinstall:
```bash
kopi install 21 --force
```

**3. Network Issues**
```bash
Error: Failed to download JDK
```
**Solution:**
- Check your internet connection
- Check proxy settings if behind a firewall
- Use `--timeout` to increase timeout for slow connections
- Try again later if rate limited

**4. Permission Denied**
```bash
Error: Permission denied: /path/to/directory
```
**Solution:**
- On Unix/macOS: Use `sudo` or check file permissions
- On Windows: Run as Administrator
- Ensure you have write access to `~/.kopi` directory

**5. Disk Space**
```bash
Error: Insufficient disk space
```
**Solution:**
- Free up disk space (JDK installations require 300-500MB)
- Configure minimum space in `~/.kopi/config.toml`
- Use `kopi prune` to remove unused JDK versions

**6. Checksum Mismatch**
```bash
Error: Checksum verification failed
```
**Solution:**
- Try downloading again (file may be corrupted)
- If problem persists, report issue as it may be a source problem

**7. Cache Not Found**
```bash
Error: Cache not found
```
**Solution:** Run `kopi cache refresh` to fetch the latest JDK metadata

### Exit Codes

Kopi uses specific exit codes to help with scripting and automation:

- `0`: Success
- `1`: General error
- `2`: Invalid input or configuration error
- `13`: Permission denied
- `17`: Resource already exists
- `20`: Network error
- `28`: Disk space error

### Getting Help

If you encounter issues not covered here:

1. Run the command with verbose logging:
   ```bash
   kopi install 21 -vv
   ```

2. Check the GitHub issues: https://github.com/anthropics/claude-code/issues

3. For feedback or bug reports, please report the issue at:
   https://github.com/anthropics/claude-code/issues