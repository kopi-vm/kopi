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

Remove an installed JDK version. (Not yet implemented)

**Usage:**
```bash
kopi uninstall <version>                 # Remove an installed JDK version
kopi uninstall <distribution>@<version>  # Remove specific distribution
```

## Version Management Commands

### `kopi use`

Switch to a JDK version in current shell. (Not yet implemented)

**Usage:**
```bash
kopi use <version>                       # Switch to a JDK version in current shell
```

### `kopi global`

Set default JDK version globally. (Not yet implemented)

**Usage:**
```bash
kopi global <version>                    # Set default JDK version globally
```

### `kopi local`

Set JDK version for current project. (Not yet implemented)

**Usage:**
```bash
kopi local <version>                     # Set JDK version for current project
```

### `kopi pin`

Alias for 'kopi local'. (Not yet implemented)

**Usage:**
```bash
kopi pin <version>                       # Alias for 'kopi local' (pins JDK version in project config)
```

## Information Commands

### `kopi list`

List installed JDK versions. (Not yet implemented)

**Usage:**
```bash
kopi list                                # List installed JDK versions
kopi list --all                          # Show all versions including remote ones (not yet implemented)
```

### `kopi current`

Show current JDK version and details. (Not yet implemented)

**Usage:**
```bash
kopi current                             # Show current JDK version and details
```

### `kopi which`

Show path to current java executable. (Not yet implemented)

**Usage:**
```bash
kopi which                               # Show path to current java executable
kopi which <version>                     # Show path for specific JDK version
```

## Setup and Maintenance Commands

### `kopi setup`

Initial setup and configuration for kopi. Creates necessary directories and installs shims.

**Usage:**
```bash
kopi setup                               # Initial setup and configuration
kopi setup --force                       # Force recreation of shims even if they exist
```

### `kopi shim`

Manage tool shims for JDK executables. Shims are lightweight proxy executables that intercept Java tool invocations and transparently route them to the correct JDK version based on your project configuration.

**Subcommands:**

#### `kopi shim add`

Create shims for specific JDK tools.

**Usage:**
```bash
kopi shim add <tool>                     # Create shim for a specific tool
kopi shim add <tool1> <tool2> ...        # Create shims for multiple tools
kopi shim add --all                      # Create shims for all known JDK tools
kopi shim add --force <tool>             # Force recreate existing shim
```

**Examples:**
```bash
kopi shim add java javac                 # Create shims for java and javac
kopi shim add native-image gu            # Create GraalVM-specific shims
kopi shim add --all                      # Create all standard JDK tool shims
```

#### `kopi shim remove`

Remove installed shims.

**Usage:**
```bash
kopi shim remove <tool>                  # Remove shim for a specific tool
kopi shim remove <tool1> <tool2> ...     # Remove shims for multiple tools
kopi shim remove --all                   # Remove all shims
```

**Examples:**
```bash
kopi shim remove jshell                  # Remove jshell shim
kopi shim remove --all                   # Clean up all shims
```

#### `kopi shim list`

List all installed shims and their status.

**Usage:**
```bash
kopi shim list                           # List all installed shims
kopi shim list --format <format>         # Specify output format (table/plain/json)
```

**Examples:**
```bash
kopi shim list                           # Show shims in table format
kopi shim list --format json             # Output as JSON for scripting
```

#### `kopi shim verify`

Verify the integrity of installed shims.

**Usage:**
```bash
kopi shim verify                         # Verify all shims
kopi shim verify <tool>                  # Verify specific shim
kopi shim verify --fix                   # Fix any issues found
```

**Examples:**
```bash
kopi shim verify                         # Check all shims for issues
kopi shim verify java --fix              # Verify and fix java shim if needed
```

**Notes:**
- Shims are created in `~/.kopi/shims/` directory
- The shims directory should be added to your PATH
- Shims automatically detect the required JDK version from `.kopi-version` or `.java-version` files
- Performance overhead is minimal (typically < 10ms)

