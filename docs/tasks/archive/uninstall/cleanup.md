# Cleanup Option Implementation Plan

## Overview

The `--cleanup` option for the `kopi uninstall` command detects and cleans up failed or partial uninstall operations. This option is essential for maintaining system integrity after interrupted or failed JDK removal operations.

## Command Interface

### Usage
```bash
kopi uninstall --cleanup                 # Clean up failed operations with confirmation
kopi uninstall --cleanup --force         # Force cleanup without confirmation
kopi uninstall --cleanup --dry-run       # Preview cleanup actions
kopi uninstall <version> --cleanup       # Uninstall specified version then perform cleanup
```

### Behavior
- When used alone (`kopi uninstall --cleanup`), performs only cleanup operations
- When used with a version (`kopi uninstall temurin@21 --cleanup`), performs the uninstall first, then cleanup
- Cleanup runs after the normal uninstall operation completes successfully

## Core Functionality

### Detection Capabilities

1. **Temporary Removal Directories**
   - Scan for `.*.removing` directories in `~/.kopi/jdks/`
   - These are created during uninstall operations and should be cleaned up

2. **Partially Removed JDKs**
   - Identify JDK installations missing essential files (e.g., `bin/java`)
   - Check for incomplete directory structures
   - Validate metadata integrity

3. **Orphaned Metadata Files**
   - Find `.meta.json` files without corresponding JDK directories
   - Clean up stale metadata entries

4. **Symlink Cleanup (Unix Systems)**
   - Detect orphaned symbolic links
   - Remove broken symlinks that point to non-existent JDK installations

### Cleanup Actions

#### 1. Cleanup Temp Directories
- **Target**: `.*.removing` directories from failed operations
- **Action**: Remove these temporary directories completely
- **Safety**: Verify they are indeed temporary directories before removal

#### 2. Complete Partial Removals
- **Target**: JDKs missing essential files (incomplete uninstall)
- **Action**: Remove remaining files and directories
- **Safety**: Confirm with user unless `--force` is specified

#### 3. Orphaned Metadata Cleanup
- **Target**: `.meta.json` files without corresponding JDK directories
- **Action**: Remove orphaned metadata files
- **Safety**: Verify the JDK directory truly doesn't exist

#### 4. Symlink Cleanup (Unix only)
- **Target**: Broken symbolic links in kopi directories
- **Action**: Remove orphaned symlinks
- **Safety**: Only remove symlinks that point to non-existent targets

## Implementation Structure

The cleanup functionality is integrated into the uninstall command:

### Module Organization
```
src/commands/uninstall.rs            # Main command implementation with --cleanup handling
src/uninstall/
├── cleanup.rs                       # Cleanup operations (UninstallCleanup struct)
├── handler.rs                       # UninstallHandler with recover_from_failures method
└── platform integration             # Uses src/platform/ modules
```

### Core Components

#### 1. Cleanup Implementation (`src/uninstall/cleanup.rs`)
```rust
pub struct UninstallCleanup<'a> {
    repository: &'a JdkRepository<'a>,
}

impl<'a> UninstallCleanup<'a> {
    pub fn detect_and_cleanup_partial_removals(&self) -> Result<Vec<CleanupAction>>
    pub fn execute_cleanup(&self, actions: Vec<CleanupAction>, force: bool) -> Result<CleanupResult>
    pub fn suggest_cleanup_actions(&self, error: &KopiError) -> Vec<String>
    pub fn force_cleanup_jdk(&self, jdk_path: &Path) -> Result<()>
}
```

#### 2. Cleanup Action Types
```rust
#[derive(Debug)]
pub enum CleanupAction {
    CleanupTempDir(PathBuf),
    CompleteRemoval(PathBuf),
    CleanupOrphanedMetadata(PathBuf),
}
```

#### 3. Cleanup Result
```rust
#[derive(Debug, Default)]
pub struct CleanupResult {
    pub successes: Vec<String>,
    pub failures: Vec<String>,
}
```

## User Experience

### Output Format

#### Detection Phase
```
Scanning for cleanup issues...
Found 3 issues:
  • Temporary directory: ~/.kopi/jdks/.temurin-21.0.5+11.removing (245 MB)
  • Partial removal: corretto-17.0.8.8.1 (missing bin/java, lib/modules)
  • Orphaned metadata: ~/.kopi/jdks/zulu-11.0.20/.meta.json

Total recoverable space: 245 MB
```

#### Confirmation (without --force)
```
The following cleanup actions will be performed:
  ✓ Remove temporary directory: .temurin-21.0.5+11.removing
  ✓ Complete removal of: corretto-17.0.8.8.1
  ✓ Clean orphaned metadata: zulu-11.0.20/.meta.json

Proceed with cleanup? [y/N]
```

#### Cleanup Progress
```
Cleaning up...
  ✓ Cleaned temporary directory: .temurin-21.0.5+11.removing
  ✓ Completed removal: corretto-17.0.8.8.1
  ✓ Removed orphaned metadata: zulu-11.0.20/.meta.json

Cleanup completed successfully. Freed 245 MB of disk space.
```

### Error Handling

#### No Issues Found
```
No cleanup issues detected. Your kopi installation is clean.
```

#### Permission Errors
```
Error: Permission denied cleaning up corretto-17.0.8.8.1
Suggestion: Run 'kopi uninstall --cleanup --force' as administrator or with sudo
```

#### Partial Cleanup
```
Warning: Some issues could not be resolved:
  • corretto-17.0.8.8.1: Permission denied
  • Use 'kopi uninstall --cleanup --force' or manually remove with elevated permissions
```

## Platform-Specific Considerations

### Windows
- Handle file locking by antivirus software
- Use Windows-specific file deletion APIs for stubborn files
- Handle long path names properly

### Unix/Linux/macOS
- Use symlink-aware file operations
- Handle permission issues with appropriate error messages
- Leverage platform-specific file system features

## Safety Features

### Pre-cleanup Validation
1. Verify paths are within kopi home directory
2. Confirm temporary directories match expected patterns
3. Validate metadata files are actually orphaned
4. Check symlinks are truly broken

### Atomic Operations
- Use temporary markers during cleanup
- Rollback capability for critical failures
- Comprehensive logging of all actions

### User Confirmation
- Clear description of what will be removed
- Disk space that will be freed
- Option to skip individual items
- Force flag to bypass confirmations

## Integration with Normal Uninstall

When used with a version specification:
1. Execute the normal uninstall operation first
2. If successful, proceed with cleanup operations
3. Report both uninstall and cleanup results

This allows users to clean up their kopi installation after any uninstall operation:
```bash
# Uninstall a specific JDK and clean up any other failed operations
kopi uninstall temurin@21 --cleanup

# This will:
# 1. Uninstall temurin@21 (with confirmation unless --force)
# 2. Scan for and clean up any failed/partial removals from previous operations
```

## Testing Strategy

### Unit Tests
- Test detection logic with mocked file systems
- Validate cleanup operations in isolated environments
- Test platform-specific behavior

### Integration Tests
- Create realistic cleanup scenarios
- Test with actual partial uninstall states
- Verify safety mechanisms work correctly
- Test combined uninstall + cleanup operations

### Edge Cases
- Empty kopi home directory
- Corrupted metadata files
- Permission denied scenarios
- Network file systems
- Cleanup after successful uninstall