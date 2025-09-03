# macOS JDK Bundle Structure Handling Design

## Overview

This design document details the implementation strategy for handling diverse JDK directory structures on macOS, particularly the application bundle format (`Contents/Home`) used by many distributions. The solution ensures transparent JDK management across different packaging formats while maintaining compatibility with existing Kopi functionality.

## Purpose

The primary purpose of this implementation is to:

- Automatically detect and handle different JDK directory structures on macOS
- Provide transparent JAVA_HOME resolution regardless of packaging format
- Maintain performance by leveraging metadata caching
- Ensure compatibility with macOS ecosystem tools and practices

Unlike Linux and Windows where JDKs use a consistent direct structure, macOS distributions employ various layouts that require special handling to function correctly.

## Problem Statement

### Current Issue

Kopi currently assumes a direct directory structure for all platforms. On macOS, different JDK distributions use varying structures:

1. **Bundle Structure** (Temurin, TencentKona): `Contents/Home/bin/java`
2. **Direct Structure** (Liberica): `bin/java`
3. **Hybrid Structure** (Azul Zulu): Symlinks at root pointing to bundle structure

This causes installation failures when Kopi cannot locate Java binaries in expected locations.

### Root Cause

macOS follows the application bundle convention where applications are packaged as `.app` directories with a specific internal structure. Many JDK distributions follow this convention for consistency with the macOS ecosystem, while others maintain cross-platform compatibility with direct structures.

## Solution Architecture

### Detection Strategy

Implement a multi-phase detection algorithm that checks for different structure patterns:

```
1. Check for bin/ at root (handles direct and hybrid structures)
2. Check for Contents/Home/ (handles bundle structures)
3. Check nested directories for Contents/Home/
4. Fallback: recursive search for bin/java
```

### Directory Structure Preservation

**Key Decision**: Preserve original structures rather than normalizing them.

- Bundle structures remain as `Contents/Home/`
- Direct structures remain at root
- Hybrid structures preserve symlinks

This approach:

- Maintains code signing and notarization
- Preserves vendor intentions
- Simplifies implementation
- Enables future `/usr/libexec/java_home` integration

## Implementation Components

### 1. Structure Detection Module (Installation Time)

**Location**: `src/archive/mod.rs`

**Core Function**:

The primary detection function takes the extracted directory path and distribution name as inputs, returning the resolved JDK root directory path or an error if no valid structure is found.

**Important Distinction**:

- This module is used ONLY during installation (before `InstalledJdk` exists)
- It detects structure from a temporary extraction directory
- Results are saved as metadata for later use by `InstalledJdk`
- Cannot use `InstalledJdk` methods because the JDK isn't installed yet

**Responsibilities**:

- Detect JDK structure type after extraction
- Validate presence of essential components (`bin/java`)
- Determine `java_home_suffix` value (e.g., "Contents/Home" or "")
- Return appropriate JDK root path and structure information

### 2. Metadata Extension

**Location**: `src/storage/mod.rs`

**Enhanced Metadata Structure**:

```json
{
  // Existing API fields
  "distribution": "temurin",
  "java_version": "21.0.2",

  // New installation metadata
  "installation_metadata": {
    "java_home_suffix": "Contents/Home",
    "structure_type": "bundle",
    "platform": "macos_aarch64"
  }
}
```

**Metadata Creation Flow**:

1. During installation, `detect_jdk_root()` returns structure information
2. Installation process creates `installation_metadata` object
3. Existing `save_jdk_metadata()` is extended to include this object
4. Metadata file is written to disk alongside the installed JDK

**Metadata Usage Flow**:

1. `InstalledJdk` is created from directory name (doesn't read metadata yet)
2. When `resolve_java_home()` or `resolve_bin_path()` is called:
   - First attempt to load and cache metadata
   - Use `java_home_suffix` if available
   - Fall back to detection if metadata missing or invalid

### 3. Installation Integration

**Location**: `src/commands/install.rs`

**Process Flow**:

1. Extract archive to temporary location
2. Detect JDK structure using detection module
3. Log detected structure type at INFO level with format:
   - Include structure type (bundle/direct/hybrid)
   - Include distribution name and version
   - Example output: "Detected bundle structure for temurin@21.0.2"
4. Move appropriate directory to final installation path
5. Save structure metadata for runtime use (using Metadata Extension from #2)

### 4. Common Path Resolution

**Location**: `src/storage/listing.rs` (InstalledJdk struct)

**Purpose**:
Centralize all JDK path resolution logic in one place, used by both shim and env command at runtime.

**Implementation**:

Add two methods to the `InstalledJdk` struct:

1. **resolve_java_home() -> PathBuf**:
   - Returns the correct JAVA_HOME path for this JDK
   - On macOS: Checks for Contents/Home and appends if present
   - On other platforms: Returns the JDK path directly
   - Uses cached metadata if available, falls back to detection

2. **resolve_bin_path() -> PathBuf**:
   - Returns the path to the bin directory
   - Internally calls resolve_java_home() and appends "bin"
   - Used by shim to find executables

**Metadata Loading**:

- Lazy-loads metadata from `.meta.json` file in JDK directory
- Caches result in struct field for subsequent calls
- Falls back to runtime detection if metadata missing or invalid

**Fallback Detection**:
When metadata is missing or corrupted:

1. Check if `self.path.join("bin").exists()` (direct structure)
2. Check if `self.path.join("Contents/Home/bin").exists()` (bundle structure)
3. Log warning about missing metadata
4. Return detected path

**Platform Abstraction**:

- Uses `#[cfg(target_os = "macos")]` for macOS-specific logic
- Other platforms always use direct structure (no special handling)

### 5. Shim Enhancement

**Location**: `src/bin/kopi-shim.rs` and `src/shim/mod.rs`

**Integration with Common Path Resolution**:

The shim will leverage the `InstalledJdk` methods for path resolution:

**Updated Process Flow**:

1. `find_jdk_installation` returns an `InstalledJdk` instance (not just a path)
2. `build_tool_path` uses the `InstalledJdk` instance:
   - Calls `jdk.resolve_bin_path()` to get the correct bin directory
   - Appends the tool name to create full path
   - No need for shim-specific metadata loading or structure detection
3. Execute tool binary directly (without setting JAVA_HOME)

**Benefits of This Approach**:

- Shim doesn't duplicate path resolution logic
- Consistent behavior with env command
- Easier to maintain and test
- Performance remains optimal (metadata cached within InstalledJdk)

**Important Note**:
The current shim implementation does NOT set JAVA_HOME environment variable. It directly executes the Java binary using the resolved path. This is sufficient for most use cases since:

- The Java process itself doesn't require JAVA_HOME to be set
- Tools that need JAVA_HOME should use `kopi env` instead
- This approach avoids potential conflicts with existing JAVA_HOME settings

**Performance Considerations**:

- Metadata read: ~1ms (single file read)
- Fallback directory check: ~5ms (single stat call)
- Total overhead remains under 10ms even in worst case

**Cache Strategy**:
Since shim processes are short-lived, no in-memory caching is implemented. The metadata file acts as the persistent cache, eliminating repeated directory structure detection.

### 6. Env Command Integration

**Location**: `src/commands/env.rs`

**Integration with Common Path Resolution**:

The env command will use the same `InstalledJdk` methods as the shim:

**Process Flow**:

1. Resolve version and find matching `InstalledJdk` instance
2. Call `jdk.resolve_java_home()` to get the correct JAVA_HOME path
   - This method handles all platform-specific logic internally
   - Automatically checks metadata and performs fallback detection
3. Format the JAVA_HOME value for the target shell
4. Output shell-appropriate syntax

**Benefits of Shared Implementation**:

- No duplication of path resolution logic
- Guaranteed consistency between shim and env command
- Single source of truth for JDK structure handling
- Simplified testing and maintenance

**Shell Output Examples**:

- Bash/Zsh: `export JAVA_HOME="/Users/user/.kopi/jdks/temurin-21.0.2-aarch64/Contents/Home"`
- Fish: `set -gx JAVA_HOME "/Users/user/.kopi/jdks/temurin-21.0.2-aarch64/Contents/Home"`
- PowerShell: `$env:JAVA_HOME = "C:\Users\user\.kopi\jdks\temurin-21.0.2-x64"`

The path resolution complexity is entirely hidden within the `InstalledJdk` implementation.

## Processing Timeline

### Installation Time (Before InstalledJdk Exists)

**Components Used**:

- `detect_jdk_root()` in `src/archive/mod.rs`
- Cannot use `InstalledJdk` methods (JDK not installed yet)

**Process**:

1. Extract archive to temporary directory
2. Call `detect_jdk_root()` to analyze structure
3. Determine `java_home_suffix` ("Contents/Home", "", etc.)
4. Move files to final installation location
5. Save metadata including `java_home_suffix`
6. Create `InstalledJdk` entry for future use

### Runtime (After Installation)

**Components Used**:

- `InstalledJdk::resolve_java_home()`
- `InstalledJdk::resolve_bin_path()`

**Process**:

1. Load `InstalledJdk` instance from installed JDK
2. Call appropriate resolve method:
   - Shim: Uses `resolve_bin_path()`
   - Env: Uses `resolve_java_home()`
3. Methods internally:
   - Check for cached metadata
   - Use `java_home_suffix` if available
   - Fall back to runtime detection if needed

### Key Difference

- **Installation**: Detects structure from scratch, saves result
- **Runtime**: Uses saved metadata, falls back to detection only if necessary
- **Shared Logic**: While the detection algorithm is similar, it's implemented in different places for different contexts

### Why Not Share the Detection Logic?

1. **Different Contexts**:
   - Installation works with temporary extraction directories
   - Runtime works with installed JDK directories
   - Different error handling requirements

2. **Different Data Structures**:
   - Installation uses raw filesystem paths
   - Runtime uses `InstalledJdk` instances with additional context

3. **Performance Considerations**:
   - Installation can afford more thorough checking (one-time operation)
   - Runtime needs to be optimized for speed (frequent execution)

4. **Dependency Direction**:
   - `InstalledJdk` depends on metadata created during installation
   - Installation cannot depend on `InstalledJdk` (circular dependency)

## Platform-Specific Behavior

### macOS

- Perform full structure detection
- Apply `Contents/Home` suffix when present
- Preserve bundle attributes and symlinks

### Linux/Windows

- Skip structure detection (always direct)
- Use installation path as JAVA_HOME directly
- No special handling required

## Performance Optimization

### Metadata Caching Strategy

1. **Installation Time**: Detect structure once, save to metadata
2. **Runtime**: Read metadata file (single I/O operation)
3. **Fallback**: Detect structure if metadata missing, update for future

### Expected Performance

- Installation: One-time structure detection (~10ms)
- Shim execution: Metadata read (~1ms)
- Fallback detection: Directory check (~5ms)

### Fallback Performance Impact

**When Fallback Occurs**:

- First call to `resolve_java_home()`: ~5ms (directory existence check)
- Subsequent calls in same process: ~0ms (cached in memory)
- Different process: ~5ms again (no cross-process cache)

**Optimization Strategy**:

- Cache detection result in `InstalledJdk` struct during process lifetime
- Consider updating metadata file after successful fallback detection (Phase 2)
- For shim (short-lived process), 5ms overhead is acceptable
- For long-running processes, caching eliminates repeated checks

## Testing Strategy

### Unit Tests

**Structure Detection**:

- Test bundle structure detection
- Test direct structure detection
- Test hybrid structure with symlinks
- Test nested bundle structures
- Test invalid structures

**Metadata Handling**:

- Test metadata creation with structure info
- Test metadata reading and parsing
- Test fallback when metadata missing

### Integration Tests

**Installation Tests**:

- Install Temurin (bundle structure)
- Install Liberica (direct structure)
- Install Azul Zulu (hybrid structure)
- Verify correct JAVA_HOME for each

**Shim Tests**:

- Execute Java with different structures
- Verify environment variables
- Test version switching between structures

### Platform Tests

**macOS Specific**:

- Test on Intel (x86_64)
- Test on Apple Silicon (aarch64)
- Verify code signing preservation
- Test Gatekeeper compatibility

## Error Handling

### Error Scenarios

Uses existing Kopi error types from `src/error/mod.rs` and exit codes from `src/error/exit_codes.rs`:

1. **Invalid JDK Structure** (`ValidationError`, Exit code: 2)

   ```
   Error: Validation error: Invalid JDK structure - unable to locate bin/java

   The extracted archive does not appear to be a valid JDK.
   Please verify the download source and try again.
   ```

2. **Extraction Failure** (`Extract`, Exit code: 1)

   ```
   Error: Failed to extract archive: Unable to determine JDK root directory

   The archive structure is not recognized. This may indicate a corrupted
   or unsupported JDK distribution.
   ```

3. **Metadata Corruption** (`SystemError`, Exit code: 1)
   - Log warning but continue with runtime detection
   - Attempt to regenerate metadata after successful detection
   - If regeneration fails, continue without metadata caching

4. **Permission Issues** (`PermissionDenied`, Exit code: 13)

   ```
   Error: Permission denied: Cannot access JDK directory at /path/to/jdk

   Check file permissions and ensure Kopi has read access to the JDK installation.
   ```

5. **Invalid Metadata Format** (`InvalidMetadata`, Exit code: 1)

   ```
   Error: Invalid metadata format

   The JDK metadata file is corrupted or incompatible. Kopi will attempt
   to regenerate it on the next operation.
   ```

## Security Considerations

1. **Symlink Validation**: Ensure symlinks point within JDK directory
2. **Path Traversal**: Validate all paths remain within kopi directory
3. **Metadata Integrity**: Validate JSON structure before parsing

### macOS Code Signing and Notarization

**Verification Strategy**: Kopi will NOT verify code signatures or notarization during installation.

**Rationale**:

- macOS Gatekeeper automatically verifies signatures at runtime
- Verification during extraction would be redundant and complex
- Industry standard practice is to preserve, not verify, signatures

**Preservation Strategy**:

- Maintain original directory structure (Contents/Home)
- Preserve Extended Attributes (xattr) during extraction
- Retain all symbolic links and bundle components
- Keep Info.plist and \_CodeSignature directories intact

This approach ensures:

- Gatekeeper can verify signatures when Java is executed
- No false positives from premature verification
- Simplified implementation with fewer failure points
- Consistency with other JDK management tools

## Migration Path

### Existing Installations

1. **Graceful Upgrade**:
   - Existing JDKs without metadata continue to work
   - Structure detection happens at runtime when metadata is missing
   - No action required from users

2. **Lazy Metadata Generation**:
   - Metadata is NOT automatically generated for existing installations
   - Avoids potentially disruptive filesystem scanning on upgrade
   - Metadata will be created only for new installations

3. **No Breaking Changes**:
   - All existing JDKs remain functional
   - Runtime detection ensures compatibility
   - Performance impact is minimal (~5ms per execution)

### Version Compatibility

- Kopi versions without this feature continue to work (may fail on some JDKs)
- New versions handle both old and new installation formats
- Metadata format includes version field for future changes

## Success Criteria

1. **Functionality**: All major macOS JDK distributions install and run correctly
2. **Performance**: Shim execution time remains under 50ms
3. **Compatibility**: No regression for existing installations
4. **Transparency**: Users unaware of underlying structure differences
5. **Code Quality**: Single implementation of path resolution logic shared by all components
6. **Consistency**: Shim and env command always resolve to the same paths for a given JDK

## Alternative Approaches Considered

### 1. Normalize All Structures

**Approach**: Convert all JDKs to direct structure
**Rejected Because**:

- Breaks code signing
- Complex transformation logic
- Loses vendor-specific optimizations

### 2. Vendor-Specific Handlers

**Approach**: Custom code for each distribution
**Rejected Because**:

- High maintenance burden
- Fragile to distribution changes
- Doesn't handle custom builds

### 3. User Configuration

**Approach**: Require users to specify structure type
**Rejected Because**:

- Poor user experience
- Error prone
- Against Kopi's philosophy of transparency

## Monitoring and Metrics

Post-deployment monitoring:

1. Track structure detection success rate
2. Measure performance impact on shim execution
3. Monitor error rates by distribution
4. Collect user feedback on macOS functionality

## Documentation Updates

Required documentation changes:

1. Update installation guide with macOS notes
2. Add troubleshooting section for structure issues
3. Document metadata file format
4. Update developer guide with structure detection logic

## Related Work

- ADR-018: macOS JDK Bundle Structure Handling
- ADR-003: JDK Storage Format
- asdf-java: Reference implementation for structure handling
- jabba: Alternative approach using normalization
- GitHub Actions setup-java: Simple detection approach

## Design Decisions

Based on project requirements, the following decisions have been made:

1. **Custom JDK Support**: Non-standard custom JDK builds will NOT be supported. Only recognized structure patterns (bundle, direct, hybrid) will be handled. Custom builds with non-standard layouts will result in a validation error.

2. **Corrupted Structures**: Bundle structures that don't match expected patterns will be treated as errors (`ValidationError` with exit code 2). No attempt will be made to repair or work around corrupted extractions.

3. **User Interface**:
   - Structure type will NOT be displayed in `kopi list` output to maintain interface simplicity
   - Structure type WILL be logged at INFO level during installation for debugging purposes:
     ```
     INFO: Detected macOS bundle structure (Contents/Home) for temurin@21.0.2
     INFO: Detected direct structure for liberica@24.0.2
     INFO: Detected hybrid structure with symlinks for zulu@24.32.13
     ```

## Conclusion

This implementation provides robust handling of macOS JDK structures while maintaining simplicity and performance. By preserving original structures and leveraging metadata caching, we ensure compatibility with the macOS ecosystem while delivering a transparent user experience across all platforms.