## Advanced Features

### `kopi default`

Set default distribution for installations. (Not yet implemented)

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

Remove unused JDK versions. (Not yet implemented)

**Usage:**
```bash
kopi prune                               # Remove unused JDK versions
```

### `kopi doctor`

Diagnose kopi installation issues. (Not yet implemented)

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

### Default Distributions

- `temurin` - Eclipse Temurin (formerly AdoptOpenJDK)
- `corretto` - Amazon Corretto
- `zulu` - Azul Zulu
- `oracle` - Oracle JDK
- `graalvm` - GraalVM
- `liberica` - BellSoft Liberica
- `sapmachine` - SAP Machine
- `semeru` - IBM Semeru
- `dragonwell` - Alibaba Dragonwell

### Custom Distributions

Additional distributions can be configured in `~/.kopi/config.toml` using the `additional_distributions` field. See the Global Config section for details.

## Configuration Files

### Global Config: `~/.kopi/config.toml`

Stores default distribution preference and global settings.

Example configuration:

```toml
# Default JDK distribution for installations
default_distribution = "temurin"

# Additional custom distributions (optional)
# These are added to the list of recognized distributions
additional_distributions = ["company-jdk", "custom-build"]

[storage]
# Minimum required disk space in MB for JDK installation (default: 500)
min_disk_space_mb = 1024
```

#### Custom Distributions

The `additional_distributions` field allows you to use custom or private JDK distributions that are not in Kopi's default list. This is useful for:
- Corporate internal JDK builds
- Private distributions
- Experimental or custom builds

When configured, these distributions can be used with all Kopi commands:
```bash
kopi install company-jdk@21
kopi install CUSTOM-BUILD@17  # Case-insensitive
kopi cache search company-jdk
```

Note: Custom distributions are normalized to lowercase for consistency.

### Project Version Files

Kopi supports two formats for project-specific Java version configuration:

#### `.java-version` (Compatibility Mode)

Simple text file containing only a version number for compatibility with existing tools:
```
21
```
or
```
11.0.2
```
or
```
21-ea
```

- **No distribution specification** - uses the default distribution from global config
- Maintains compatibility with GitHub Actions setup-java and other tools
- Supports only exact version numbers (no ranges or wildcards)

#### `.kopi-version` (Native Format)

Kopi's native format using `@` separator for distribution and version:
```
temurin@21
```
or
```
corretto@11.0.2+9
```
or
```
zulu@21-ea+35
```

- Clear separation between distribution and version using `@`
- When only version is specified (e.g., `21`), uses default distribution
- **No version ranges**: Does not support Maven-style (`[1.7,1.8)`) or npm-style (`^1.2.3`, `~1.2.3`) specifications
- **Exact versions only**: Must specify precise version numbers

### Version Resolution

When a major version only is specified (e.g., `21`), kopi will:
- Automatically select the latest available minor and patch version
- For example, `21` might resolve to `21.0.2+13` if that's the latest available
- This provides convenience while maintaining reproducibility once installed

### Configuration Hierarchy

Version resolution order (highest to lowest priority):
1. Environment variable: `KOPI_JAVA_VERSION`
2. `.kopi-version` file (walks up directory tree)
3. `.java-version` file (walks up directory tree, for compatibility)
4. Global configuration (`~/.kopi/config.toml`)

## Shell Integration

Kopi uses shims for transparent version management:
- Add `~/.kopi/bin` to PATH
- Creates shims for `java`, `javac`, `jar`, etc.
- Automatic version switching based on project configuration (when implemented)

Note: The `kopi shell` command is planned but not yet implemented. When available, it will:
- Launch a new shell subprocess with JDK environment variables properly configured
- Set `JAVA_HOME`, update `PATH` to include JDK bin directory
- Provide isolated environments when shim approach isn't suitable
- Respect project-specific JDK versions when launched within a project directory

## Version Specification Format

Kopi supports exact version specifications with flexible formats to accommodate different JDK distributions:

