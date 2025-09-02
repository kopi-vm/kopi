# NFR-0002: Lock cleanup reliability

## Metadata
- Type: Non-Functional Requirement
- Category: Reliability
- Owner: Development Team
- Reviewers: Architecture Team
- Status: Approved
- Priority: P0
- Date Created: 2025-09-02
- Date Modified: 2025-09-02

## Links
- Analysis: [`docs/analysis/AN-0001-concurrent-process-locking.md`](../analysis/AN-0001-concurrent-process-locking.md)
- Related ADRs: [`ADR-0001-concurrent-process-locking-strategy.md`](../adr/ADR-0001-concurrent-process-locking-strategy.md)
- Related Requirements: FR-0001, FR-0002, FR-0003 (all lock operations)
- Issue: N/A – No tracking issue created yet
- Task: N/A – Implementation not started

## Requirement Statement

The system SHALL achieve 100% automatic lock cleanup on local filesystems using native std::fs::File advisory locks, with graceful degradation to atomic operations only on network filesystems.

## Rationale

Lock cleanup reliability is critical to prevent system deadlocks:
- Native advisory locks are automatically released by the kernel on process termination
- This eliminates the need for complex stale lock detection
- Network filesystems (NFS) have unreliable lock support, requiring alternative strategies
- Atomic filesystem operations provide sufficient safety without locks

## Acceptance Criteria

1. **Local Filesystem Reliability**
   - On local filesystems, locks SHALL be automatically released in 100% of cases
   - This SHALL include normal exit, panic, SIGKILL, and system crash scenarios
   - No manual cleanup SHALL be required

2. **Network Filesystem Handling**
   - On network filesystems (NFS, CIFS, SMB), the system SHALL skip file locking
   - Operations SHALL rely solely on atomic filesystem operations
   - A warning SHALL be displayed when network filesystem is detected

3. **Lock File State**
   - Lock files SHALL contain no data (kernel manages lock state)
   - Lock files SHALL be created with owner-only permissions (0600)
   - Lock file existence SHALL NOT indicate lock is held

4. **Recovery Time**
   - After process termination, locks SHALL be available immediately (< 1 second)
   - No timeout or polling SHALL be required for cleanup
   - New processes SHALL acquire released locks without delay

## Measurement Methods

- **Crash Testing**: Force-kill processes at various stages
- **Filesystem Testing**: Test on ext4, APFS, NTFS, NFS, SMB
- **Stress Testing**: Rapid lock acquire/release cycles
- **Recovery Testing**: Measure time from kill to re-acquisition

## Target Metrics

| Metric | Target | Minimum Acceptable |
|--------|--------|-------------------|
| Cleanup rate (local FS) | 100% | 100% |
| Cleanup rate (network FS) | N/A (skip locks) | N/A |
| Recovery time | < 1s | < 5s |
| Lock file permissions | 0600 | 0600 |
| Kernel lock release | Automatic | Automatic |

## Implementation Notes

- Use RAII pattern to ensure lock release on scope exit
- Implement filesystem type detection for NFS/network drives
- Log filesystem type detection at DEBUG level
- Never write PID or timestamp data to lock files

## Verification Steps

1. **Normal Exit Test**
   - Acquire lock and exit normally
   - Verify immediate lock availability

2. **Crash Test**
   - Acquire lock and force-kill process
   - Verify lock is released automatically

3. **Panic Test**
   - Acquire lock and trigger panic
   - Verify cleanup occurs

4. **Network Filesystem Test**
   - Run on NFS mount
   - Verify locking is skipped with warning

5. **Permission Test**
   - Check lock file permissions
   - Verify owner-only access (0600)

## Dependencies

- Native std::fs::File locking API (Rust 1.89.0+)
- Filesystem type detection mechanism
- Atomic rename operation support

## Out of Scope

- Manual lock cleanup mechanisms
- Stale lock detection for advisory locks
- Distributed lock cleanup
- Lock cleanup on network filesystems