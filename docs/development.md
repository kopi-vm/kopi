# Kopi Development Guide

This guide contains technical information for developers working on the Kopi codebase.

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
RUST_LOG=kopi::storage=debug kopi uninstall 21       # Debug storage operations

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

### Comparison with Direct Execution

The shim overhead is negligible compared to JVM startup time:

- JVM cold start: 100-500ms
- Shim overhead: 5-20ms (2-4% of total)

## Debugging Environment Variables

- `RUST_LOG` - Control logging verbosity (see Debugging and Logging section)

## Exit Codes

Kopi uses specific exit codes to help with scripting and automation:

- `0`: Success
- `1`: General error
- `2`: Invalid input or configuration error
- `3`: No local version found
- `4`: JDK not installed
- `10`: Active JDK (reserved for future use)
- `13`: Permission denied
- `14`: Partial removal failure
- `17`: Resource already exists
- `20`: Network error
- `28`: Disk space error
- `127`: Command not found