### Standard Version Formats

- `21` - Latest Java 21 (uses default distribution)
- `21.0.1` - Specific version (uses default distribution)
- `temurin@17.0.2` - Specific distribution and version
- `corretto@21` - Latest Java 21 from Amazon Corretto
- `latest` - Latest available version
- `latest --lts` - Latest LTS version

### Extended Version Formats

Many JDK distributions use extended version formats with more than 3 components:

- **Amazon Corretto**: 4-5 components (e.g., `corretto@21.0.7.6.1`)
- **Alibaba Dragonwell**: 6 components (e.g., `dragonwell@21.0.7.0.7.6`)
- **Standard with build**: `temurin@21.0.7+6`
- **Pre-release versions**: `graalvm-ce@21.0.1-rc.1`

### Version Search Behavior

Kopi can search by both `java_version` and `distribution_version`:

```bash
# Searches by java_version (standard format)
kopi install temurin@21.0.7+6

# Searches by distribution_version (4+ components auto-detected)
kopi install corretto@21.0.7.6.1

# For ambiguous cases, specify explicitly
kopi install corretto@21.0.7 --java-version
kopi install corretto@21.0.7 --distribution-version
```

### Version Pattern Matching

When using commands like `uninstall` or `use`, partial version patterns match installed versions:

- Pattern `21` matches any version starting with `21` (e.g., `21.0.7.6.1`)
- Pattern `21.0` matches any version starting with `21.0`
- Pattern `21.0.7` matches any version starting with `21.0.7`
- Pattern `21.0.7.6` matches any version starting with `21.0.7.6`

**Note**: Kopi does not support version ranges or wildcards:
- No Maven-style ranges: `[1.7,1.8)`, `(,1.8]`, `[1.5,)`
- No npm-style ranges: `^1.2.3`, `~1.2.3`, `>=1.2.3 <2.0.0`
- No wildcards: `21.*`, `11.0.*`

This design keeps version management simple and reproducible.

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

### Kopi-specific Variables
- `KOPI_HOME` - Override default kopi home directory (default: `~/.kopi`)
- `JAVA_HOME` - Set by kopi when switching JDK versions
- `PATH` - Modified by kopi to include JDK bin directory
- `RUST_LOG` - Control logging verbosity (see Debugging and Logging section)

### HTTP Proxy Configuration
Kopi supports standard HTTP proxy environment variables for downloading JDKs and fetching metadata:

- `HTTP_PROXY` or `http_proxy` - Proxy server for HTTP requests
- `HTTPS_PROXY` or `https_proxy` - Proxy server for HTTPS requests
- `NO_PROXY` or `no_proxy` - Comma-separated list of hosts to bypass proxy

**Examples:**
```bash
# Set proxy for all requests
export HTTP_PROXY=http://proxy.company.com:8080
export HTTPS_PROXY=http://proxy.company.com:8080

# Set proxy with authentication
export HTTP_PROXY=http://username:password@proxy.company.com:8080
export HTTPS_PROXY=http://username:password@proxy.company.com:8080

# Bypass proxy for specific hosts
export NO_PROXY=localhost,127.0.0.1,internal.company.com

# Use proxy for a single command
HTTPS_PROXY=http://proxy:8080 kopi install 21
```

**Notes:**
- Proxy settings are automatically detected from environment variables
- Both uppercase and lowercase variable names are supported
- Authentication credentials can be included in the proxy URL
- The `NO_PROXY` variable supports wildcards (e.g., `*.internal.com`)

Note: Minimum disk space requirement is configured via `~/.kopi/config.toml` (see Global Config section above)

## Security Considerations

Kopi implements several security measures to ensure safe operation:

### Path Validation

- All file operations are restricted to the KOPI_HOME directory (`~/.kopi` by default)
- Path traversal attempts (e.g., `../../../etc/passwd`) are blocked
- Symlinks are validated to ensure they don't point outside the kopi directory

### Version String Validation

