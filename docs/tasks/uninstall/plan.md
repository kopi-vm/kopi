# Uninstall Command Implementation Plan

## Overview
This document outlines the phased implementation plan for the `kopi uninstall` command, which is responsible for removing installed JDK distributions and cleaning up associated resources.

## Command Syntax
- `kopi uninstall <version>` - Uninstall JDK with specified version (searches all distributions)
- `kopi uninstall <distribution>@<version>` - Uninstall specific distribution and version
- `kopi uninstall --all` - Remove all installed JDKs

**Note**: When multiple JDKs match the version (different distributions), the command will prompt for selection unless a specific distribution is provided.

## Phase 1: JDK Discovery and Validation

### Input Resources
- `/src/storage/mod.rs` - Storage management patterns
- `/src/models/jdk.rs` - JDK metadata structures
- `/docs/reference.md` - Storage location specifications

### Deliverables
1. **JDK Scanner Module** (`/src/jdk_scanner/mod.rs`)
   - Scan `~/.kopi/jdks/` directory structure
   - Parse installed JDK metadata from directory names
   - Validate JDK installation integrity
   - Version matching logic
   - Distribution identification

2. **Installed JDK Model** (`/src/models/installed_jdk.rs`)
   - Installed JDK metadata structure
   - Installation path tracking
   - Version and distribution parsing from directory names
   - Size calculation for installed JDKs

3. **Unit Tests** (use mocks extensively)
   - `src/jdk_scanner/mod.rs` - Directory scanning tests (mock file system)
   - `src/models/installed_jdk.rs` - Model parsing and validation tests

4. **Integration Tests** (`/tests/jdk_scanner_integration.rs`) (no mocks)
   - Real directory scanning with test JDK installations
   - Version matching accuracy
   - Multiple distribution handling

### Success Criteria
- Accurately identify all installed JDKs
- Parse version and distribution from directory names
- Handle malformed directory names gracefully
- Support fuzzy version matching (e.g., "21" matches "21.0.1")

## Phase 2: Usage Detection and Safety Checks

### Input Resources
- Phase 1 deliverables (JDK scanner)
- `/src/commands/current.rs` - Current JDK detection patterns
- Platform-specific process handling documentation

### Deliverables
1. **Usage Detector Module** (`/src/usage_detector/mod.rs`)
   - Check if JDK is currently active (symlinked)
   - Detect running Java processes
   - Check environment variables (JAVA_HOME, PATH)
   - Project-specific usage detection (.kopi-version, .java-version)
   - IDE and build tool integration checks

2. **Safety Validator** (`/src/safety/uninstall.rs`)
   - Validate removal is safe
   - Check for dependent projects
   - Verify no active processes using JDK
   - Backup important configuration files

3. **Process Utils** (`/src/utils/process.rs`)
   - Platform-specific process enumeration
   - Java process detection
   - Process command line parsing

4. **Unit Tests** (use mocks extensively)
   - `src/usage_detector/mod.rs` - Usage detection tests (mock processes and files)
   - `src/safety/uninstall.rs` - Safety validation tests (mock usage scenarios)
   - `src/utils/process.rs` - Process detection tests (mock process lists)

5. **Integration Tests** (`/tests/usage_detection_integration.rs`) (no mocks)
   - Real process detection with running Java applications
   - Environment variable checks
   - Symlink validation

### Success Criteria
- Detect active JDK usage accurately
- Prevent unsafe removals
- Clear warnings for potentially breaking changes
- Support force removal with appropriate warnings

## Phase 3: Removal Operations and Cleanup

### Input Resources
- Phase 1 & 2 deliverables
- `/src/storage/mod.rs` - Storage patterns for cleanup
- Platform-specific file removal considerations

### Deliverables
1. **Removal Module** (`/src/removal/mod.rs`)
   - Safe directory removal with verification
   - Symlink cleanup in `~/.kopi/bin/`
   - Atomic removal (move to temp, then delete)
   - Rollback capability on failure
   - Progress reporting for large removals

2. **Symlink Manager** (`/src/symlink/mod.rs`)
   - Update or remove symlinks in bin directory
   - Handle broken symlinks gracefully
   - Platform-specific symlink handling
   - Recreate symlinks for remaining JDKs

