# Lock cleanup reliability

## Metadata

- ID: NFR-vcxp8
- Type: Non-Functional Requirement
- Category: Reliability
- Priority: P0 (Critical)
- Owner: Development Team
- Reviewers: Architecture Team
- Status: Accepted

## Links

- Implemented by Tasks: N/A – Not yet implemented
- Related Requirements: FR-02uqo, FR-ui8x2, FR-v7ql4
- Related ADRs: [ADR-8mnaz](../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Tests: N/A – Not yet tested
- Issue: N/A – No tracking issue created yet
- PR: N/A – Not yet implemented

## Requirement Statement

The system SHALL achieve 100% automatic lock cleanup on local filesystems using native std::fs::File advisory locks, with graceful degradation to atomic operations only on network filesystems.

## Rationale

Lock cleanup reliability is critical to prevent system deadlocks:

- Native advisory locks are automatically released by the kernel on process termination
- This eliminates the need for complex stale lock detection
- Network filesystems (NFS) have unreliable lock support, requiring alternative strategies
- Atomic filesystem operations provide sufficient safety without locks

## User Story (if applicable)

The system shall automatically clean up locks in all termination scenarios to ensure subsequent operations can proceed without manual intervention.

## Acceptance Criteria

- [ ] Locks automatically released in 100% of cases on local filesystems
- [ ] Includes normal exit, panic, SIGKILL, and system crash scenarios
- [ ] No manual cleanup required
- [ ] System skips file locking on network filesystems (NFS, CIFS, SMB)
- [ ] Operations rely on atomic filesystem operations when locks unavailable
- [ ] Warning displayed when network filesystem detected
- [ ] Lock files contain no data (kernel manages state)
- [ ] Lock files created with owner-only permissions (0600)
- [ ] Lock file existence does not indicate lock is held
- [ ] Locks available immediately after process termination (< 1 second)
- [ ] No timeout or polling required for cleanup

## Technical Details (if applicable)

### Non-Functional Requirement Details

- Reliability: 100% automatic cleanup on supported filesystems
- Performance: Lock release < 1 second after process termination
- Security: Lock files with 0600 permissions (owner read/write only)
- Compatibility: Graceful degradation on unsupported filesystems

### Filesystem Support Matrix

| Filesystem | Lock Support | Fallback Strategy |
| ---------- | ------------ | ----------------- |
| ext4       | Full         | N/A               |
| APFS       | Full         | N/A               |
| NTFS       | Full         | N/A               |
| NFS        | Unreliable   | Atomic operations |
| SMB/CIFS   | Unreliable   | Atomic operations |

## Verification Method

### Test Strategy

- Test Type: Integration
- Test Location: `tests/lock_reliability_tests.rs` (planned)
- Test Names: `test_nfr_vcxp8_crash_cleanup`, `test_nfr_vcxp8_network_fs_fallback`

### Verification Commands

```bash
# Specific commands to verify this requirement
cargo test test_nfr_vcxp8
# Stress test
for i in {1..100}; do cargo test test_nfr_vcxp8_stress; done
```

### Success Metrics

- Metric 1: 100% lock cleanup rate across 1000 forced terminations
- Metric 2: Lock re-acquisition time < 1 second in all cases
- Metric 3: Zero stale locks after test suite completion

## Dependencies

- Depends on: Native std::fs::File locking API (Rust 1.89.0+)
- Blocks: FR-02uqo, FR-ui8x2, FR-v7ql4 (all require reliable cleanup)

## Platform Considerations

### Unix

- Advisory locks via flock()
- Automatic cleanup by kernel on process termination
- Detection of network filesystems via statfs()

### Windows

- File locks via LockFileEx()
- Automatic cleanup by Windows kernel
- Network drive detection via GetDriveType()

### Cross-Platform

- Consistent behavior for lock cleanup
- Unified network filesystem detection

## Risks & Mitigation

| Risk                                | Impact | Likelihood | Mitigation                       | Validation                     |
| ----------------------------------- | ------ | ---------- | -------------------------------- | ------------------------------ |
| Filesystem doesn't support locks    | High   | Low        | Detect and use atomic operations | Test on various filesystems    |
| Kernel bug prevents cleanup         | High   | Very Low   | Document known issues            | Monitor kernel bug trackers    |
| Permission issues on lock directory | Medium | Low        | Create with proper umask         | Test with various umask values |

## Implementation Notes

- Use `std::fs::File::lock_exclusive()` from Rust 1.89.0
- Detect filesystem type at runtime
- Log filesystem detection results at debug level
- Consider caching filesystem type detection
- Implement atomic operations using rename() for fallback

## External References

N/A – No external references

## Change History

- 2025-09-02: Initial version
- 2025-09-03: Updated to use 5-character ID format
