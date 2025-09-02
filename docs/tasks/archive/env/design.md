# Kopi Env Command Design

## Overview

The `kopi env` command outputs environment variables for shell evaluation, providing a lightweight mechanism to configure Java-related environment variables without modifying PATH or launching interactive shells. This design focuses on outputting shell scripts that set `JAVA_HOME` environment variable.

## Purpose

The primary purpose of `kopi env` is to provide environment variable configuration that can be:
- Integrated into shell initialization scripts (`.bashrc`, `.zshrc`, etc.)
- Used with shell hooks for automatic environment setup
- Combined with tools like `direnv` for project-specific environments
- Used in CI/CD pipelines and scripts

Unlike `kopi shell`, which provides an interactive session with full PATH management, `kopi env` focuses solely on environment variable output, making it suitable for non-interactive use cases.

## Command Specification

### Usage

```bash
kopi env [<version>] [options]
```

### Arguments

- `<version>` (optional): JDK version specification
  - Format: `<version>` or `<distribution>@<version>`
  - Examples: `21`, `temurin@21.0.5+11`, `corretto@17`
  - If omitted, uses version resolution hierarchy

### Options

- `--shell <shell>`: Override shell detection (bash, zsh, fish, powershell, cmd)
- `--export`: Include export statements (default: true for bash/zsh/fish, false for others)
- `--quiet`: Suppress error messages, output nothing on failure (exit codes are still set)

## Shell Output Formats

### Bash/Zsh

```bash
export JAVA_HOME="/home/user/.kopi/jdks/temurin-21.0.5+11"
```

### Fish

```fish
set -gx JAVA_HOME "/home/user/.kopi/jdks/temurin-21.0.5+11"
```

### PowerShell

```powershell
$env:JAVA_HOME = "C:\Users\user\.kopi\jdks\temurin-21.0.5+11"
```

### Windows Command Prompt (cmd)

```cmd
set JAVA_HOME=C:\Users\user\.kopi\jdks\temurin-21.0.5+11
```

## Version Resolution

When no version is specified, the command follows the standard kopi version resolution hierarchy:

1. Environment variable: `KOPI_JAVA_VERSION`
2. Local project file: `.kopi-version` (walks up directory tree)
3. Local project file: `.java-version` (walks up directory tree)
4. Global configuration: `~/.kopi/config.toml`

If no version is found through any of these methods, the command exits with an error (unless `--quiet` is specified).

## Implementation Architecture

### Core Components

1. **EnvCommand**: Main command handler
   - Parse command arguments
   - Resolve JDK version
   - Validate JDK installation
   - Generate shell-specific output

2. **Shell Detection**: Reuse existing `platform::shell` module
   - Use `detect_shell()` for automatic detection
   - Use `parse_shell_name()` for `--shell` option parsing
   - Existing implementation detects parent process using sysinfo crate
   - Falls back to `SHELL` environment variable on Unix
   - Supports: bash, zsh, fish, powershell/pwsh, cmd

3. **EnvFormatter**: Format environment variables for different shells
   - Generate shell-specific syntax
   - Handle path separators per platform
   - Support export/non-export modes

### Integration Points

1. **Version Resolution**: Reuse existing `resolve_version` functionality
2. **JDK Installation**: Use `JdkInstallation` to verify JDK exists
3. **Platform Detection**: Leverage existing platform utilities
4. **Error Handling**: Use existing `KopiError` types:
   - `NoLocalVersion { searched_paths }` when no version is found
   - `JdkNotInstalled { jdk_spec, version, auto_install_enabled }` when JDK is missing
   - `UnsupportedShell(shell)` when invalid shell is specified

## Error Handling

### Exit Codes

Uses existing kopi exit codes from `src/error/exit_codes.rs`:

- `0`: Success, environment variables output
- `1`: General error (default)
- `3`: NoLocalVersion - No version found through resolution hierarchy
- `4`: JdkNotInstalled - Specified JDK is not installed
- `7`: UnsupportedShell - Invalid shell specified with --shell option

### Error Scenarios

The existing `ErrorContext` system in `src/error/context.rs` automatically provides helpful suggestions for each error type:

1. **No Version Found** (Exit code: 3)
   ```bash
   $ kopi env
   Error: No JDK configured for current project
   
   Run 'kopi global <version>' to set a default version
   ```

