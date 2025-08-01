# Kopi Reference Manual

## Overview

Kopi is a JDK version management tool that integrates with your shell to seamlessly switch between different Java Development Kit versions. It uses a flexible metadata system that can fetch JDK information from multiple sources including cached HTTP endpoints and the foojay.io API, providing a simple, fast interface similar to tools like volta, nvm, and pyenv.

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

Remove an installed JDK version and free up disk space.

**Usage:**
```bash
kopi uninstall <version>                 # Remove an installed JDK version
kopi uninstall <distribution>@<version>  # Remove specific distribution
kopi uninstall <distribution> --all      # Remove all versions of a distribution
```

**Options:**
- `--force`: Skip confirmation prompts and safety checks
- `--dry-run`: Show what would be removed without actually removing
- `--all`: Remove all versions of a distribution (requires distribution name)
- `--cleanup`: Clean up failed or partial uninstall operations (can be used alone or with version)

**Examples:**
```bash
kopi uninstall 21                        # Remove Java 21 (must be only one installed)
kopi uninstall temurin@21.0.5+11         # Remove specific version
kopi uninstall corretto --all            # Remove all Corretto versions
kopi uninstall zulu@17 --dry-run         # Preview what would be removed
kopi uninstall temurin@21 --force        # Skip confirmation prompt
kopi uninstall --cleanup                 # Clean up failed uninstall operations
kopi uninstall --cleanup --force         # Force cleanup without confirmation
kopi uninstall --cleanup --dry-run       # Preview cleanup actions
kopi uninstall temurin@21 --cleanup      # Uninstall temurin@21 then perform cleanup
```

**Safety Features:**
- Requires exact specification when multiple JDKs match
- Shows disk space that will be freed
- Confirms removal before proceeding (unless `--force` is used)
- Atomic removal with rollback on failure
- Platform-specific cleanup (Windows antivirus handling, Unix symlink cleanup)

**Error Cleanup:**
If an uninstall fails, kopi provides cleanup functionality:
- Use `kopi uninstall --cleanup` to clean up failed operations
- Detects temporary removal directories (`.*.removing`)
- Finds partially removed JDKs (missing essential files)
- Cleans up orphaned metadata files
- Handles platform-specific cleanup scenarios

## Version Management Commands

### `kopi shell` (alias: `use`)

Set JDK version for current shell session. This command outputs shell-specific environment setup commands that should be evaluated by your shell.

**Usage:**
```bash
kopi shell <version>                     # Set JDK version for current shell session
kopi use <version>                       # Alias for 'kopi shell'
```

**Options:**
- `--shell <shell>`: Override shell detection (bash, zsh, fish, powershell)

**Examples:**
```bash
# Bash/Zsh
eval "$(kopi shell 21)"

# Fish
kopi shell 21 | source

# PowerShell
kopi shell 21 | Invoke-Expression

# Use specific shell format
eval "$(kopi shell 21 --shell bash)"
```

**Notes:**
- Automatically installs the JDK if not already installed
- Sets JAVA_HOME and updates PATH for the current shell session
- Changes are session-specific and don't affect other shells

### `kopi env`

Output environment variables for shell evaluation, similar to direnv. This command outputs shell-specific environment setup for JAVA_HOME without modifying PATH.

**Usage:**
```bash
kopi env                                 # Output environment variables for current JDK
kopi env <version>                       # Output environment variables for specific JDK
```

**Options:**
- `--shell <shell>`: Override shell detection (bash, zsh, fish, powershell, cmd)
- `--export`: Include export statement (default: true)

**Version Resolution:**
The command resolves the JDK version in the following order:
1. Explicit version parameter (if provided)
2. `KOPI_JAVA_VERSION` environment variable
3. `.kopi-version` file in current or parent directories
4. `.java-version` file in current or parent directories
5. Global default version

**Examples:**
```bash
# Bash/Zsh - Auto-detect version and shell
eval "$(kopi env)"

# Fish - Use current project version
kopi env | source

# PowerShell - Specific version
kopi env temurin@21 | Invoke-Expression

# Windows CMD - Use --shell flag
FOR /F "tokens=*" %i IN ('kopi env --shell cmd') DO %i

# Without export statement (just the value)
kopi env --export=false

# Use in shell hooks (.bashrc/.zshrc)
if command -v kopi &> /dev/null; then
    eval "$(kopi env)"
fi
```

**Shell-Specific Output Formats:**
- **Bash/Zsh**: `export JAVA_HOME="/path/to/jdk"`
- **Fish**: `set -gx JAVA_HOME "/path/to/jdk"`
- **PowerShell**: `$env:JAVA_HOME = "/path/to/jdk"`
- **CMD**: `set JAVA_HOME=/path/to/jdk`

**Notes:**
- Outputs to stdout for shell evaluation, stderr for messages
- Properly escapes paths with spaces and special characters
- Verifies JDK is installed before outputting
- Unlike `kopi shell`, this only sets JAVA_HOME without PATH modifications
- Ideal for integration with direnv, shell prompts, or custom scripts

