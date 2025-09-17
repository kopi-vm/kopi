# Kopi Doctor Command Design

## Overview

The `kopi doctor` command is a diagnostic tool that checks the health of the kopi installation and provides actionable solutions for common issues. It performs a comprehensive examination of the kopi environment, including installation integrity, configuration validity, shell integration, and system dependencies.

## Purpose

The primary purpose of `kopi doctor` is to:

- Diagnose common installation and configuration problems
- Verify that kopi is properly integrated with the user's shell
- Check system dependencies and permissions
- Validate JDK installations and metadata cache
- Provide clear, actionable remediation steps
- Assist users in troubleshooting issues before they impact usage

## Command Specification

### Usage

```bash
kopi doctor [options]
```

### Options

- `--json`: Output results in JSON format for programmatic use
- `--verbose`: Show detailed diagnostic information
- `--check <category>`: Run only specific category of checks
  - Categories: `installation`, `shell`, `jdks`, `permissions`, `network`, `cache`

## Diagnostic Categories

### 1. Installation Checks

Verify the kopi installation itself:

- **Kopi Binary**: Check if kopi executable is accessible and has correct permissions
- **Version Compatibility**: Ensure kopi version is up-to-date
- **Installation Directory**: Verify `~/.kopi` directory structure exists
- **Required Directories**: Check for `jdks/`, `shims/`, `cache/` subdirectories
- **Config File**: Validate `~/.kopi/config.toml` if present

### 2. Shell Integration

Verify shell configuration and PATH setup:

- **PATH Configuration**: Check if `~/.kopi/shims` is in PATH
- **PATH Priority**: Ensure kopi shims come before system Java
- **Shell Detection**: Identify current shell (bash, zsh, fish, etc.)
- **Shell Configuration**: Check if shell init files are properly configured
- **Shim Functionality**: Test if shims can be executed

### 3. JDK Installation Health

Check installed JDKs:

- **Installation Integrity**: Verify JDK directories contain expected files
- **Executable Permissions**: Check if Java executables have correct permissions
- **Symlink Validity**: Ensure shims point to valid JDK executables
- **Version Consistency**: Match installed versions with metadata
- **Disk Space**: Check available space for JDK installations

### 4. Permission Checks

Verify file system permissions:

- **Write Permissions**: Check write access to kopi directories
- **Execute Permissions**: Verify execute permissions on binaries and shims
- **Ownership Issues**: Detect ownership mismatches
- **Platform-specific**: Handle Windows ACLs and Unix permissions

### 5. Network Connectivity

Test API connectivity:

- **Foojay API Access**: Test connection to api.foojay.io
- **HTTPS Support**: Verify TLS/SSL connectivity
- **Proxy Detection**: Check for proxy environment variables
- **DNS Resolution**: Verify DNS lookup for API endpoints

### 6. Cache Validation

Check metadata cache health:

- **Cache Existence**: Verify cache files exist
- **Cache Format**: Validate JSON structure
- **Cache Staleness**: Check if cache needs refresh
- **Cache Permissions**: Ensure cache is readable/writable

## Output Formats

### Default Output

Human-readable format with check results and recommendations:

```
Kopi Doctor v0.1.0
==================

Checking kopi installation health...

[✓] Installation
    ✓ Kopi binary found at /usr/local/bin/kopi
    ✓ Version 0.1.0 is up to date
    ✓ Installation directory exists: ~/.kopi
    ✓ All required subdirectories present

[✗] Shell Integration
    ✗ ~/.kopi/shims not found in PATH
      → Add the following to your ~/.zshrc:
        export PATH="$HOME/.kopi/shims:$PATH"
    ✓ Current shell: zsh
    ✗ Shell configuration not found
      → Run: kopi shell --apply

[✓] JDK Installations (3 found)
    ✓ temurin@21.0.5+11 - OK
    ✓ corretto@17.0.13.11.1 - OK
    ⚠ zulu@11.0.20 - Missing executable: jar
      → Reinstall with: kopi install zulu@11 --force

[✓] Permissions
    ✓ Write access to ~/.kopi
    ✓ Execute permissions on shims
    ✓ No ownership issues detected

[✓] Network
    ✓ Foojay API reachable
    ✓ HTTPS connectivity working
    ✓ No proxy detected

[⚠] Cache
    ⚠ Cache is 45 days old (stale)
      → Refresh with: kopi cache refresh
    ✓ Cache format valid
    ✓ Cache permissions OK

Summary: 2 issues found, 1 warning
```

### JSON Format

Structured output for scripting and CI/CD:

```json
{
  "version": "0.1.0",
  "timestamp": "2024-03-15T10:30:00Z",
  "summary": {
    "total_checks": 20,
    "passed": 17,
    "failed": 2,
    "warnings": 1
  },
  "categories": {
    "installation": {
      "status": "pass",
      "checks": [
        {
          "name": "binary_found",
          "status": "pass",
          "message": "Kopi binary found at /usr/local/bin/kopi"
        }
      ]
    },
    "shell": {
      "status": "fail",
      "checks": [
        {
          "name": "path_configured",
          "status": "fail",
          "message": "~/.kopi/shims not found in PATH",
          "suggestion": "Add 'export PATH=\"$HOME/.kopi/shims:$PATH\"' to ~/.zshrc"
        }
      ]
    }
  }
}
```

## Implementation Architecture

### Core Components

1. **DoctorCommand**: Main command handler
   - Orchestrate diagnostic checks
   - Handle command options
   - Format and display results

2. **DiagnosticEngine**: Core diagnostic framework
   - Run checks in categories
   - Aggregate results
   - Format diagnostic output

3. **Check Modules**: Individual diagnostic checks
   - `InstallationChecker`: Verify kopi installation
   - `ShellChecker`: Check shell integration
   - `JdkChecker`: Validate JDK installations
   - `PermissionChecker`: Test file permissions
   - `NetworkChecker`: Test connectivity
   - `CacheChecker`: Validate cache health

### Check Result Structure

```rust
pub struct CheckResult {
    pub name: String,
    pub category: CheckCategory,
    pub status: CheckStatus,
    pub message: String,
    pub details: Option<String>,
    pub suggestion: Option<String>,
}

pub enum CheckStatus {
    Pass,
    Fail,
    Warning,
    Skip,
}
```

## Error Handling

### Exit Codes

Uses existing kopi exit codes from `src/error/exit_codes.rs`:

- `0`: Success - all checks passed
- `1`: General error during diagnosis (default error code)
- `2`: ValidationError - invalid command options or configuration issues found
- `20`: NetworkError - network connectivity issues detected

Note: The doctor command returns 0 only when all checks pass. Any failed checks or warnings result in appropriate non-zero exit codes.

### Error Scenarios

1. **Permission Denied**

   ```
   Error: Cannot read kopi configuration
   Permission denied: ~/.kopi/config.toml

   Try running with appropriate permissions or check file ownership.
   ```

2. **Network Timeout**

   ```
   Warning: Network check timed out
   Unable to reach api.foojay.io

   This may indicate network issues or firewall restrictions.
   Check your internet connection and proxy settings.
   ```

## Advanced Diagnostics

### Verbose Mode

With `--verbose`, show additional details:

```
[✓] JDK Installations (3 found)
    ✓ temurin@21.0.5+11
      - Installation: ~/.kopi/jdks/temurin-21.0.5+11
      - Size: 312 MB
      - Executables: 38 found
      - Last used: 2024-03-14
      - Shims verified: java, javac, jar, jshell
```

### Category-Specific Checks

Run only specific categories:

```bash
# Check only shell integration
$ kopi doctor --check shell

# Check only JDK installations
$ kopi doctor --check jdks
```

## Platform-Specific Considerations

### Windows

- Check Windows-specific paths and permissions
- Verify junction points for shims
- Test PowerShell and CMD integration
- Check for WSL environments

### macOS

- Handle macOS security restrictions (Gatekeeper)
- Check for Homebrew conflicts
- Verify command line tools installation

### Linux

- Check distribution-specific issues
- Verify standard FHS compliance
- Test different shell environments

## Testing Strategy

### Unit Tests

1. **Check Logic**: Test individual diagnostic checks
2. **Result Aggregation**: Test summary generation
3. **Output Formatting**: Verify human and JSON output

### Integration Tests

1. **Full Diagnosis**: Run complete diagnostic suite
2. **Shell Detection**: Verify shell identification across platforms
3. **Network Checks**: Test with various network conditions

## Future Enhancements

1. **Diagnostic History**: Track diagnostic results over time
2. **Custom Checks**: Allow plugins for additional checks
3. **Performance Profiling**: Identify slow operations
4. **Integration Tests**: Check compatibility with IDEs and build tools
5. **Telemetry**: Anonymous diagnostic statistics (opt-in)

## Comparison with Similar Tools

### brew doctor (Homebrew)

- Focus on formula and tap issues
- Kopi focuses on JDK-specific concerns

### volta doctor

- Checks Node.js toolchain health
- Similar approach but different ecosystem

### sdk doctor (SDKMAN)

- Basic installation checks
- Kopi provides more comprehensive diagnostics

### rustup doctor

- Toolchain and component verification
- Similar category-based approach
