# Uninstall Command Implementation Plan

## Current Status (Updated)

### ✅ Completed Phases

- **Phase 1**: Core Uninstall Logic and Safety Checks - COMPLETE
- **Phase 2**: Exact Specification Enforcement and Batch Operations - COMPLETE

### 🚧 In Progress

- **Phase 3**: Command Implementation and CLI Integration - NOT STARTED
- **Phase 4**: Metadata Updates and Integration - PARTIALLY COMPLETE (metadata handled by JdkRepository)
- **Phase 5**: Platform-Specific Handling and Error Recovery - PARTIALLY COMPLETE (atomic operations implemented)

## Overview

This document outlines the phased implementation plan for the `kopi uninstall` command, which is responsible for removing installed JDK distributions from the local system and managing disk space efficiently.

## Command Syntax

- `kopi uninstall <version>` - Uninstall JDK with specified version
- `kopi uninstall <distribution>@<version>` - Uninstall specific distribution and version
- `kopi uninstall <distribution> --all` - Uninstall all versions of a distribution
- `kopi uninstall jre@<distribution>@<version>` - Uninstall JRE variant

## Phase 1: Core Uninstall Logic and Safety Checks ✅ COMPLETED

### Input Resources

- `/docs/tasks/archive/uninstall/design.md` - Uninstall command design specification
- `/src/models/metadata.rs` - JdkMetadata model
- `/src/models/distribution.rs` - Distribution enum and parsing
- `/src/models/version.rs` - Version and VersionRequest types
- `/src/storage/repository.rs` - JdkRepository for storage operations
- `/src/storage/listing.rs` - JdkLister for installed JDK discovery
- `/src/commands/` - Existing command structure

### Deliverables ✅ COMPLETED

1. **Uninstall Module** (`/src/uninstall/mod.rs`) ✅
   - JDK resolution using pattern matching ✅
   - Integration with JdkRepository for removal ✅
   - Atomic removal with rollback capability ✅
   - Progress reporting for large removals (>100MB) ✅
   - Disk space calculation using `JdkRepository::get_jdk_size()` ✅

2. **Safety Check Module** (`/src/uninstall/safety.rs`) ✅
   - Active JDK detection stub functions ✅:
     - `is_active_global_jdk()` - returns Ok(false) placeholder ✅
     - `is_active_local_jdk()` - returns Ok(false) placeholder ✅
   - Permission verification (`verify_removal_permission`) ✅
   - Path validation (via JdkRepository) ✅
   - Dependency checking for other tools (`check_tool_dependencies`) ✅

3. **Unit Tests** ✅
   - `src/uninstall/mod.rs` - JDK resolution and removal tests ✅
   - `src/uninstall/safety.rs` - Safety check validation tests ✅

4. **Integration Tests** (`/tests/uninstall_integration.rs`) ✅
   - Real directory removal testing ✅
   - Stub active JDK detection verification ✅
   - Permission error handling ✅

### Success Criteria

- Correctly identify JDKs to uninstall based on version specification
- Stub functions for active JDK detection ready for future implementation
- Safely remove JDK directories with rollback on failure
- Calculate and display accurate disk space information

## Phase 2: Exact Specification Enforcement and Batch Operations ✅ COMPLETED

### Input Resources

- Phase 1 deliverables
- `/src/storage/listing.rs` - InstalledJdk model for display
- Error message patterns for clarity

### Deliverables ✅ COMPLETED

1. **Selection Module** (`/src/uninstall/selection.rs`) ✅
   - Error reporting when multiple JDKs match a pattern ✅
   - Clear instructions for exact specification ✅
   - Helper functions (`JdkSelector::filter_by_distribution`, `format_selection_summary`) ✅
   - Distribution filtering using case-insensitive matching ✅
   - No interactive selection - returns error for ambiguous patterns ✅

2. **Batch Operations** (`/src/uninstall/batch.rs`) ✅
   - Multi-JDK removal using JdkRepository ✅
   - Batch uninstall logic with `BatchUninstaller` ✅
   - Batch confirmation prompts ✅
   - Multi-progress bars for visual feedback ✅
   - Transaction-like behavior (report all successes/failures) ✅

3. **Unit Tests** ✅
   - `src/uninstall/selection.rs` - Filter and formatting tests ✅
   - `src/uninstall/batch.rs` - Batch operation tests ✅

4. **Integration Tests** (`/tests/uninstall_batch_integration.rs`) ✅
   - Multiple JDK removal scenarios ✅
   - Error message validation for ambiguous patterns ✅
   - Partial failure recovery testing ✅

### Success Criteria

