# FR-0002: Process-level locking for uninstallation operations

## Metadata
- Type: Functional Requirement
- Owner: Development Team
- Reviewers: Architecture Team
- Status: Approved
- Priority: P0
- Date Created: 2025-09-02
- Date Modified: 2025-09-02

## Links
- Analysis: [`docs/analysis/AN-0001-concurrent-process-locking.md`](../analysis/AN-0001-concurrent-process-locking.md)
- Related ADRs: [`ADR-0001-concurrent-process-locking-strategy.md`](../adr/ADR-0001-concurrent-process-locking-strategy.md)
- Related Requirements: FR-0001 (installation), FR-0004 (timeout)
- Issue: N/A – No tracking issue created yet
- Task: N/A – Implementation not started

## Requirement Statement

The system SHALL provide exclusive process-level locking for JDK uninstallation operations to ensure atomic and safe removal of JDK installations.

## Rationale

Without process-level locking, concurrent uninstallation operations could result in:
- Partial directory deletions leaving orphaned files
- Race conditions with installation processes
- Conflicts when one process is using a JDK while another removes it
- Inconsistent metadata after partial removal

## Acceptance Criteria

1. **Exclusive Lock for Removal**
   - GIVEN a JDK uninstallation request
   - WHEN attempting to uninstall a specific JDK version
   - THEN the process SHALL acquire an exclusive lock for that version
   - AND no other process SHALL be able to install or uninstall that version concurrently

2. **Atomic Operation**
   - GIVEN an uninstallation in progress
   - WHEN the operation completes
   - THEN the JDK directory SHALL be completely removed
   - OR the operation SHALL fail with no partial removal

3. **Conflict Prevention**
   - GIVEN a JDK version being uninstalled
   - WHEN another process attempts to install the same version
   - THEN the installation SHALL wait or fail based on timeout configuration
   - AND the operations SHALL not interfere with each other

4. **Metadata Consistency**
   - GIVEN a successful uninstallation
   - WHEN the operation completes
   - THEN all associated metadata SHALL be removed
   - AND the system state SHALL be consistent

## Implementation Notes

- Use the same lock file as installation: `~/.kopi/locks/{vendor}-{version}-{os}-{arch}.lock`
- Lock type: Exclusive (writer) lock
- Ensure lock is held for entire uninstallation process
- Clean up metadata files after successful directory removal

## Verification Steps

1. **Concurrent Uninstall Test**
   - Start two processes uninstalling the same JDK version
   - Verify only one proceeds while the other waits or fails

2. **Install-Uninstall Conflict Test**
   - Start uninstallation of a JDK version
   - Attempt to install the same version concurrently
   - Verify operations are serialized correctly

3. **Atomic Removal Test**
   - Kill uninstallation process mid-operation
   - Verify either complete removal or no changes

## Dependencies

- Native std::fs::File locking API (Rust 1.89.0+)
- Shared lock file with installation operations

## Out of Scope

- Forced removal without locks
- Uninstallation of JDKs in use by running processes
- Cleanup of JDKs installed outside of kopi