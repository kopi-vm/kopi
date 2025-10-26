# FR-ui8x2 Process-Level Uninstallation Locking

## Metadata

- Type: Functional Requirement
- Status: Approved
  <!-- Draft: Under discussion | Approved: Ready for implementation | Rejected: Decision made not to pursue this requirement -->

## Links

- Prerequisite Requirements:
  - [FR-02uqo-installation-locking](../requirements/FR-02uqo-installation-locking.md)
  - [FR-gbsz6-lock-timeout-recovery](../requirements/FR-gbsz6-lock-timeout-recovery.md)
  - [NFR-g12ex-cross-platform-compatibility](../requirements/NFR-g12ex-cross-platform-compatibility.md)
  - [NFR-vcxp8-lock-cleanup-reliability](../requirements/NFR-vcxp8-lock-cleanup-reliability.md)
- Dependent Requirements:
  - [FR-rxelv-file-in-use-detection](../requirements/FR-rxelv-file-in-use-detection.md)
- Related Tasks:
  - [T-98zsb-uninstallation-locking](../tasks/T-98zsb-uninstallation-locking/README.md)
  - [T-s2g7h-active-use-detection](../tasks/T-s2g7h-active-use-detection/README.md)

## Requirement Statement

The system SHALL acquire an exclusive process-level lock for each JDK uninstallation so that removal steps execute atomically and do not conflict with concurrent installs or other uninstallations targeting the same coordinate.

## Rationale

Without uninstallation locks, concurrent operations may delete directories and metadata that other processes rely on, leading to partial removals, broken shims, and inconsistent Kopi state.

## User Story (if applicable)

As a Kopi user, I want uninstallation to guard against concurrent modifications so that removing a JDK never leaves behind partial directories or broken metadata.

## Acceptance Criteria

- [ ] Acquire the shared lock file (`$KOPI_HOME/locks/{vendor}-{version}-{os}-{arch}.lock`) before any uninstall steps run.
- [ ] While the lock is held, installations or uninstallations for the same coordinate block or time out according to FR-gbsz6 rules.
- [ ] Successful uninstallation removes the JDK directory, shims, and metadata atomically; failures roll back partial work.
- [ ] Processes attempting to install the locked coordinate receive consistent blocking or timeout behaviour.
- [ ] Post-uninstallation verification confirms filesystem artefacts and metadata entries are fully removed.

## Technical Details (if applicable)

### Functional Requirement Details

- Share the lock acquisition helper used by installation so telemetry, messages, and fallback behaviour remain consistent.
- Execute removal in discrete phases: verify coordinate not active default, remove shims, delete directory, purge metadata, refresh cache.
- Abort with a descriptive warning if the JDK is running or active; rely on FR-rxelv to surface in-use binaries.

### Non-Functional Requirement Details

N/A – No additional non-functional constraints beyond related NFRs.

## Platform Considerations

### Unix

- Use advisory locks stored under `$KOPI_HOME/locks/`; ensure deletion handles case-sensitive filesystems and permission nuances.

### Windows

- Account for NTFS semantics when removing directories; retry deletions with backoff if handles remain open temporarily.

### Cross-Platform

- Normalise lock filenames using shared path helpers and maintain identical metadata cleanup steps across platforms.

## Risks & Mitigation

| Risk                        | Impact | Likelihood | Mitigation                               | Validation                         |
| --------------------------- | ------ | ---------- | ---------------------------------------- | ---------------------------------- |
| JDK in use during uninstall | High   | Medium     | Detect running processes; abort safely   | Integration tests with active Java |
| Partial deletion on crash   | Medium | Low        | Stage deletion steps; resume safely      | Kill process mid-uninstall         |
| Lock file orphaned          | Low    | Low        | Rely on kernel cleanup; sweep on startup | Monitor lock directory in tests    |

## Implementation Notes

- Refuse to uninstall if the target JDK is the active default unless forced via explicit flag.
- Emit detailed logging for each cleanup phase to aid recovery if failures occur.
- Consider a two-phase delete (mark then delete) to support rollback on failure scenarios.

## External References

N/A – No external references.

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd) in the templates README.