- Display clear error message when multiple JDKs match with exact specification instructions
- Provide helpful examples showing how to specify JDKs exactly
- Successfully remove all versions with --all flag
- Show comprehensive batch operation summary
- Handle partial failures gracefully

## Phase 3: Command Implementation and CLI Integration ❌ NOT STARTED

### Input Resources

- Phase 1 & 2 deliverables ✅ AVAILABLE
- `/src/main.rs` - Existing CLI structure with clap
- `/src/commands/` - Command pattern implementation

### Deliverables ❌ PENDING

1. **Uninstall Command** (`/src/commands/uninstall.rs`)
   - Command argument parsing
   - Integration with uninstall modules
   - Option flag handling:
     - `--force`: Placeholder for future safety check override (currently no-op)
     - `--dry-run`: Preview without removing
     - `--all`: Remove all versions
   - Error handling and exit codes

2. **CLI Integration** (update `/src/main.rs`)
   - Add uninstall subcommand with clap derive
   - Command-line options and help text
   - Proper exit code mapping

3. **User Feedback Module** (`/src/uninstall/feedback.rs`)
   - Confirmation prompts (unless --force)
   - Progress indicators for large removals
   - Success/warning messages
   - Disk space freed reporting

4. **Unit Tests** (use mocks extensively)
   - `src/commands/uninstall.rs` - Command logic tests (mock uninstall operations)
   - `src/uninstall/feedback.rs` - User interaction tests (mock terminal output)
   - CLI argument parsing tests

5. **Integration Tests** (`/tests/uninstall_command_integration.rs`) (no mocks)
   - Full command execution testing
   - Various argument combinations
   - Error message validation
   - Exit code verification

### Success Criteria

- `kopi uninstall 21` prompts for confirmation and removes JDK
- `kopi uninstall corretto@21 --force` removes without confirmation
- `kopi uninstall corretto --all` removes all Corretto versions
- `kopi uninstall 21 --dry-run` shows what would be removed
- Clear error messages with appropriate exit codes

## Phase 4: Metadata Updates and Integration 🟡 PARTIALLY COMPLETE

### Input Resources

- Phase 1-3 deliverables
- `/src/cache/mod.rs` - Cache management functions
- `/src/models/metadata.rs` - JdkMetadata model
- `/src/storage/repository.rs` - Metadata persistence via JdkRepository

### Deliverables

1. **Metadata Update Module** 🟡 HANDLED BY JdkRepository
   - JDK metadata removal is handled by `JdkRepository::remove_jdk()`
   - No separate metadata module needed
   - Metadata cleanup integrated into removal process

2. **Integration Updates**
   - Update list command to show disk usage using JdkLister::get_jdk_size()
   - Enhance current command to warn if active JDK is missing
   - Add uninstall information to doctor command

3. **Post-Uninstall Checks** (`/src/uninstall/post_check.rs`)
   - Verify complete removal using JdkLister
   - Check for orphaned .meta.json files
   - Validate shim functionality
   - Suggest next actions if needed

4. **Unit Tests** (use mocks extensively)
   - `src/uninstall/metadata.rs` - Metadata update tests (mock file operations)
   - `src/uninstall/post_check.rs` - Validation tests (mock JdkLister)

5. **Integration Tests** (`/tests/uninstall_metadata_integration.rs`) (no mocks)
   - Metadata consistency after uninstall
   - Command integration verification
   - Multi-command workflow testing

### Success Criteria

- Metadata files are cleaned up after JDK removal
- Other commands handle missing JDKs gracefully
- Post-uninstall state is validated
- Clear guidance provided when last JDK removed

## Phase 5: Platform-Specific Handling and Error Recovery 🟡 PARTIALLY COMPLETE

### Input Resources

- All previous phase deliverables
- Platform-specific documentation
- Error scenarios from testing

### Deliverables

1. **Platform Handler** ❌ NOT IMPLEMENTED
   - Windows-specific handling needed:
     - Files in use detection
     - Antivirus interference handling
   - Unix/Linux/macOS handling needed:
     - Symbolic link cleanup
     - Permission preservation
   - Note: Basic atomic operations are implemented

2. **Error Recovery Module** (`/src/uninstall/recovery.rs`)
   - Partial removal detection
   - Cleanup of failed removals
   - Recovery suggestions
   - Force removal options

3. **End-to-End Integration Tests** (`/tests/uninstall_e2e.rs`)
   - Complete uninstall workflows
   - Platform-specific scenarios
   - Error recovery testing
   - Concurrent operation handling

4. **Documentation Updates**
   - Update `/docs/reference.md` with uninstall command details
   - Add troubleshooting section
   - Platform-specific notes
   - Common error solutions

### Success Criteria

- Handles platform-specific edge cases
- Recovers from partial failures
- Provides clear error resolution steps
- Documentation complete and accurate