### `kopi global`

Set the global default JDK version. This becomes the default for all new shell sessions.

**Usage:**
```bash
kopi global <version>                    # Set default JDK version globally
```

**Aliases:** `g`, `default`

**Examples:**
```bash
kopi global 21                           # Set Java 21 as global default
kopi global temurin@17.0.2               # Set specific distribution/version as default
kopi default corretto@21                 # Using 'default' alias
```

**Notes:**
- Automatically installs the JDK if not already installed
- Updates the global configuration in `~/.kopi/config.toml`
- Takes effect in new shell sessions

### `kopi local`

Set JDK version for the current project. Creates a `.kopi-version` file in the current directory.

**Usage:**
```bash
kopi local <version>                     # Set JDK version for current project
```

**Aliases:** `l`, `pin`

**Examples:**
```bash
kopi local 21                            # Use Java 21 for this project
kopi local corretto@17                   # Use Amazon Corretto 17
kopi pin temurin@21.0.1                  # Using 'pin' alias
```

**Notes:**
- Automatically installs the JDK if not already installed
- Creates `.kopi-version` file in the current directory
- Takes precedence over global settings
- Affects all subdirectories (walks up to find config)

## Information Commands

### `kopi list`

List all installed JDK versions with their distribution, version, and disk usage.

**Usage:**
```bash
kopi list                                # List installed JDK versions
```

**Alias:** `ls`

**Output includes:**
- Distribution name and icon
- Full version number
- Disk space usage
- Installation path

**Example output:**
```
Installed JDKs:
  ☕ temurin       21.0.5+11        489 MB   ~/.kopi/jdks/temurin-21.0.5+11
  🌳 corretto      17.0.13.11.1     324 MB   ~/.kopi/jdks/corretto-17.0.13.11.1
  🔷 zulu          11.0.25+9        298 MB   ~/.kopi/jdks/zulu-11.0.25+9
```

### `kopi current`

Show the currently active JDK version and details.

**Usage:**
```bash
kopi current                             # Show current JDK version and details
kopi current -q                          # Show only version number
kopi current --json                      # Output in JSON format
```

**Options:**
- `-q, --quiet`: Show only the version number without additional information
- `--json`: Output in JSON format for scripting

**Examples:**
```bash
kopi current
# Output: ☕ temurin 21.0.5+11 (current: shell)

kopi current -q
# Output: 21.0.5+11

kopi current --json
# Output: {"distribution":"temurin","version":"21.0.5+11","source":"shell"}
```

### `kopi which`

Show installation path for a JDK version or specific JDK tool.

**Usage:**
```bash
kopi which                               # Show path to current java executable
kopi which <version>                     # Show path for specific JDK version
kopi which --tool <tool>                 # Show path for specific tool (default: java)
kopi which --home                        # Show JDK home directory instead of executable path
```

**Alias:** `w`

**Options:**
- `--tool <tool>`: Show path for specific JDK tool (default: java)
- `--home`: Show JDK home directory instead of executable path
- `--json`: Output in JSON format for scripting

**Examples:**
```bash
kopi which                               # /home/user/.kopi/jdks/temurin-21.0.5+11/bin/java
kopi which 17                            # /home/user/.kopi/jdks/temurin-17.0.13+11/bin/java
kopi which --tool javac                  # /home/user/.kopi/jdks/temurin-21.0.5+11/bin/javac
kopi which --home                        # /home/user/.kopi/jdks/temurin-21.0.5+11
kopi which corretto@21 --json           # {"path":"/home/user/.kopi/jdks/corretto-21.0.5.12.1/bin/java",...}
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

### Default Distribution

The default distribution for installations is configured in `~/.kopi/config.toml`:

```toml
default_distribution = "temurin"
```

This setting determines which distribution is used when you install a JDK without specifying a distribution:
```bash
kopi install 21                          # Uses default distribution (temurin)
kopi install corretto@21                 # Explicitly uses corretto
```

To change the default distribution, edit the configuration file directly or use:
```bash
# Set a new global default JDK (also updates default distribution)
kopi global corretto@21
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

### `kopi doctor`

Run comprehensive diagnostics on your kopi installation to identify and fix common issues.

**Usage:**
```bash
kopi doctor                              # Run all diagnostic checks
kopi doctor --json                       # Output results in JSON format
kopi doctor --check <category>           # Run only specific category of checks

# Use global verbose flag for detailed output
kopi -v doctor                           # Show detailed diagnostic information
```

**Categories:**
- `installation`: Check kopi binary, version, directories, and configuration
- `shell`: Verify shell integration and PATH configuration
- `jdks`: Validate installed JDK integrity and disk usage
- `permissions`: Check file and directory permissions
- `network`: Test API connectivity and proxy settings
- `cache`: Validate cache files and check for staleness

