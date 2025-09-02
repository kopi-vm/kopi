# FR-0001: Process-level locking for installation operations

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
- Related Requirements: FR-0002 (uninstallation), FR-0003 (cache), FR-0004 (timeout)
- Issue: N/A – No tracking issue created yet
- Task: N/A – Implementation not started

## Requirement Statement

The system SHALL provide exclusive process-level locking for JDK installation operations to prevent concurrent installations to the same JDK version.

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
   - THEN the lock SHALL be specific to the exact version (vendor-version-os-arch)
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

- Use native `std::fs::File` locking (stable since Rust 1.89.0)
- Lock file location: `~/.kopi/locks/{vendor}-{version}-{os}-{arch}.lock`
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
- Filesystem support for advisory locks

## Out of Scope

- Distributed locking across multiple machines
- Lock priority or queuing mechanisms
- GUI for lock monitoring