# Platform Dependencies Review (2025-07-05)

This document catalogs all platform-dependent code found outside of `src/platform/` in the Kopi codebase. This review will guide future refactoring efforts to consolidate platform-specific logic into the platform module.

## Summary

Platform-dependent code exists in several modules outside of `src/platform/`. Most platform-specific functionality is properly abstracted through the platform module, but some direct platform conditionals remain for specific features.

## Platform-Dependent Code by Module

### 1. Cache Module (`src/cache/mod.rs`)

**Windows-specific atomic file rename handling (lines 78-86):**

```rust
#[cfg(windows)]
{
    // On Windows, rename fails if destination exists, so remove it first
    if path.exists() {
        fs::remove_file(path).map_err(|e| {
            KopiError::ConfigError(format!("Failed to remove old cache file: {e}"))
        })?;
    }
}
```

**Refactoring opportunity:** Move atomic rename logic to `platform::file_ops` module.

### 2. Security Module (`src/security/mod.rs`)

**Unix file permissions checking (lines 90-104):**

- Uses `#[cfg(unix)]` with `std::os::unix::fs::PermissionsExt`
- Checks file mode against 0o644

**Windows file permissions checking (lines 106-137):**

- Uses `#[cfg(windows)]` with Windows-specific imports
- Complex ACL-based permission checking

**Secure file permissions setting:**

- Unix (lines 171-177): Sets mode to 0o644
- Windows (lines 179-183): Sets read-only attribute

**Refactoring opportunity:** Create `platform::permissions` module with unified permission checking/setting API.

### 3. Install Command (`src/commands/install.rs`)

**Architecture detection (lines 27-58):**

- Multiple `#[cfg(target_arch = ...)]` blocks for:
  - x86_64, x86, aarch64, arm, powerpc64, s390x
- Each architecture maps to Foojay-specific string

**OS detection (lines 384-395):**

- `#[cfg(target_os = ...)]` for linux, windows, macos
- Returns platform-specific strings

**Refactoring opportunity:** Already uses some platform functions but architecture detection could be moved to platform module.

### 4. Archive Module (`src/archive/mod.rs`)

**Unix-specific zip extraction permissions (lines 179-185):**

```rust
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    if let Some(mode) = file.unix_mode() {
        fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
    }
}
```

**Refactoring opportunity:** Move permission preservation logic to `platform::file_ops`.

### 5. Shim Module (`src/shim/mod.rs`, `src/shim/installer.rs`)

**Platform-specific implementations:**

- Unix shim creation/verification methods with `#[cfg(unix)]`
- Windows shim creation/verification methods with `#[cfg(windows)]`
- Different executable detection and handling logic

**Already uses platform abstractions for:**

- `platform::executable_extension()`
- `platform::process::exec_replace()`
- `platform::symlink` operations

**Refactoring opportunity:** Minimal - shims are inherently platform-specific by design.

### 6. Test Files

Multiple test files contain platform-specific tests:

- `tests/install_scenarios.rs`: Unix-specific symlink tests
- `tests/shim_integration.rs`: Platform-specific shim behavior tests
- `tests/install_e2e.rs`: Unix-specific permission tests

**Note:** Test platform conditionals are appropriate and should remain.

## Platform-Specific Dependencies

### Cargo.toml Dependencies

**Unix-specific:**

```toml
[target.'cfg(unix)'.dependencies]
libc = "0.2"
```

**Windows-specific:**

```toml
[target.'cfg(windows)'.dependencies]
winreg = "0.53.0"
junction = "1.2.0"
winapi = { version = "0.3", features = ["fileapi"] }
```

**Good practice:** These dependencies are only used within the platform module.

## Recommendations for Refactoring

### High Priority

1. **Cache atomic rename operations** - Move to `platform::file_ops::atomic_rename()`
2. **Security permissions handling** - Create `platform::permissions` module with:
   - `check_file_permissions(path) -> Result<bool>`
   - `set_secure_permissions(path) -> Result<()>`

### Medium Priority

3. **Architecture detection in install command** - Move to `platform::detection::get_foojay_architecture()`
4. **Archive permission preservation** - Add to `platform::file_ops::preserve_permissions()`

### Low Priority

5. **Shim platform conditionals** - Already well-structured, minimal benefit from further abstraction

## Files Using Platform Module (Good Examples)

These files properly use platform abstractions:

- `src/search/mod.rs`
- `src/api/client.rs`
- `src/search/searcher.rs`
- `src/shim/platform.rs`
- `src/shim/tools.rs`

## Conclusion

The codebase shows good platform abstraction overall. The main opportunities for improvement are:

1. Consolidating file operation platform differences (atomic rename, permissions)
2. Moving architecture/OS detection fully into the platform module
3. Creating a unified permissions API for security operations

The platform-specific code in tests and inherently platform-specific features (like shims) should remain as-is.
