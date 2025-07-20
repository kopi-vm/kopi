# Kopi Which Command Design

## Overview

The `kopi which` command shows the installation path for a JDK version, providing a way to locate Java executables and JDK installations. This design focuses on outputting the path to the java executable for the current or specified JDK version.

## Purpose

The primary purpose of `kopi which` is to:
- Locate the java executable path for the currently active JDK
- Find the installation path for a specific JDK version
- Verify that a JDK is properly installed by checking executable existence
- Provide path information for scripting and integration purposes
- Help users understand which Java installation is being used

## Command Specification

### Usage

```bash
kopi which [<version>] [options]
```

### Arguments

- `<version>` (optional): JDK version specification
  - Format: `<version>` or `<distribution>@<version>`
  - Examples: `21`, `temurin@21.0.5+11`, `corretto@17`
  - If omitted, shows path for the currently active JDK

### Options

- `--tool <name>`: Show path for specific JDK tool (default: java)
  - Examples: `javac`, `jar`, `jshell`, `native-image`
- `--home`: Show JDK home directory instead of executable path
- `--json`: Output in JSON format for programmatic use

### Alias

- `w`: Short alias for the which command

## Output Formats

### Default Output

Shows only the path to the java executable (or specified tool):

```bash
$ kopi which
/home/user/.kopi/jdks/temurin-21.0.5+11/bin/java

$ kopi which corretto@17
/home/user/.kopi/jdks/corretto-17.0.13.11.1/bin/java

$ kopi which --tool javac
/home/user/.kopi/jdks/temurin-21.0.5+11/bin/javac

$ kopi which --home
/home/user/.kopi/jdks/temurin-21.0.5+11
```

### JSON Format

Structured output for scripting:

```json
{
  "distribution": "temurin",
  "version": "21.0.5+11",
  "tool": "java",
  "tool_path": "/home/user/.kopi/jdks/temurin-21.0.5+11/bin/java",
  "jdk_home": "/home/user/.kopi/jdks/temurin-21.0.5+11",
  "source": "shell"
}
```

With `--tool javac`:
```json
{
  "distribution": "temurin",
  "version": "21.0.5+11",
  "tool": "javac",
  "tool_path": "/home/user/.kopi/jdks/temurin-21.0.5+11/bin/javac",
  "jdk_home": "/home/user/.kopi/jdks/temurin-21.0.5+11",
  "source": "shell"
}
```

## Version Resolution

### When No Version is Specified

The command follows the standard kopi version resolution hierarchy:

1. Environment variable: `KOPI_JAVA_VERSION`
2. Local project file: `.kopi-version` (walks up directory tree)
3. Local project file: `.java-version` (walks up directory tree)
4. Global configuration: `~/.kopi/config.toml`

### When Version is Specified

- Looks for exact match in installed JDKs
- Uses pattern matching for partial versions (e.g., `21` matches `21.0.5+11`)
- Returns error if multiple versions match ambiguously

## Implementation Architecture

### Core Components

1. **WhichCommand**: Main command handler
   - Parse command arguments
   - Resolve JDK version (current or specified)
   - Locate JDK installation
   - Format and output path information

2. **Version Resolution**: Reuse existing resolution logic
   - Use `resolve_version` for current JDK detection
   - Use `parse_jdk_spec` for version argument parsing
   - Leverage `JdkRepository` for finding installed JDKs

3. **Path Resolution**: Determine executable or home paths
   - Construct path to requested tool based on platform
   - Support various JDK tools (java, javac, jar, jshell, etc.)
   - Handle platform-specific differences (`.exe` on Windows)
   - Verify executable exists before returning path
   - Return JDK home when `--home` option is used

4. **Output Formatting**: Present path information
   - Default format outputs only the executable path
   - JSON format for structured data needs

### Platform Considerations

#### Unix-like Systems (Linux, macOS)
- Java executable: `<jdk_home>/bin/java`
- No file extension needed
- Use standard path separators

#### Windows
- Java executable: `<jdk_home>\bin\java.exe`
- Add `.exe` extension
- Handle backslash path separators
- Consider both native Windows paths and WSL paths

## Error Handling

### Exit Codes

Uses existing kopi exit codes from `src/error/exit_codes.rs`:

- `0`: Success, path found and output
- `1`: General error (default)
- `2`: ValidationError - Multiple ambiguous matches or invalid format
- `3`: NoLocalVersion - No version found when using current resolution
- `4`: JdkNotInstalled - Specified JDK is not installed
- `5`: ToolNotFound - Specified tool not found in JDK installation

### Error Scenarios

1. **No Current Version** (Exit code: 3)
   ```bash
   $ kopi which
   Error: No JDK configured for current project
   
   Run 'kopi global <version>' to set a default version
   ```

2. **JDK Not Installed** (Exit code: 4)
   ```bash
   $ kopi which temurin@22
   Error: JDK 'temurin@22' is not installed
   
   Run 'kopi install temurin@22' to install it
   ```

3. **Multiple Matches** (Exit code: 2)
   ```bash
   $ kopi which 21
   Error: Multiple JDKs match version '21'
   
   Found:
     temurin@21.0.5+11
     corretto@21.0.7.6.1
   
   Please specify the full version or distribution
   ```

