# Process-level locking for installation operations

## Metadata

- Type: Functional Requirement
- Status: Accepted
  <!-- Proposed: Under discussion | Accepted: Approved for implementation | Implemented: Code complete | Verified: Tests passing | Deprecated: No longer applicable -->

## Links

- Implemented by Tasks: N/A – Not yet implemented
- Related Requirements: FR-ui8x2, FR-v7ql4, FR-gbsz6, FR-c04js, NFR-vcxp8, NFR-g12ex
- Related ADRs: ADR-8mnaz
- Tests: N/A – Not yet tested
- Issue: N/A – No tracking issue created yet
- PR: N/A – Not yet implemented

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

## Verification Method

### Test Strategy

- Test Type: Integration
- Test Location: `tests/locking_tests.rs` (planned)
- Test Names: `test_fr_02uqo_concurrent_install_lock`, `test_fr_02uqo_parallel_different_versions`, `test_fr_02uqo_crash_releases_lock`

### Verification Commands

```bash
# Specific commands to verify this requirement
cargo test test_fr_02uqo_concurrent_install_lock
cargo test test_fr_02uqo_parallel_different_versions
cargo test test_fr_02uqo_crash_releases_lock
```

### Success Metrics

- Metric 1: Zero corrupted installations after 100 concurrent installation stress tests.
- Metric 2: Lock acquisition time for uncontended locks remains below 100 ms.
- Metric 3: Locks become available within 1 second after forced process termination in 100% of observed cases.

## Dependencies

- Depends on: N/A – No dependencies
- Blocks: FR-gbsz6, FR-ui8x2, FR-v7ql4 (shared locking infrastructure)

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
