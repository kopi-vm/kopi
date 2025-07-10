# Uninstall Command Implementation Plan

## Overview
This document outlines the phased implementation plan for the `kopi uninstall` command, which is responsible for removing installed JDK distributions and cleaning up associated resources.

**Current Status**: ~20% implemented. Basic building blocks exist (JDK listing, simple removal, version detection) but most safety features and user interaction are not yet implemented.

## Command Syntax
- `kopi uninstall <version>` - Uninstall JDK with specified version (searches all distributions)
- `kopi uninstall <distribution>@<version>` - Uninstall specific distribution and version
- `kopi uninstall --all` - Remove all installed JDKs

**Note**: When multiple JDKs match the version (different distributions), the command will prompt for selection unless a specific distribution is provided.

## Phase 1: JDK Discovery and Validation ✅ (Partially Complete)

### Input Resources
- `/src/storage/listing.rs` - **Existing** JDK listing functionality
- `/src/storage/mod.rs` - Storage management patterns
- `/src/models/jdk.rs` - JDK metadata structures
- `/src/models/version.rs` - **Existing** version representation and matching
- `/src/models/parser.rs` - **Existing** version parsing logic
- `/docs/reference.md` - Storage location specifications

**Note**: Version-related functionality is located in `/src/models/` not `/src/version/` (which doesn't exist)

### Existing Components
- ✅ **JdkLister** (`/src/storage/listing.rs`) - Already implements:
  - Scanning `~/.kopi/jdks/` directory
  - Parsing distribution and version from directory names
  - Size calculation for installed JDKs
  - `InstalledJdk` struct with basic metadata

### Deliverables
1. **Enhance JdkLister** (`/src/storage/listing.rs`)
   - ❌ Add fuzzy version matching logic (e.g., "21" matches "21.0.1")
   - ❌ Add validation of JDK installation integrity
   - ❌ Improve error handling for malformed directory names
   - ❌ Add caching for repeated scans

2. **Enhance Version Matching** (`/src/models/version.rs`)
   - ❌ Enhance existing version matching with fuzzy matching for uninstall
   - ❌ Add methods to find all matching installed versions
   - ❌ Support wildcard patterns for bulk operations

3. **Unit Tests** (use mocks extensively)
   - ✅ Basic tests exist in `src/storage/listing.rs`
   - ❌ Add comprehensive mock-based tests
   - ❌ Test edge cases and malformed inputs

4. **Integration Tests** (`/tests/jdk_listing_integration.rs`) (no mocks)
   - ❌ Real directory scanning with test JDK installations
   - ❌ Version matching accuracy
   - ❌ Multiple distribution handling

### Success Criteria
- ✅ Accurately identify all installed JDKs
- ✅ Parse version and distribution from directory names
- ❌ Handle malformed directory names gracefully
- ❌ Support fuzzy version matching (e.g., "21" matches "21.0.1")

## Phase 2: Usage Detection and Safety Checks ❌ (Not Started)

### Input Resources
- Phase 1 deliverables (Enhanced JDK listing)
- `/src/shim/version_resolver.rs` - **Existing** version detection logic
- `/src/commands/current.rs` - Current JDK detection patterns
- Platform-specific process handling documentation

### Existing Components
- ✅ **VersionResolver** (`/src/shim/version_resolver.rs`) - Already detects:
  - `.kopi-version` and `.java-version` files
  - `KOPI_JAVA_VERSION` environment variable
  - Global default from `~/.kopi/default-version`

### Deliverables
1. **Usage Detector Module** (`/src/usage_detector/mod.rs`)
   - ❌ Check if JDK is currently active (symlinked)
   - ❌ Detect running Java processes
   - ❌ Check environment variables (JAVA_HOME, PATH)
   - ❌ Integrate existing VersionResolver for project detection
   - ❌ IDE and build tool integration checks

2. **Safety Validator** (`/src/safety/uninstall.rs`)
   - ❌ Validate removal is safe
   - ❌ Check for dependent projects
   - ❌ Verify no active processes using JDK
   - ❌ Create safety report before removal

3. **Process Detection** (`/src/platform/process.rs`)
   - ❌ Platform-specific process enumeration (Windows/Unix/macOS)
   - ❌ Java process detection
   - ❌ Process command line parsing
   - ❌ JDK path matching in process arguments

4. **Unit Tests** (use mocks extensively)
   - ❌ `src/usage_detector/mod.rs` - Usage detection tests (mock processes and files)
   - ❌ `src/safety/uninstall.rs` - Safety validation tests (mock usage scenarios)
   - ❌ `src/platform/process.rs` - Process detection tests (mock process lists)

5. **Integration Tests** (`/tests/usage_detection_integration.rs`) (no mocks)
   - ❌ Real process detection with running Java applications
   - ❌ Environment variable checks
   - ❌ Symlink validation

### Success Criteria
- ❌ Detect active JDK usage accurately
- ❌ Prevent unsafe removals
- ❌ Clear warnings for potentially breaking changes
- ❌ Support force removal with appropriate warnings

## Phase 3: Removal Operations and Cleanup 🔶 (Basic Implementation Exists)

### Input Resources
- Phase 1 & 2 deliverables
- `/src/storage/repository.rs` - **Existing** basic removal functionality
- `/src/platform/symlink.rs` - **Existing** basic symlink operations
- Platform-specific file removal considerations (handle in `/src/platform/` if needed)

### Existing Components
- ✅ **JdkRepository** (`/src/storage/repository.rs`) - Already implements:
  - `remove_jdk()` method with security validation
  - Basic directory removal (but not atomic)
- ✅ **Symlink utilities** (`/src/platform/symlink.rs`) - Basic operations:
  - `create_symlink()`, `verify_symlink()`, `is_symlink()`

### Deliverables
1. **Enhanced JdkRepository** (`/src/storage/repository.rs`)
   - ❌ Enhance existing `remove_jdk()` with atomic removal (move to temp, then delete)
   - ❌ Add rollback capability on failure
   - ❌ Add progress reporting for large removals
   - ❌ Integrate with safety checks from Phase 2
   - ❌ Add cleanup operations (empty directories, orphaned files)
   - ❌ Cache invalidation for removed JDKs
   - Note: Platform-specific file locking/removal issues should be handled via `/src/platform/` modules

2. **Symlink Manager** (`/src/storage/symlink.rs`)
   - ❌ List all symlinks in `~/.kopi/bin/`
   - ❌ Update or remove symlinks after JDK removal
   - ❌ Handle broken symlinks gracefully
   - ❌ Recreate symlinks for remaining JDKs
   - ❌ Batch symlink operations for efficiency

3. **Unit Tests** (use mocks extensively)
   - ✅ Basic tests exist for `remove_jdk()`
   - ❌ Enhanced removal tests with mocked file operations
   - ❌ Symlink management tests with mocked operations
   - ❌ Cleanup operation tests with mocked file system

4. **Integration Tests** (`/tests/removal_integration.rs`) (no mocks)
   - ❌ End-to-end removal testing with real directories
   - ❌ Symlink update verification
   - ❌ Cleanup validation
   - ❌ Failure recovery scenarios

### Success Criteria
- ✅ Basic removal of JDK installations
- ❌ Atomic operations preventing partial states
- ❌ Proper symlink management
- ❌ Clean removal with no orphaned files
- ❌ Efficient handling of large JDK installations (>1GB)

## Phase 4: Command Implementation and CLI Integration ❌ (Not Started)

### Input Resources
- Phase 1-3 deliverables
- `/src/main.rs` - Existing CLI structure with clap
- `/src/commands/` - Command patterns from other commands (install, list, etc.)

### Deliverables
1. **Uninstall Command** (`/src/commands/uninstall.rs`)
   - ❌ Command argument parsing
   - ❌ Interactive selection for ambiguous versions
   - ❌ Confirmation prompts for safety
   - ❌ `--force` flag to skip safety checks
   - ❌ `--yes` flag to skip confirmations
   - ❌ `--all` flag to remove all JDKs
   - ❌ Dry-run support

2. **Interactive Selector** (`/src/ui/selector.rs`)
   - ❌ Terminal-based selection UI
   - ❌ Display JDK details (version, distribution, size, path)
   - ❌ Multi-select support for bulk operations
   - ❌ Keyboard navigation

3. **CLI Integration** (update `/src/main.rs`)
   - ❌ Add uninstall subcommand with clap derive
   - ❌ Command-line options:
     - `--force`: Skip safety checks
     - `--yes`: Skip confirmation prompts
     - `--all`: Remove all installed JDKs
     - `--dry-run`: Show what would be removed
   - ❌ Help text and examples

4. **Unit Tests** (use mocks extensively)
   - ❌ `src/commands/uninstall.rs` - Command logic tests (mock scanner and removal)
   - ❌ `src/ui/selector.rs` - UI interaction tests (mock terminal input)
   - ❌ CLI argument parsing tests

5. **Integration Tests** (`/tests/uninstall_command_integration.rs`) (no mocks)
   - ❌ Full command execution testing
   - ❌ Interactive selection testing
   - ❌ Multiple removal scenarios
   - ❌ Error handling validation

### Success Criteria
- ❌ `kopi uninstall 21` removes JDK 21 (with confirmation)
- ❌ `kopi uninstall corretto@17` removes specific distribution
- ❌ `kopi uninstall --all` removes all JDKs after confirmation
- ❌ Clear interactive selection when multiple matches
- ❌ Informative dry-run output

## Phase 5: Post-Removal Operations and Validation ❌ (Not Started)

### Input Resources
- Phase 1-4 deliverables
- Configuration management patterns
- Global state management

### Module Organization
Post-removal functionality will be integrated directly into the uninstall command rather than creating separate modules. This keeps the architecture simple and cohesive.

### Deliverables
1. **Enhanced Uninstall Command** (`/src/commands/uninstall.rs`)
   - ❌ Add post-removal operations:
     - Update default JDK if removed
     - Relink symlinks to next available JDK
     - Update project configurations
     - Notify user of configuration changes
   - ❌ Add validation checks:
     - Verify complete removal
     - Check for orphaned resources
     - Validate symlink integrity
     - Report any inconsistencies

2. **Configuration Updates** (enhance existing `/src/config.rs`)
   - ❌ Add methods to update config after JDK removal
   - ❌ Clean project-specific configurations
   - ❌ Handle cascading updates
   - ❌ Validation of remaining configuration

3. **Unit Tests** (use mocks extensively)
   - ❌ Post-removal logic tests in `src/commands/uninstall.rs` tests
   - ❌ Configuration update tests in `src/config.rs` tests
   - ❌ Validation tests with mocked file system state

4. **Integration Tests** (`/tests/uninstall_integration.rs`) (no mocks)
   - ❌ Complete uninstall workflow validation
   - ❌ Configuration update verification
   - ❌ Symlink integrity after removal
   - ❌ Multi-JDK scenarios

5. **Documentation Updates**
   - ❌ Update `/docs/reference.md` with uninstall command details
   - ❌ Add troubleshooting for common issues
   - ❌ Document recovery procedures

### Success Criteria
- ❌ Graceful handling of default JDK removal
- ❌ Automatic symlink updates
- ❌ Clean configuration state
- ❌ No orphaned resources
- ❌ Clear user communication

## Implementation Summary

### Overall Progress: ~20% Complete

**Existing Components:**
- ✅ Basic JDK listing and directory scanning (`JdkLister`)
- ✅ Simple JDK removal with security checks (`JdkRepository::remove_jdk()`)
- ✅ Version file detection (`.kopi-version`, `.java-version`)
- ✅ Basic symlink operations

**Major Missing Components:**
- ❌ **No uninstall command** - The CLI command itself doesn't exist
- ❌ **No safety checks** - Cannot detect running Java processes or active usage
- ❌ **No user interaction** - No confirmation prompts or interactive selection
- ❌ **No atomic operations** - Removal could leave system in partial state
- ❌ **No symlink management** - Symlinks not updated after removal
- ❌ **No post-removal cleanup** - Configuration and defaults not updated

### Simplified Architecture

The plan has been simplified to avoid unnecessary module proliferation:
- **Removal operations** → Enhanced in existing `JdkRepository` rather than creating `/src/removal/`
- **Cleanup operations** → Integrated into `JdkRepository::remove_jdk()` rather than separate `/src/cleanup/`
- **Post-removal operations** → Part of the uninstall command logic rather than separate `/src/post_removal/`
- **Symlink management** → Consolidated in `/src/storage/symlink.rs` rather than scattered modules

This approach:
- Keeps related functionality together
- Reduces complexity and module boundaries
- Leverages existing code structure
- Makes the codebase easier to understand and maintain

### Recommended Implementation Order

1. **Start with Phase 4** - Create the basic `uninstall` command structure first
   - This provides immediate user value and a framework to build on
   - Can start with simple removal using existing `JdkRepository::remove_jdk()`

2. **Then Phase 2** - Add safety checks to prevent dangerous removals
   - Critical for preventing users from breaking their environment
   - Process detection is the most important safety feature

3. **Then Phase 3** - Enhance removal operations
   - Add atomic operations and rollback
   - Implement proper symlink management

4. **Then Phase 1** - Enhance JDK discovery
   - Add fuzzy version matching
   - Improve error handling

5. **Finally Phase 5** - Post-removal operations
   - Configuration updates
   - Validation and cleanup

### Critical Path Items

The following items are blocking and must be implemented for a minimal viable uninstall command:

1. **Command structure** (`src/commands/uninstall.rs`) - Without this, users can't run the command
2. **Basic safety check** - At minimum, check if JDK is currently in use
3. **Confirmation prompt** - Prevent accidental deletion
4. **Symlink cleanup** - System will be broken if symlinks point to deleted JDKs

## Implementation Guidelines

### Platform-Specific Code Organization
All platform-dependent functionality must be placed under `/src/platform/`:
- Process detection and enumeration → `/src/platform/process.rs`
- Symlink operations → `/src/platform/symlink.rs` (already exists)
- Any OS-specific file operations → Create appropriate modules under `/src/platform/`
- Use conditional compilation (`#[cfg(target_os = "...")]`) for OS-specific implementations

### For Each Phase:
1. Start with `/clear` command to reset context
2. Load this plan.md and relevant phase resources
3. Implement deliverables incrementally
4. Run quality checks after each module:
   - `cargo fmt`
   - `cargo test`
   - `cargo clippy`
   - `cargo check`
5. Commit completed phase before proceeding

### Testing Strategy

#### Unit Tests (use mocks extensively)
- Test individual module functionality in isolation
- Mock file system operations, process lists, and user input
- Focus on edge cases and error conditions
- Example:
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      use mockall::*;
      
      #[test]
      fn test_safe_removal_with_mock_fs() {
          let mut mock_fs = MockFileSystem::new();
          mock_fs.expect_exists()
              .returning(|_| true);
          mock_fs.expect_remove_dir_all()
              .returning(|_| Ok(()));
          // Test removal logic
      }
  }
  ```

#### Integration Tests (no mocks)
- Test complete uninstall workflows
- Verify actual file system changes
- Test with real JDK installations
- Validate platform-specific behavior
- Example:
  ```rust
  #[test]
  fn test_complete_uninstall_workflow() {
      // Setup: Install a test JDK
      let test_jdk = setup_test_jdk("temurin", "21");
      
      // Execute uninstall
      let result = run_uninstall_command(&["21", "--yes"]);
      assert!(result.is_ok());
      
      // Verify removal
      assert!(!test_jdk.path.exists());
      assert!(!symlink_exists("java"));
  }
  ```

### Error Handling Priorities
1. Active JDK in use - clear warning with process details
2. Permission errors - suggest elevated privileges
3. Missing JDK - helpful message with available versions
4. Disk errors - safe failure with rollback
5. Broken symlinks - automatic cleanup
6. Configuration errors - graceful degradation

### Safety Considerations
1. Always check for active usage before removal
2. Implement atomic operations to prevent partial states
3. Backup critical configurations before modification
4. Provide clear rollback instructions
5. Log all removal operations for audit

### User Experience Priorities
1. Clear, actionable error messages
2. Informative progress reporting
3. Safe defaults (require confirmation)
4. Helpful suggestions for common issues
5. Fast operation for good user experience

## Next Steps
Begin with Phase 1, focusing on accurate JDK discovery and establishing the foundation for safe removal operations.