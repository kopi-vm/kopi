# Kopi Development Guide

This guide contains technical information for developers working on the Kopi codebase.

## Debugging and Logging

Kopi provides flexible logging controls for troubleshooting and debugging:

### Verbosity Levels

Use the `-v/--verbose` flag (can be specified multiple times) with any command:

```bash
kopi install 21              # Default: warnings and errors only
kopi -v install 21           # Info level: show major operations
kopi -vv install 21          # Debug level: detailed flow information
kopi -vvv install 21         # Trace level: very detailed debugging
```

The verbose flag is global and works with all commands:

```bash
kopi -v list                 # Show info logs for list command
kopi -vv shell 21            # Debug version switching (alias: use)
kopi -vvv current            # Trace current version detection
```

### Environment Variable Control

For persistent logging or module-specific debugging, use the `RUST_LOG` environment variable:

```bash
# Set logging level for entire session
export RUST_LOG=debug
kopi install 21

# Debug specific modules
RUST_LOG=kopi::download=debug kopi install 21        # Debug downloads only
RUST_LOG=kopi::api=trace kopi cache search latest    # Trace API calls
RUST_LOG=kopi::storage=debug kopi uninstall temurin@21  # Debug storage operations

# Multiple module filters
RUST_LOG=kopi::download=debug,kopi::security=trace kopi install 21
```

### Common Debugging Scenarios

**Installation Issues:**

```bash
kopi -vv install 21          # See download URLs, checksums, extraction paths
```

**Version Resolution Problems:**

```bash
RUST_LOG=kopi::version=debug kopi install temurin@21  # Debug version parsing
```

**API Communication:**

```bash
RUST_LOG=kopi::api=debug kopi cache search latest     # Debug foojay.io API calls
```

**Storage and Disk Space:**

```bash
RUST_LOG=kopi::storage=debug kopi -v install 21       # Debug installation paths
```

## Security Considerations

Kopi implements several security measures to ensure safe operation:

### Path Validation

- Persistent artefacts (JDKs, shims, cache files) are created via `paths::*` helpers under `KOPI_HOME` (`~/.kopi` by default); temporary downloads use OS temp space but are moved back into Kopi-managed directories before use
- Path traversal attempts (e.g., `../../../etc/passwd`) are blocked by `shim::security::SecurityValidator::validate_path`
- Symlinks are validated (`validate_symlink`) to ensure they remain inside Kopi home

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

- When `auto_install.prompt` is enabled (default), Kopi asks for confirmation before fetching missing JDKs; disabling the prompt allows unattended installs
- Installation subprocesses are terminated if they exceed the configurable `auto_install.timeout_secs` budget
- Version strings are validated before installation attempts

### Best Practices

1. **Regular Updates**: Keep kopi updated to get the latest security fixes
2. **Verify Downloads**: Kopi automatically verifies checksums for all JDK downloads
3. **Permission Management**: Ensure `~/.kopi` directory has appropriate permissions
4. **Audit Shims**: Periodically run `kopi shim verify` to check shim integrity

## Performance Characteristics

### Shim Overhead

Kopi's shims are designed for minimal performance impact. The automated regression in `tests/shim_performance_test.rs` keeps average invocation latency below 50 ms and validates that metadata caching further improves warm-path execution on Unix-like platforms.

- **Cold start (average)**: < 50 ms in the cross-platform performance suite
- **Warm start**: Faster once `<distribution>-<version>.meta.json` descriptors exist alongside the JDK
- **Total overhead**: Remains well below JVM startup times, keeping overall session cost dominated by the Java process

### Performance Optimizations

1. **Release Profile**: Shims are built with a custom `release-shim` profile
   - Link-time optimization (LTO) enabled
   - Single codegen unit for better optimization
   - Debug symbols stripped

2. **Centralised Tool Registry**: `shim::tools::ToolRegistry` defines the supported command surface, keeping validation and discovery consistent across distributions

3. **Fast Version Resolution**:
   - Checks `KOPI_JAVA_VERSION` before touching the filesystem
   - Walks parent directories for `.kopi-version` / `.java-version` and stops at the first match
   - Falls back to the global default stored in `~/.kopi/version`

4. **Platform-Specific Optimizations**:
   - Direct process replacement on Unix (exec)
   - Efficient subprocess spawning on Windows

### Comparison with Direct Execution

The shim overhead is negligible compared to JVM startup time:

- JVM cold start: typically hundreds of milliseconds
- Shim overhead: < 50 ms average in continuous tests (a small fraction of total startup time)

## Debugging Environment Variables

- `RUST_LOG` - Control logging verbosity (see Debugging and Logging section)

## Exit Codes

Kopi uses specific exit codes to help with scripting and automation:

- `0`: Success
- `1`: General error (fallback for uncategorised failures)
- `2`: Invalid input or configuration error
- `3`: No local version found
- `4`: JDK not installed
- `5`: Requested tool not found in the active JDK
- `6`: Shell detection failed
- `7`: Unsupported shell
- `13`: Permission denied
- `17`: Resource already exists
- `20`: Network, HTTP, or metadata fetch error
- `28`: Disk space error
- `75`: Lock acquisition cancelled by user signal
- `127`: Command or shell not found
