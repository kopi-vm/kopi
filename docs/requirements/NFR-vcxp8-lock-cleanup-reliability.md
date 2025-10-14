# NFR-vcxp8 Lock Cleanup Reliability

## Metadata

- Type: Non-Functional Requirement
- Status: Approved
  <!-- Draft: Under discussion | Approved: Ready for implementation | Rejected: Decision made not to pursue this requirement -->

## Links

- Prerequisite Requirements:
  - N/A – No prerequisites
- Dependent Requirements:
  - [FR-02uqo-installation-locking](../requirements/FR-02uqo-installation-locking.md)
  - [FR-ui8x2-uninstallation-locking](../requirements/FR-ui8x2-uninstallation-locking.md)
  - [FR-v7ql4-cache-locking](../requirements/FR-v7ql4-cache-locking.md)
- Related Tasks:
  - [T-ec5ew-locking-foundation](../tasks/T-ec5ew-locking-foundation/README.md)

## Requirement Statement

Kopi SHALL ensure automatic cleanup of lock state on supported local filesystems while providing graceful degradation to atomic operations when advisory locks are unavailable or unreliable.

## Rationale

Reliable cleanup prevents deadlocks, eliminates manual intervention after crashes, and guarantees subsequent operations can progress without delay.

## User Story (if applicable)

The system shall automatically release locks after every exit condition so that users never have to manually remove stale lock artefacts.

## Acceptance Criteria

- [ ] Local filesystem locks (ext4, APFS, NTFS) are automatically released after normal exit, panic, SIGKILL, or system crash in 100% of tested scenarios.
- [ ] Network or degraded filesystems (NFS, SMB/CIFS) trigger detection logic that disables advisory locks and switches to atomic operations with warning output.
- [ ] No manual cleanup is required; lock files remain zero-length placeholders with no stateful contents.
- [ ] Lock files are created with owner-only permissions (`0o600` or stricter) to ensure security and reliable cleanup.
- [ ] Locks become reacquirable within 1 second of process termination in stress tests covering at least 1000 forced exits.
- [ ] Startup hygiene scans remove orphaned temporary files created by fallback strategies.

## Technical Details (if applicable)

### Functional Requirement Details

N/A – Behavioural focus only.

### Non-Functional Requirement Details

- Reliability: 100% automatic cleanup rate on supported filesystems across 1000 forced termination tests.
- Performance: Lock release availability within 1 second post-termination.
- Security: Lock directories created with restrictive permissions; no secrets stored in lock files.
- Compatibility: Detect filesystem capabilities using `statfs` (Unix) and `GetVolumeInformation`/`GetDriveType` (Windows).

## Platform Considerations

### Unix

- Use `flock` and detect filesystem type with `statfs`; fallback to atomic staging when reliability cannot be guaranteed.

### Windows

- Use `LockFileEx`; detect network drives with `GetDriveType` and fallback gracefully to atomic operations when necessary.

### Cross-Platform

- Provide consistent warning and logging when falling back to atomic operations.
- Maintain a shared startup cleanup routine to address orphaned artefacts.

## Risks & Mitigation

| Risk                                | Impact | Likelihood | Mitigation                              | Validation                     |
| ----------------------------------- | ------ | ---------- | --------------------------------------- | ------------------------------ |
| Filesystem lacks advisory locks     | High   | Low        | Detect and use atomic operations        | Test across filesystem matrix  |
| Kernel bug prevents cleanup         | High   | Very Low   | Document known issues; provide override | Monitor vendor advisories      |
| Permission issues on lock directory | Medium | Low        | Create directories with strict umask    | Test with varying umask values |

## Implementation Notes

- Emit debug logging to capture filesystem detection outcomes and cleanup actions.
- Cache detection results per path to minimise repeated syscalls while still allowing manual override.
- Provide configuration toggles to force fallback mode for troubleshooting.
- Document fallback behaviour in user documentation maintained externally.

## External References

- [`std::fs::File`](https://doc.rust-lang.org/std/fs/struct.File.html) – Rust standard library locking behaviour and cleanup semantics

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd) in the templates README.