- Version strings are validated to contain only safe characters (alphanumeric, `@`, `.`, `-`, `_`, `+`)
- Maximum length of 100 characters enforced
- Special patterns that could be used for injection attacks are rejected

### Tool Validation

- Only recognized JDK tools can be shimmed
- Unknown or system commands (e.g., `rm`, `curl`) are rejected
- Tool names are validated against a comprehensive registry

### File Permission Checks

- Shim targets must be executable files
- On Unix systems, world-writable files are rejected
- Regular file validation ensures directories cannot be executed

### Auto-Install Security

- Auto-installation prompts require explicit user confirmation
- Timeout protection prevents hanging on user input
- Version strings are validated before installation attempts

### Best Practices

1. **Regular Updates**: Keep kopi updated to get the latest security fixes
2. **Verify Downloads**: Kopi automatically verifies checksums for all JDK downloads
3. **Permission Management**: Ensure `~/.kopi` directory has appropriate permissions
4. **Audit Shims**: Periodically run `kopi shim verify` to check shim integrity

## Performance Characteristics

### Shim Overhead

Kopi's shims are designed for minimal performance impact:

- **Cold start**: < 10ms (first invocation)
- **Warm start**: < 5ms (subsequent invocations)
- **Total overhead**: Typically < 20ms including version resolution
- **Binary size**: < 1MB for optimized release builds

### Performance Optimizations

1. **Release Profile**: Shims are built with a custom `release-shim` profile
   - Link-time optimization (LTO) enabled
   - Single codegen unit for better optimization
   - Debug symbols stripped

2. **Efficient Tool Detection**: Uses a static registry for O(1) tool lookups

3. **Fast Version Resolution**: 
   - Caches version file locations
   - Minimal file I/O operations
   - Early exit on environment variable override

4. **Platform-Specific Optimizations**:
   - Direct process replacement on Unix (exec)
   - Efficient subprocess spawning on Windows

### Benchmark Results

Run benchmarks with:
```bash
cargo bench --bench shim_bench
```

Typical results on modern hardware:
- Tool detection: ~50ns
- Version validation: ~200ns
- Path validation: ~1μs
- Total shim overhead: ~5-20ms

### Comparison with Direct Execution

The shim overhead is negligible compared to JVM startup time:
- JVM cold start: 100-500ms
- Shim overhead: 5-20ms (2-4% of total)

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
- If behind a corporate proxy, set proxy environment variables (see HTTP Proxy Configuration)
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

### Shim-Specific Issues

**1. Shim Not Working**
```bash
Error: Tool 'java' not found in JDK
```
**Solution:**
- Ensure `~/.kopi/shims` is in your PATH
- Run `kopi shim verify` to check shim integrity
- Recreate the shim: `kopi shim add java --force`

**2. Version Not Switching**
```bash
# Wrong Java version despite .kopi-version file
```
**Solution:**
- Check version file location: must be in current or parent directory
- Verify version format: `temurin@21` or just `21`
- Check environment variable: `KOPI_JAVA_VERSION` overrides files
- Enable debug logging: `RUST_LOG=kopi::shim=debug java -version`

**3. Performance Issues**
```bash
# Slow shim execution
```
**Solution:**
- Run benchmarks: `cargo bench --bench shim_bench`
- Check for antivirus interference on Windows
- Ensure shims are built with release profile
- Verify no network delays in version resolution

**4. Security Validation Errors**
```bash
Error: Security error: Path contains directory traversal
```
**Solution:**
- Check for suspicious patterns in version files
- Ensure no malformed symlinks in kopi directories
- Run `kopi shim verify --fix` to repair issues

### Getting Help

If you encounter issues not covered here:

1. Run the command with verbose logging:
   ```bash
   kopi install 21 -vv
   ```

2. Check the GitHub issues: https://github.com/anthropics/claude-code/issues

3. For feedback or bug reports, please report the issue at:
   https://github.com/anthropics/claude-code/issues