4. **Corrupted Installation** (Exit code: 1)
   ```bash
   $ kopi which temurin@21
   Error: Java executable not found in JDK installation
   
   Expected: /home/user/.kopi/jdks/temurin-21.0.5+11/bin/java
   
   Run 'kopi install temurin@21 --force' to reinstall
   ```

5. **Tool Not Found** (Exit code: 5)
   ```bash
   $ kopi which --tool native-image
   Error: Tool 'native-image' not found in JDK installation
   
   The tool may not be included in this JDK distribution.
   ```

## Usage Examples

### Basic Usage

```bash
# Show current Java executable path
$ kopi which
/home/user/.kopi/jdks/temurin-21.0.5+11/bin/java

# Show path for specific version
$ kopi which corretto@17
/home/user/.kopi/jdks/corretto-17.0.13.11.1/bin/java

# Using alias
$ kopi w 21
/home/user/.kopi/jdks/temurin-21.0.5+11/bin/java

# Show path for specific tool
$ kopi which --tool javac
/home/user/.kopi/jdks/temurin-21.0.5+11/bin/javac

$ kopi which temurin@17 --tool jar
/home/user/.kopi/jdks/temurin-17.0.13.11.1/bin/jar

# Show JDK home directory
$ kopi which --home
/home/user/.kopi/jdks/temurin-21.0.5+11

$ kopi which corretto@21 --home
/home/user/.kopi/jdks/corretto-21.0.7.6.1
```

### Scripting Integration

```bash
# Get the path
JAVA_PATH=$(kopi which)

# Use in scripts
if java_path=$(kopi which temurin@21 2>/dev/null); then
    echo "Found Java at: $java_path"
    "$java_path" -version
else
    echo "Java 21 not installed"
fi

# JSON output for complex scripts
kopi which --json | jq -r '.jdk_home'

# Set JAVA_HOME using --home
export JAVA_HOME=$(kopi which --home)

# Find specific tool
JAVAC_PATH=$(kopi which --tool javac)
```

### CI/CD Usage

```yaml
# GitHub Actions example
- name: Verify Java Installation
  run: |
    JAVA_EXEC=$(kopi which)
    $JAVA_EXEC -version
    
# Or with specific version
- name: Check Java 21
  run: |
    if ! kopi which temurin@21 >/dev/null 2>&1; then
      echo "Java 21 not found, installing..."
      kopi install temurin@21
    fi
```

### Debugging JDK Issues

```bash
# Verify JDK is properly installed
$ kopi which temurin@21 --json | jq
{
  "distribution": "temurin",
  "version": "21.0.5+11",
  "tool": "java",
  "tool_path": "/home/user/.kopi/jdks/temurin-21.0.5+11/bin/java",
  "jdk_home": "/home/user/.kopi/jdks/temurin-21.0.5+11",
  "source": "global"
}

# Check if executable exists
$ test -x "$(kopi which)" && echo "Java is executable"

# Verify specific tools exist
$ test -x "$(kopi which --tool javac)" && echo "javac is available"
$ test -x "$(kopi which --tool native-image)" || echo "GraalVM native-image not found"
```

## Testing Strategy

### Unit Tests

1. **Version Resolution**
   - Test current version detection
   - Test specific version lookup
   - Test pattern matching behavior
   - Test ambiguous version handling

2. **Path Construction**
   - Test Unix path generation
   - Test Windows path generation
   - Test path validation logic
   - Test tool path construction (java, javac, jar, etc.)
   - Test --home option returns JDK directory

3. **Output Formatting**
   - Test default output format (path only)
   - Test JSON output structure

4. **Error Cases**
   - Test missing JDK scenarios
   - Test corrupted installation detection
   - Test multiple match scenarios
   - Test missing tool scenarios (ToolNotFound)

### Integration Tests

1. **Cross-Platform Testing**
   - Verify correct paths on Linux
   - Verify correct paths on macOS
   - Verify correct paths on Windows
   - Test WSL scenarios

2. **Real JDK Testing**
   - Test with actual installed JDKs
   - Verify executable existence
   - Test execution of returned path
   - Test various tools (javac, jar, jshell)
   - Verify --home returns valid JDK directory

3. **Version Resolution Testing**
   - Test with project files
   - Test with environment variables
   - Test with global configuration

## Security Considerations

1. **Path Traversal**: Ensure paths stay within kopi directory structure
2. **Command Injection**: Properly escape paths in output
3. **Symlink Security**: Validate symlinks don't point outside kopi directory
4. **No Code Execution**: Command only reports paths, never executes them

## Performance Considerations

Since `kopi which` may be used in scripts and shell prompts:

- **Target response time**: < 20ms for typical use cases
- **Optimization strategies**:
  - Cache JDK directory listings
  - Minimize file system operations
  - Fast path for current version lookup
  - Efficient pattern matching for version search

## Comparison with Similar Tools

### Traditional `which` command
- Standard `which` finds executables in PATH
- `kopi which` finds specific JDK installations
- Provides version-aware path resolution

### jenv
- `jenv which` shows shim location
- `kopi which` shows actual JDK executable
- More direct path to real binary

### SDKMAN
- `sdk home java <version>` shows JAVA_HOME
- `kopi which` shows java executable path
- More focused on executable location

### jabba
- `jabba which` shows current version name
- `kopi which` shows executable path
- Different focus (version vs path)