**Examples:**
```bash
kopi doctor                              # Run all checks with colored output
kopi doctor --check network              # Check only network connectivity
kopi doctor --json > doctor-report.json  # Save results as JSON
kopi -v doctor                           # See detailed check information
```

**Exit Codes:**
- `0`: All checks passed
- `1`: One or more checks failed
- `2`: Warnings detected (no failures)
- `20`: Network error or timeout

**Features:**
- Parallel execution of independent checks for fast results
- Progress indicator for long-running checks
- Actionable suggestions for fixing detected issues
- Platform-specific recommendations (Windows, macOS, Linux)
- Performance optimization with caching of expensive operations
- Total timeout protection (30 seconds maximum)

## Cache Management Commands

### `kopi cache`

Manage the JDK metadata cache used for searching and installing JDK versions. Kopi uses a multi-source metadata system that provides:
- Fast access through pre-generated metadata files (HTTP source)
- Real-time data from the foojay.io API
- Offline capability with local metadata directories
- Automatic fallback between sources for reliability

#### `kopi cache refresh`

Update the metadata cache from configured sources.

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

**Notes:**
- The cache is automatically updated when needed during install operations
- Use `kopi refresh` as a shortcut for `kopi cache refresh`
- The `kopi cache update` command has been replaced with `kopi cache refresh`
- Metadata sources are tried in priority order with automatic fallback
- HTTP metadata source provides 20-30x faster access than direct API calls

## Supported Distributions

### Default Distributions

- `temurin` - Eclipse Temurin (formerly AdoptOpenJDK)
- `corretto` - Amazon Corretto
- `zulu` - Azul Zulu
- `openjdk` - OpenJDK
- `graalvm` - GraalVM
- `dragonwell` - Alibaba Dragonwell
- `sapmachine` - SAP Machine
- `liberica` - BellSoft Liberica
- `mandrel` - Red Hat Mandrel
- `kona` - Tencent Kona
- `semeru` - IBM Semeru
- `trava` - Trava OpenJDK

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
- Add `~/.kopi/shims` to PATH
- Creates shims for `java`, `javac`, `jar`, etc.
- Automatic version switching based on project configuration

The `kopi shell` command provides an alternative to shims:
- Outputs shell-specific commands to set JDK environment variables
- Sets `JAVA_HOME` and updates `PATH` to include JDK bin directory
- Must be evaluated by your shell (e.g., `eval "$(kopi shell 21)"`)
- Provides session-specific environments without modifying global PATH
- Respects project-specific JDK versions when executed within a project directory

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


## Environment Variables

Kopi respects the following environment variables:

### Kopi-specific Variables
- `KOPI_HOME` - Override default kopi home directory (default: `~/.kopi`)
- `JAVA_HOME` - Set by kopi when switching JDK versions
- `PATH` - Modified by kopi to include JDK bin directory

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

## Metadata System Architecture

Kopi uses a flexible metadata system that can fetch JDK information from multiple sources, providing both performance and reliability:

### Metadata Sources

1. **HTTP Metadata Source** (Primary)
   - Pre-generated metadata files hosted on GitHub Pages
   - Updated daily with the latest JDK releases
   - Platform-specific files for reduced data transfer
   - 20-30x faster than direct API access

2. **Foojay API** (Fallback)
   - Real-time data from api.foojay.io
   - Always up-to-date with the latest releases
   - Complete package information including download URLs
   - Used when HTTP source is unavailable

3. **Local Directory Source** (Optional)
   - For offline environments or bundled installations
   - Can be configured for air-gapped systems
   - Reads metadata from local filesystem

### How It Works

1. When you run commands like `kopi list` or `kopi install`, Kopi checks metadata sources in priority order
2. If the primary source (HTTP) is available, it fetches pre-generated metadata quickly
3. If the primary source fails, it automatically falls back to the Foojay API
4. Results are cached locally to improve subsequent operations
5. The system includes lazy loading for package details to minimize data transfer

### Configuration

The metadata system can be configured in `~/.kopi/config/config.toml`:

```toml
[metadata]
# HTTP source (fastest, default priority)
[[metadata.sources]]
type = "http"
name = "github"
enabled = true
base_url = "https://kopi-vm.github.io/metadata"

# Foojay API (fallback)
[[metadata.sources]]
type = "foojay"
name = "foojay-api"
enabled = true

# Local directory (optional, for offline use)
[[metadata.sources]]
type = "local"
name = "bundled"
enabled = false
directory = "${KOPI_HOME}/bundled-metadata"
```

### Performance Benefits

- **List operations**: ~100ms (vs 2-3 seconds with API-only)
- **Search operations**: ~50ms (vs 1-2 seconds with API-only)
- **Automatic caching**: Reduces repeated network requests
- **Lazy loading**: Fetches full package details only when needed

## Troubleshooting

For troubleshooting common issues and solutions, see the [Troubleshooting Guide](troubleshooting.md).

## Developer Documentation

For information about debugging, security considerations, and performance characteristics, see the [Development Guide](development.md).