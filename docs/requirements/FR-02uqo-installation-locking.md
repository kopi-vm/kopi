# Process-level locking for installation operations

## Metadata

- Type: Functional Requirement
- Status: Accepted
  <!-- Proposed: Under discussion | Accepted: Approved for implementation | Implemented: Code complete | Verified: Tests passing | Deprecated: No longer applicable -->

## Links

- Related Analyses:
  - [AN-m9efc-concurrent-process-locking](../analysis/AN-m9efc-concurrent-process-locking.md)
- Prerequisite Requirements:
  - N/A – No prerequisites
- Dependent Requirements:
  - [FR-gbsz6-lock-timeout-recovery](../requirements/FR-gbsz6-lock-timeout-recovery.md)
  - [FR-ui8x2-uninstallation-locking](../requirements/FR-ui8x2-uninstallation-locking.md)
  - [FR-v7ql4-cache-locking](../requirements/FR-v7ql4-cache-locking.md)
  - [FR-rxelv-file-in-use-detection](../requirements/FR-rxelv-file-in-use-detection.md)
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Related Tasks:
  - N/A – Not yet implemented

## Requirement Statement

The system SHALL provide exclusive process-level locking for JDK installation operations so that concurrent installation attempts targeting the same vendor-version-os-arch coordinate never execute simultaneously.

## Rationale

Without process-level locking, multiple kopi processes attempting to install the same JDK version could corrupt installations, trigger race conditions during directory creation, leave metadata inconsistent, and waste bandwidth on duplicate downloads.

## User Story (if applicable)

As a kopi user, I want the tool to acquire an exclusive installation lock before modifying the filesystem, so that concurrent installation attempts cannot corrupt my JDK installs.

## Acceptance Criteria

- [ ] Exclusive lock acquisition prevents more than one process from installing the same canonicalized vendor-version-os-arch coordinate at a time.
- [ ] Lock keys are derived from canonicalized coordinates after alias resolution to guarantee equivalent requests share the same lock file.
- [ ] Lock release occurs on both successful and failed installation exits, returning the system to an unlocked state.
- [ ] Operating-system-managed cleanup releases locks automatically when a process crashes or is killed, allowing new installers to proceed without manual intervention.
- [ ] Installations for different coordinates run in parallel without blocking each other.

## Technical Details (if applicable)

### Functional Requirement Details

- Use native `std::fs::File::lock_exclusive()` for blocking acquisition and `try_lock_exclusive()` for optional non-blocking modes.
- Lock files reside at `$KOPI_HOME/locks/{vendor}-{version}-{os}-{arch}.lock` with normalized components.
- Acquire the lock before any filesystem mutations and hold it until installation completes or aborts.
- Support both blocking waits (default) and configurable non-blocking attempts driven by timeout settings.
- Dropping the file handle or invoking `unlock()` releases the lock; cleanup relies on OS semantics.

### Non-Functional Requirement Details

N/A – Not applicable.

## Platform Considerations

### Unix

- Implements advisory locking via `flock` through Rust standard library wrappers.
- Stores lock files in `$KOPI_HOME/locks/` with owner-only permissions.

### Windows

- Uses `LockFileEx` via Rust standard library.
- Stores lock files in `%KOPI_HOME%\locks\` and relies on kernel-managed cleanup.

### Cross-Platform

- Lock filenames must remain identical across platforms after normalization.
- Path handling must use `std::path` to accommodate separators.

## Risks & Mitigation

| Risk                            | Impact | Likelihood | Mitigation                    | Validation                  |
| ------------------------------- | ------ | ---------- | ----------------------------- | --------------------------- |
| Filesystem lacks advisory locks | High   | Low        | Detect and fall back to mutex | Test on network filesystems |
| Lock file permissions incorrect | Medium | Medium     | Create with restricted umask  | Verify permissions in tests |
| Stale lock files accumulate     | Low    | Medium     | Cleanup on startup before use | Monitor lock directory size |

## Implementation Notes

- Provide configurable blocking vs. timeout-driven behavior through FR-gbsz6 settings.
- Canonicalize coordinates after resolving aliases, architecture defaults, and vendor synonyms.
- Ensure logging captures lock acquisition, contention, and release events for diagnostics.
- Treat lock files as zero-length placeholders; never persist metadata inside them.

## External References

N/A – No external references

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd).
