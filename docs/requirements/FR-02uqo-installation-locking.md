# Process-level locking for installation operations

## Metadata

- ID: FR-02uqo
- Type: Functional Requirement
- Category: Platform
- Priority: P0 (Critical)
- Owner: Development Team
- Reviewers: Architecture Team
- Status: Accepted

## Links

- Implemented by Tasks: N/A – Not yet implemented
- Related Requirements: FR-ui8x2, FR-v7ql4, FR-gbsz6, FR-c04js, NFR-vcxp8, NFR-g12ex
- Related ADRs: [ADR-8mnaz](../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Tests: N/A – Not yet tested
- Issue: N/A – No tracking issue created yet
- PR: N/A – Not yet implemented

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

## User Story (if applicable)

As a kopi user, I want the tool to handle concurrent installation attempts safely, so that my JDK installations don't become corrupted when multiple processes run simultaneously.

## Technical Details (if applicable)

### Functional Requirement Details

- Lock acquisition must happen before any filesystem modifications
- Lock key must be canonicalized after all alias resolution
- Lock must be exclusive (write lock) to prevent any concurrent access
- Lock release must be automatic on process termination

## Verification Method

### Test Strategy

- Test Type: Integration
- Test Location: `tests/locking_tests.rs` (planned)
- Test Names: `test_fr_02uqo_concurrent_install_lock`, `test_fr_02uqo_parallel_different_versions`

### Verification Commands

```bash
# Specific commands to verify this requirement
cargo test test_fr_02uqo
```

### Success Metrics

- Metric 1: Zero corrupted installations during concurrent install attempts
- Metric 2: Lock acquisition time < 100ms for uncontended locks

## Platform Considerations

### Unix

- Uses advisory file locks via `flock` system call
- Lock files stored in `$KOPI_HOME/locks/`

### Windows

- Uses Windows file locking via `LockFileEx` API
- Lock files stored in `%KOPI_HOME%\locks\`

### Cross-Platform

- Lock file naming must be consistent across platforms
- Path separators handled by std::path abstractions

## Risks & Mitigation

| Risk                             | Impact | Likelihood | Mitigation                      | Validation                  |
| -------------------------------- | ------ | ---------- | ------------------------------- | --------------------------- |
| Filesystem doesn't support locks | High   | Low        | Fallback to process-local mutex | Test on network filesystems |
| Lock file permissions incorrect  | Medium | Medium     | Create with appropriate umask   | Verify permissions in tests |
| Stale lock files accumulate      | Low    | Medium     | Cleanup on startup              | Monitor lock directory size |

## External References

N/A – No external references

## Change History

- 2025-09-02: Initial version
- 2025-09-03: Updated to use 5-character ID format

## Out of Scope

- Distributed locking across multiple machines
- Lock priority or queuing mechanisms
- GUI for lock monitoring