## Implementation Guidelines

### For Each Phase:

1. Start with `/clear` command to reset context
2. Load this plan.md and relevant phase resources
3. Implement deliverables incrementally
4. Run quality checks after each module:
   - `cargo fmt`
   - `cargo clippy`
   - `cargo check`
   - `cargo test --lib`
5. Commit completed phase before proceeding

### Testing Strategy

#### Unit Tests (use mocks extensively)

- Test individual module functionality in isolation
- Mock JdkRepository and file system operations
- Mock user interactions and prompts
- Focus on logic validation and edge cases
- Example:
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      use mockall::*;

      #[test]
      fn test_safety_check_stub() {
          // Test stub functions always return false for now
          assert_eq!(is_active_global_jdk("temurin", "21.0.5+11").unwrap(), false);
          assert_eq!(is_active_local_jdk("temurin", "21.0.5+11").unwrap(), false);

          // Future: will test actual active JDK detection when implemented
      }
  }
  ```

#### Integration Tests (no mocks)

- Test complete uninstall workflows
- Use temporary directories with real JdkRepository
- Verify actual file system state changes
- Test platform-specific behaviors
- Example:
  ```rust
  #[test]
  fn test_real_jdk_removal() {
      let temp_dir = tempfile::tempdir().unwrap();
      let config = KopiConfig::test_config(temp_dir.path());
      let repo = JdkRepository::new(&config);

      // Create mock JDK structure
      let jdk_path = temp_dir.path().join("jdks").join("temurin-21.0.5+11");
      fs::create_dir_all(&jdk_path).unwrap();

      // Execute actual uninstall using repository
      let result = repo.remove_jdk(
          &Distribution::Temurin,
          &Version::new(21, 0, 5, Some("11".to_string()))
      );
      assert!(result.is_ok());

      // Verify removal
      assert!(!jdk_path.exists());
  }
  ```

### Error Handling Priorities

1. Ambiguous version specification - require exact JDK specification with helpful examples
2. Active JDK protection - stub implementation (always allows removal for now)
3. Permission errors - suggest appropriate solutions
4. Files in use - platform-specific guidance
5. Partial removals - provide recovery options
6. Missing JDKs - clear error with available options

### Safety Considerations

1. Always validate removal paths are within kopi directory
2. Implement atomic operations with rollback capability
3. Preserve user data and configurations
4. Never remove shims during JDK uninstall
5. Require confirmation for destructive operations
6. Active JDK protection deferred to future implementation (stub returns false)

### Implementation Note: Exact Specification Requirement

Instead of interactive selection when multiple JDKs match, the uninstall command returns an error with clear instructions. This design choice:

- Prevents accidental removal of wrong JDK versions
- Ensures users are explicit about which JDK to remove
- Avoids complexity of interactive prompts in automated environments
- Provides clear, actionable error messages with examples

### Exit Codes

- 0: Success
- 2: Invalid arguments or configuration
- 4: JDK not found
- 10: Active JDK (reserved for future use when active JDK detection is implemented)
- 13: Permission denied
- 14: Partial removal failure

## Next Steps

### Immediate Priority: Phase 3 - CLI Integration

1. Create `/src/commands/uninstall.rs`:

   ```rust
   pub struct UninstallCommand {
       config: KopiConfig,
   }
   ```

2. Update `/src/main.rs`:
   - Add `Uninstall` variant to `Commands` enum
   - Add command options: `--force`, `--dry-run`, `--all`
   - Wire up to UninstallCommand

3. Update `/src/commands/mod.rs`:
   - Add `pub mod uninstall;`

### Secondary Tasks

1. **Force Flag Implementation**:
   - Currently safety checks always pass (stubs return false)
   - Need to implement force flag to bypass future safety checks

2. **Active JDK Detection**:
   - Replace stub functions when `global` and `local` commands are ready
   - Update safety checks to actually detect active JDKs

3. **List Command Enhancement**:
   - Show JDK sizes in `kopi list` output
   - Already have `JdkRepository::get_jdk_size()` available

## Implementation Summary

### What's Working

- ✅ Core uninstall logic with atomic operations
- ✅ Version pattern matching for all distribution formats
- ✅ Batch uninstall capability
- ✅ Progress indicators and disk space reporting
- ✅ Error handling with clear messages
- ✅ Comprehensive test coverage

### What's Missing

- ❌ CLI command integration
- ❌ Force flag to bypass safety checks
- ❌ Active JDK detection (awaiting global/local commands)
- ❌ Running process detection
- ❌ Platform-specific edge cases

The uninstall functionality is feature-complete at the module level. The primary remaining work is integrating it into the CLI command structure.