2. **JDK Not Installed** (Exit code: 4)
   ```bash
   $ kopi env temurin@21
   Error: JDK 'temurin@21' is not installed
   
   Run 'kopi install temurin@21' to install it
   ```

3. **Invalid Shell** (Exit code: 7)
   ```bash
   $ kopi env --shell invalid
   Error: Shell 'invalid' is not supported
   
   Supported shells: bash, zsh, fish, powershell, cmd.
   ```

## Usage Examples

### Shell Initialization

Add to `.bashrc` or `.zshrc`:
```bash
# Automatically set JAVA_HOME based on kopi configuration
eval "$(kopi env)"
```

### Project-Specific Setup

In project directory with `.kopi-version`:
```bash
$ cat .kopi-version
temurin@21

$ eval "$(kopi env)"
$ echo $JAVA_HOME
/home/user/.kopi/jdks/temurin-21.0.5+11
```

### CI/CD Integration

```yaml
# GitHub Actions example
- name: Setup Java Environment
  run: |
    eval "$(kopi env)"
    echo "JAVA_HOME=$JAVA_HOME" >> $GITHUB_ENV
```

### Direnv Integration

In `.envrc`:
```bash
eval "$(kopi env)"
```

### Manual Version Override

```bash
$ eval "$(kopi env corretto@17)"
$ echo $JAVA_HOME
/home/user/.kopi/jdks/corretto-17.0.13.11.1
```

## Testing Strategy

### Unit Tests
- Version resolution logic
- Shell detection accuracy
- Output formatting for each shell
- Error handling scenarios

### Integration Tests
- Test with actual shells (bash, zsh, fish)
- Verify environment variable setting
- Test with various version specifications
- Test project file detection

### Platform Tests
- Linux: bash, zsh, fish
- macOS: bash, zsh, fish
- Windows: PowerShell, cmd

## Security Considerations

1. **Shell Injection**: Properly escape all paths and values
2. **Path Validation**: Ensure JDK paths are within kopi directory
3. **No Arbitrary Code**: Only output variable assignments
4. **Safe Defaults**: Use `--export` by default for interactive shells

## Performance Considerations

### Performance Requirements

Since `kopi env` is designed to be used in shell hooks (e.g., directory change hooks, prompt hooks), performance is critical:

- **Target response time**: < 50ms for typical use cases
- **Maximum acceptable time**: < 100ms including version resolution
- Shell hooks are executed frequently, so any delay impacts user experience

### Performance Optimization Strategy

1. **Initial Implementation**: Add `env` subcommand to main kopi binary
2. **Benchmark Creation**: Measure performance after implementation
   - Cold start time (first execution)
   - Warm execution time (subsequent calls)
   - Version resolution overhead
   - File I/O impact

3. **Alternative Implementation**: If benchmarks show inadequate performance
   - Create separate `kopi-env` binary with minimal dependencies
   - Remove unnecessary features (e.g., network capabilities, progress bars)
   - Focus solely on local file operations and environment output
   - Potential optimizations:
     - Static linking for faster startup
     - Minimal dependency tree
     - Pre-compiled version resolution logic
     - Memory-mapped file access for config files

### Benchmark Metrics

The following metrics will be measured post-implementation:

```bash
# Benchmark scenarios
1. Simple case: eval "$(kopi env)" with global config
2. Project case: eval "$(kopi env)" in directory with .kopi-version
3. Complex case: Deep directory with multiple parent .kopi-version files
4. Error case: Missing JDK version

# Measurement points
- Binary startup time
- Config file parsing time  
- Version resolution time
- Total execution time
```

### Decision Criteria

After benchmarking, if total execution time exceeds 100ms in common scenarios:
1. Analyze bottlenecks using profiling tools
2. Attempt optimization within main binary
3. If still inadequate, implement `kopi-env` as separate lightweight binary

This approach ensures we maintain a single binary for simplicity unless performance requirements demand otherwise.

## Comparison with Similar Tools

### direnv
- `direnv` provides general environment management
- `kopi env` focuses specifically on Java environment
- Can be used together for comprehensive setup

### jenv
- `jenv` modifies PATH and provides shims
- `kopi env` only outputs environment variables
- Lighter weight, suitable for non-interactive use

### SDKMAN
- `SDKMAN` requires sourcing functions
- `kopi env` outputs simple variable assignments
- More compatible with standard shell practices