3. **Cleanup Handler** (`/src/cleanup/mod.rs`)
   - Remove empty directories
   - Clean orphaned configuration files
   - Update global/local configuration
   - Cache invalidation for removed JDKs

4. **Unit Tests** (use mocks extensively)
   - `src/removal/mod.rs` - Directory removal tests (mock file system)
   - `src/symlink/mod.rs` - Symlink management tests (mock symlink operations)
   - `src/cleanup/mod.rs` - Cleanup operation tests (mock file operations)

5. **Integration Tests** (`/tests/removal_integration.rs`) (no mocks)
   - End-to-end removal testing with real directories
   - Symlink update verification
   - Cleanup validation
   - Failure recovery scenarios

### Success Criteria
- Complete removal of JDK installations
- Proper symlink management
- Clean removal with no orphaned files
- Atomic operations preventing partial states
- Efficient handling of large JDK installations (>1GB)

## Phase 4: Command Implementation and CLI Integration

### Input Resources
- Phase 1-3 deliverables
- `/src/main.rs` - Existing CLI structure with clap
- `/src/commands/` - Command patterns from other commands

### Deliverables
1. **Uninstall Command** (`/src/commands/uninstall.rs`)
   - Command argument parsing
   - Interactive selection for ambiguous versions
   - Confirmation prompts for safety
   - `--force` flag to skip safety checks
   - `--yes` flag to skip confirmations
   - `--all` flag to remove all JDKs
   - Dry-run support

2. **Interactive Selector** (`/src/ui/selector.rs`)
   - Terminal-based selection UI
   - Display JDK details (version, distribution, size, path)
   - Multi-select support for bulk operations
   - Keyboard navigation

3. **CLI Integration** (update `/src/main.rs`)
   - Add uninstall subcommand with clap derive
   - Command-line options:
     - `--force`: Skip safety checks
     - `--yes`: Skip confirmation prompts
     - `--all`: Remove all installed JDKs
     - `--dry-run`: Show what would be removed
   - Help text and examples

4. **Unit Tests** (use mocks extensively)
   - `src/commands/uninstall.rs` - Command logic tests (mock scanner and removal)
   - `src/ui/selector.rs` - UI interaction tests (mock terminal input)
   - CLI argument parsing tests

5. **Integration Tests** (`/tests/uninstall_command_integration.rs`) (no mocks)
   - Full command execution testing
   - Interactive selection testing
   - Multiple removal scenarios
   - Error handling validation

### Success Criteria
- `kopi uninstall 21` removes JDK 21 (with confirmation)
- `kopi uninstall corretto@17` removes specific distribution
- `kopi uninstall --all` removes all JDKs after confirmation
- Clear interactive selection when multiple matches
- Informative dry-run output

## Phase 5: Post-Removal Operations and Validation

### Input Resources
- Phase 1-4 deliverables
- Configuration management patterns
- Global state management

### Deliverables
1. **Post-Removal Handler** (`/src/post_removal/mod.rs`)
   - Update default JDK if removed
   - Relink symlinks to next available JDK
   - Update project configurations
   - Notify user of configuration changes

2. **Configuration Updater** (`/src/config/updater.rs`)
   - Update global config after removal
   - Clean project-specific configurations
   - Handle cascading updates
   - Validation of remaining configuration

3. **Validation Module** (`/src/validation/post_removal.rs`)
   - Verify complete removal
   - Check for orphaned resources
   - Validate symlink integrity
   - Report any inconsistencies

4. **Unit Tests** (use mocks extensively)
   - `src/post_removal/mod.rs` - Post-removal logic tests (mock config and symlinks)
   - `src/config/updater.rs` - Configuration update tests (mock config files)
   - `src/validation/post_removal.rs` - Validation tests (mock file system state)

5. **Integration Tests** (`/tests/post_removal_integration.rs`) (no mocks)
   - Complete uninstall workflow validation
   - Configuration update verification
   - Symlink integrity after removal
   - Multi-JDK scenarios

6. **Documentation Updates**
   - Update `/docs/reference.md` with uninstall command details
   - Add troubleshooting for common issues
   - Document recovery procedures

### Success Criteria
- Graceful handling of default JDK removal
- Automatic symlink updates
- Clean configuration state
- No orphaned resources
- Clear user communication

## Implementation Guidelines

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