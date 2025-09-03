# kopi-shim Binary Size Analysis

Date: 2025-07-30

## Summary

Investigation into why kopi-shim binary size is larger than expected, currently 1.7MB with release-shim profile and 2.5MB with standard release profile.

## Current State

### Binary Sizes

- Standard release profile: **2.5MB**
- release-shim profile: **1.7MB** (32% reduction achieved through optimization flags)

### Build Configuration (release-shim profile)

```toml
[profile.release-shim]
inherits = "release"
lto = "fat"
codegen-units = 1
opt-level = "z"  # Optimize for size
panic = "abort"  # Smaller binary without unwinding
strip = true     # Strip symbols
```

## Size Analysis

### Top Contributors (cargo bloat analysis)

1. `regex_automata` - 69.2KB (6.2%) - Regular expression engine
2. `kopi::shim::run_shim` - 21.3KB (1.9%) - Main shim function
3. `toml_edit` - 20.7KB (1.8%) - TOML configuration parsing
4. `std::backtrace` - 31.6KB total - Stack trace support
5. `config` crate - 42.8KB total - Configuration management system

### Root Cause

The primary issue is that kopi-shim links the entire `kopi` library, which includes many features unnecessary for a shim:

1. **Unnecessary Dependencies**
   - `attohttpc` - HTTP client for API calls
   - `tar`, `zip` - Archive extraction
   - `indicatif` - Progress bars
   - `clap` - CLI argument parsing
   - `chrono` - Date/time handling
   - `uuid` - UUID generation
   - `walkdir` - Directory traversal
   - `sysinfo` - System information

2. **Unnecessary Features**
   - Auto-installation functionality
   - Cache management
   - Doctor diagnostics
   - Metadata fetching
   - Archive extraction
   - Download management

## Ideal Shim Requirements

A minimal shim only needs:

- Read `.kopi-version` or `.java-version` files
- Resolve JDK installation path
- Execute the appropriate Java binary
- Basic error handling

These requirements could be implemented in a few hundred KB.

## Recommendations

1. **Create a separate crate** for the shim with minimal dependencies
2. **Extract only essential code** from the main library
3. **Avoid heavy dependencies** like regex, toml parsers if possible
4. **Use simple file parsing** instead of full configuration system
5. **Remove all network-related code** from the shim

## Related Reviews

- [2025-07-06-shim-binary-size.md](./2025-07-06-shim-binary-size.md) - Initial binary size concerns
- [2025-07-23-shim-dependency-analysis.md](./2025-07-23-shim-dependency-analysis.md) - Dependency analysis

## Conclusion

The current architecture where the shim includes the entire kopi library is the main cause of the large binary size. A dedicated minimal shim implementation could reduce the size by 80-90%.
