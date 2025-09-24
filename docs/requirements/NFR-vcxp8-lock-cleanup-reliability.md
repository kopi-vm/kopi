# Lock cleanup reliability

## Metadata

- Type: Non-Functional Requirement
- Status: Accepted
  <!-- Proposed: Under discussion | Accepted: Approved for implementation | Implemented: Code complete | Verified: Tests passing | Deprecated: No longer applicable -->

## Links

- Analysis: AN-m9efc
- ADRs: ADR-8mnaz
- Depends on: N/A – No dependencies
- Blocks: FR-02uqo, FR-ui8x2, FR-v7ql4 (all rely on reliable cleanup)
- Tasks: N/A – Not yet implemented

## Requirement Statement

The system SHALL ensure 100% automatic cleanup of lock state on supported local filesystems while providing graceful degradation to atomic operations when advisory locks are unavailable or unreliable.

## Rationale

Reliable cleanup prevents deadlocks, eliminates manual intervention after crashes, reduces complexity associated with stale lock detection, and ensures subsequent operations can progress without delay.

## User Story (if applicable)

The system shall automatically release locks after every exit condition so that users never have to manually remove stale lock artifacts.

## Acceptance Criteria

- [ ] Local filesystem locks (ext4, APFS, NTFS) are automatically released in 100% of cases after normal exit, panic, SIGKILL, or system crash.
- [ ] Network or degraded filesystems (NFS, SMB/CIFS) trigger detection logic that disables advisory locks and switches to atomic operation strategies with warning output.
- [ ] No manual lock cleanup steps are required; lock files remain zero-length placeholders with no stateful contents.
- [ ] Lock files are created with owner-only permissions (`0600`), ensuring security and reliable cleanup.
- [ ] Locks become reacquirable within 1 second of process termination in stress tests covering 1000 forced exits.
- [ ] Startup hygiene scans clean up any orphaned temporary files associated with fallback strategies.

## Technical Details (if applicable)

### Functional Requirement Details

N/A – Not applicable.

### Non-Functional Requirement Details

- Reliability: 100% automatic cleanup rate on supported filesystems across 1000 forced termination tests.
- Performance: Lock release availability within 1 second post-termination.
- Security: Lock directories created with restrictive permissions; no secrets stored in lock files.
- Compatibility: Detect filesystem capabilities using `statfs` (Unix) and `GetVolumeInformation`/`GetDriveType` (Windows).

#### Filesystem Support Matrix

| Filesystem | Lock Support | Fallback Strategy |
| ---------- | ------------ | ----------------- |
| ext4       | Full         | N/A               |
| APFS       | Full         | N/A               |
| NTFS       | Full         | N/A               |
| NFS        | Unreliable   | Atomic operations |
| SMB/CIFS   | Unreliable   | Atomic operations |

## Platform Considerations

### Unix

- Advisory locks via `flock`; detect filesystem type using `statfs` and adjust strategy for network mounts.

### Windows

- File locks via `LockFileEx`; detect network drives with `GetDriveType` and fallback appropriately.

### Cross-Platform

- Provide consistent warning and logging when falling back to atomic operations.
- Maintain a shared startup cleanup routine to address orphaned artifacts.

## Risks & Mitigation

| Risk                                | Impact | Likelihood | Mitigation                              | Validation                     |
| ----------------------------------- | ------ | ---------- | --------------------------------------- | ------------------------------ |
| Filesystem lacks advisory locks     | High   | Low        | Detect and use atomic operations        | Test across filesystem matrix  |
| Kernel bug prevents cleanup         | High   | Very Low   | Document known issues; provide override | Monitor vendor advisories      |
| Permission issues on lock directory | Medium | Low        | Create directories with strict umask    | Test with varying umask values |

## Implementation Notes

- Use debug logging to capture filesystem detection outcomes and cleanup actions.
- Cache detection results per path to avoid repeated syscalls while allowing manual override.
- Provide configuration toggle to force fallback mode for troubleshooting.
- Document fallback behavior in user documentation maintained externally.

## External References

- [`std::fs::File`](https://doc.rust-lang.org/std/fs/struct.File.html) - Rust standard library documentation for file locking behavior and cleanup semantics

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd).
