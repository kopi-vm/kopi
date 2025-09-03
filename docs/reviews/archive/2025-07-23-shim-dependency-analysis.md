# Shim Dependency Analysis (2025-07-23)

This document provides a detailed analysis of the kopi-shim binary dependencies and identifies optimization opportunities to further reduce the binary size below the 1MB target. This is a follow-up to the previous binary size review from 2025-07-06.

## Summary

The kopi-shim binary size has been significantly reduced from 6.1MB to 1.6MB through the implementation of AutoInstaller delegation and the release-shim optimization profile. However, it still exceeds the 1MB target. This analysis identifies the remaining dependencies and proposes targeted optimizations to achieve the desired size.

## Current State

### Binary Size Measurement

```bash
$ cargo build --profile release-shim --bin kopi-shim && ls -lh target/release-shim/kopi-shim
-rwxr-xr-x 2 vscode vscode 1.6M Jul 23 22:36 target/release-shim/kopi-shim
```

### Implemented Optimizations

- AutoInstaller now delegates to main kopi binary via subprocess (removed HTTP/archive dependencies)
- release-shim profile with aggressive size optimizations:
  - `lto = "fat"`
  - `codegen-units = 1`
  - `opt-level = "z"`
  - `panic = "abort"`
  - `strip = true`

## Dependency Analysis

### Essential External Dependencies

1. **Configuration & Serialization**
   - `config` - TOML configuration loading
   - `serde`/`serde_json` - JSON metadata parsing
   - `toml` - TOML parsing (via config crate)
   - `dirs` - Platform-specific directory paths

2. **Error Handling & Logging**
   - `thiserror` - Error type definitions
   - `env_logger` - Logging setup
   - `log` - Logging macros
   - `colored` - Terminal color output

3. **Platform-Specific**
   - `libc` (Unix) - File permissions and exec()
   - `winapi` (Windows) - Process and file operations
   - `sysinfo` - Shell detection

### Internal Module Dependencies

The shim requires these core kopi modules:

- `shim/` - Main runtime logic with security, tools, discovery submodules
- `config/` - Configuration management
- `error/` - Error types and formatting
- `platform/` - OS-specific operations
- `version/` - Version parsing and resolution
- `storage/` - JDK repository listing
- `installation/` - Auto-installer (delegates to subprocess)

## Size Impact Analysis

### Major Contributors to Binary Size

1. **config crate (~400KB)** - Brings in heavy dependencies:
   - Multiple format parsers (JSON, TOML, YAML capabilities)
   - Async runtime components
   - Case conversion utilities

2. **sysinfo crate (~200KB)** - Used only for shell detection:
   - Full system information gathering capabilities
   - Process enumeration and statistics
   - Memory and CPU monitoring

3. **serde ecosystem (~150KB)** - JSON and configuration parsing:
   - Full serialization/deserialization framework
   - Multiple format support

4. **colored crate (~50KB)** - Terminal color support:
   - ANSI escape code handling
   - Windows console API integration

5. **env_logger (~100KB)** - Logging framework:
   - Regex for log filtering
   - Timestamp formatting

## Optimization Recommendations

### 1. Feature Flags (Immediate Impact)

Add conditional compilation to exclude non-essential features:

```toml
[features]
default = []
minimal-shim = ["no-color", "no-logging", "simple-config"]
no-color = []
no-logging = []
simple-config = []

[dependencies]
colored = { version = "3.0.0", optional = true }
env_logger = { version = "0.11", optional = true }
log = { version = "0.4.27", optional = true }
```

Estimated reduction: ~150KB

### 2. Replace Heavy Dependencies

**config → Custom TOML Parser**

- Implement minimal TOML parsing for just the needed configuration
- Or use environment variables for shim configuration
- Estimated reduction: ~400KB

**sysinfo → Direct Shell Detection**

- Implement platform-specific shell detection without full sysinfo
- Use environment variables and process inspection
- Estimated reduction: ~200KB

**colored → Plain Text Errors**

- Remove color formatting for error messages
- Use simple string formatting
- Estimated reduction: ~50KB

### 3. Optimize Serialization

**Minimize serde features**

- Use only derive feature for serde
- Consider manual JSON parsing for simple metadata
- Estimated reduction: ~50KB

### 4. Build Configuration

**Additional optimization flags**

```toml
[profile.release-shim]
# ... existing settings ...
# Additional size optimizations
debug = false
overflow-checks = false
incremental = false
```

### 5. Code Structure Optimizations

- Remove unused error variants and context
- Simplify version matching logic
- Eliminate redundant validation in hot path

## Implementation Priority

1. **Phase 1: Feature Flags** (1.6MB → ~1.4MB)
   - Add minimal-shim feature
   - Conditional compilation for colored, logging

2. **Phase 2: Replace config** (1.4MB → ~1.0MB)
   - Implement minimal TOML reader
   - Or switch to environment-only config

3. **Phase 3: Replace sysinfo** (1.0MB → ~0.8MB)
   - Direct shell detection implementation
   - Platform-specific minimal code

4. **Phase 4: Further optimizations** (0.8MB → target)
   - Manual JSON parsing
   - Code size optimizations

## Conclusion

The 1MB target is achievable through a combination of feature flags and targeted dependency replacement. The highest impact will come from replacing the `config` crate with a minimal implementation and removing `sysinfo` in favor of direct shell detection. These changes maintain the shim's core functionality while significantly reducing binary size.
