# Process-level locking for uninstallation operations

## Metadata

- Type: Functional Requirement
- Status: Accepted
  <!-- Proposed: Under discussion | Accepted: Approved for implementation | Implemented: Code complete | Verified: Tests passing | Deprecated: No longer applicable -->

## Links

- Related Analyses:
  - [AN-m9efc-concurrent-process-locking](../analysis/AN-m9efc-concurrent-process-locking.md)
- Prerequisite Requirements:
  - [FR-02uqo-installation-locking](../requirements/FR-02uqo-installation-locking.md)
- Dependent Requirements:
  - [FR-rxelv-file-in-use-detection](../requirements/FR-rxelv-file-in-use-detection.md)
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Related Tasks:
  - N/A – Not yet implemented

## Requirement Statement

The system SHALL use exclusive process-level locking for JDK uninstallation operations to guarantee atomic removal of installed JDKs and prevent conflicts with concurrent installations or other uninstallations.

## Rationale

Without uninstallation locks, concurrent operations might remove directories while installs or other processes are running, leading to partial deletions, inconsistent metadata, and corrupted shims or symlinks.

## User Story (if applicable)

As a kopi user, I want uninstallation to guard against concurrent modifications, so that removing a JDK never leaves behind partial directories or broken metadata.

## Acceptance Criteria

- [ ] An exclusive lock for the canonicalized vendor-version-os-arch coordinate is acquired before any uninstallation steps run.
- [ ] While the uninstallation lock is held, no installation or uninstallation for the same coordinate can proceed; such attempts wait or fail per FR-gbsz6 timeout rules.
- [ ] Successful uninstallation removes the JDK directory, shims, and metadata atomically; failures roll back any partial changes.
- [ ] Processes attempting to install the locked coordinate receive blocking or timeout behavior consistent with shared infrastructure.
- [ ] Post-uninstallation state verification confirms that filesystem artifacts and metadata entries are fully removed.

## Technical Details (if applicable)

### Functional Requirement Details

- Reuse the installation lock file path: `$KOPI_HOME/locks/{vendor}-{version}-{os}-{arch}.lock`.
- Hold the lock from pre-flight checks through metadata cleanup.
- Perform removal via transactional steps: verify not active default, remove shims, delete directory with `remove_dir_all`, then purge metadata.
- Detect and warn if the JDK is currently configured as default or appears to be running.

### Non-Functional Requirement Details

N/A – Not applicable.

## Platform Considerations

### Unix

- Use OS advisory locks with lock files inside `$KOPI_HOME/locks/`.
- Ensure removal handles case-sensitive filesystems and permission nuances.

### Windows

- Handle Windows-specific file locking behaviors when deleting directories; retry with backoff if handles are temporarily held.
- Lock files reside in `%KOPI_HOME%\locks\` with owner-only ACLs.

### Cross-Platform

- Maintain consistent lock naming and metadata cleanup steps across operating systems.
- Normalize path separators via `std::path` utilities.

## Risks & Mitigation

| Risk                        | Impact | Likelihood | Mitigation                             | Validation                      |
| --------------------------- | ------ | ---------- | -------------------------------------- | ------------------------------- |
| JDK in use during uninstall | High   | Medium     | Detect running processes; abort safely | Test with active Java processes |
| Partial deletion on crash   | Medium | Low        | Stage deletion steps; resume safely    | Kill process during uninstall   |
| Lock file orphaned          | Low    | Low        | Rely on OS cleanup and startup sweep   | Monitor lock directory          |

## Implementation Notes

- Refuse to uninstall if the target JDK is currently the active shim unless forced with explicit flag.
- Provide detailed logging for each cleanup phase to aid recovery if issues arise.
- Consider a two-phase delete (mark then delete) to support rollback on failure.
- Ensure any cached metadata referencing the JDK (e.g., global index) is updated atomically.

## External References

N/A – No external references

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd).
