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
kopi install --list                       # List available JDK versions from foojay.io
```

**Examples:**
```bash
kopi install 21                          # Latest Java 21 (uses default distribution)
kopi install 21.0.1                      # Specific version (uses default distribution)
kopi install temurin@17.0.2              # Specific distribution and version
kopi install corretto@21                 # Latest Java 21 from Amazon Corretto
kopi install zulu@11.0.15                # Zulu JDK version 11.0.15
kopi install 21 --lts                    # Latest LTS of Java 21
kopi install latest --lts                # Latest LTS version
```

**Options:**
- `--arch <arch>`: Specify architecture (auto-detected by default)
- `--type <type>`: JDK type (jdk, jre)
- `--lts`: Filter/install only LTS versions
- `--latest`: Install latest version matching criteria
- `--quiet/-q`: Suppress output
- `--verbose/-v`: Detailed output

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

Update metadata cache from foojay.io.

**Usage:**
```bash
kopi refresh                             # Update metadata cache from foojay.io
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

## Environment Variables

Kopi respects the following environment variables:

- `KOPI_HOME` - Override default kopi home directory (default: `~/.kopi`)
- `JAVA_HOME` - Set by kopi when switching JDK versions
- `PATH` - Modified by kopi to include JDK bin directory

Note: Minimum disk space requirement is configured via `~/.kopi/config.toml` (see Global Config section above)