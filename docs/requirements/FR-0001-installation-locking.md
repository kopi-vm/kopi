# FR-0001: Process-level locking for installation operations

## Metadata
- Type: Functional Requirement
- Owner: Development Team
- Reviewers: Architecture Team
- Status: Approved
- Priority: P0
- Date Created: 2025-09-02
- Date Modified: 2025-09-03

## Links
- Analysis: [`docs/analysis/AN-m9efc-concurrent-process-locking.md`](../analysis/AN-m9efc-concurrent-process-locking.md)
- Related ADRs: [`ADR-8mnaz-concurrent-process-locking-strategy.md`](../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Related Functional Requirements:
  - FR-0002 (uninstallation locking)
  - FR-0003 (cache operation locking)
  - FR-0004 (lock timeout and recovery)
  - FR-0005 (user feedback for lock contention)
- Related Non-Functional Requirements:
  - NFR-0002 (lock cleanup reliability and permissions)
  - NFR-0003 (cross-platform lock compatibility)
- Issue: N/A – No tracking issue created yet
- Task: N/A – Implementation not started

## Requirement Statement

The system SHALL provide exclusive process-level locking for JDK installation operations to prevent concurrent installations to the same JDK version. The lock SHALL be acquired using a canonicalized coordinate (vendor-version-os-arch) to ensure that different representations of the same JDK version map to the same lock.

## Rationale

Without process-level locking, multiple kopi processes attempting to install the same JDK version simultaneously could result in:
- Corrupted JDK installations due to partial file writes
- Race conditions during directory creation and file extraction
- Inconsistent metadata states
- Wasted bandwidth from duplicate downloads

## Acceptance Criteria

1. **Exclusive Lock Acquisition**
   - GIVEN multiple kopi processes
   - WHEN two or more processes attempt to install the same JDK version
   - THEN only one process SHALL acquire the installation lock
   - AND other processes SHALL wait or fail based on timeout configuration

2. **Lock Granularity**
   - GIVEN a JDK installation request
   - WHEN acquiring locks
   - THEN the lock SHALL be specific to the canonicalized version coordinate (vendor-version-os-arch)
   - AND the coordinate SHALL be normalized after all alias resolution
   - AND installations of different versions SHALL proceed in parallel

3. **Lock Release**
   - GIVEN a process holding an installation lock
   - WHEN the installation completes (success or failure)
   - THEN the lock SHALL be released
   - AND waiting processes SHALL be able to proceed

4. **Crash Recovery**
   - GIVEN a process holding an installation lock
   - WHEN the process crashes or is killed
   - THEN the lock SHALL be automatically released by the operating system
   - AND other processes SHALL be able to acquire the lock

## Implementation Notes

- Use native `std::fs::File` locking API (stable since Rust 1.89.0):
  - `File::lock_exclusive()` for blocking lock acquisition
  - `File::try_lock_exclusive()` for non-blocking lock acquisition
  - `File::unlock()` for lock release (or automatic on file drop)
- Lock file location: `$KOPI_HOME/locks/{vendor}-{version}-{os}-{arch}.lock`
  - Where `$KOPI_HOME` is the resolved Kopi home directory
  - Lock key components must be canonicalized post-alias resolution
- Lock type: Exclusive (writer) lock
- Lock acquisition: Support both blocking and non-blocking modes

## Verification Steps

1. **Concurrent Installation Test**
   - Start two processes installing the same JDK version
   - Verify only one proceeds while the other waits

2. **Parallel Different Version Test**
   - Start two processes installing different JDK versions
   - Verify both proceed in parallel

3. **Crash Recovery Test**
   - Start installation process and kill it mid-operation
   - Verify lock is released and new process can acquire it

## Dependencies

- Native std::fs::File locking API (Rust 1.89.0+)
- Filesystem support for advisory locks (see NFR-0002 for fallback behavior on unsupported filesystems)
- Cross-platform compatibility constraints (see NFR-0003)

## Out of Scope

- Distributed locking across multiple machines
- Lock priority or queuing mechanisms
- GUI for lock monitoring