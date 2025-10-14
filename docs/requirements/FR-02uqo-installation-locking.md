# FR-02uqo Process-Level Installation Locking

## Metadata

- Type: Functional Requirement
- Status: Approved
  <!-- Draft: Under discussion | Approved: Ready for implementation | Rejected: Decision made not to pursue this requirement -->

## Links

- Prerequisite Requirements:
  - [NFR-g12ex-cross-platform-compatibility](../requirements/NFR-g12ex-cross-platform-compatibility.md)
  - [NFR-vcxp8-lock-cleanup-reliability](../requirements/NFR-vcxp8-lock-cleanup-reliability.md)
- Dependent Requirements:
  - [FR-gbsz6-lock-timeout-recovery](../requirements/FR-gbsz6-lock-timeout-recovery.md)
  - [FR-ui8x2-uninstallation-locking](../requirements/FR-ui8x2-uninstallation-locking.md)
  - [FR-v7ql4-cache-locking](../requirements/FR-v7ql4-cache-locking.md)
  - [FR-rxelv-file-in-use-detection](../requirements/FR-rxelv-file-in-use-detection.md)
- Related Tasks:
  - [T-5msmf-installation-locking](../tasks/T-5msmf-installation-locking/README.md)
  - [T-ec5ew-locking-foundation](../tasks/T-ec5ew-locking-foundation/README.md)

## Requirement Statement

The system SHALL acquire an exclusive process-level lock before performing JDK installation work so that two Kopi processes targeting the same canonical vendor-version-os-arch coordinate never mutate the installation concurrently.

## Rationale

Concurrent installation attempts corrupt shared directories, waste download bandwidth, and leave metadata inconsistent. Relying on process-level locks prevents these failures and improves user trust in Kopi when multiple terminals or automation pipelines run simultaneously.

## User Story (if applicable)

As a Kopi user, I want the tool to coordinate installations with an exclusive lock so that parallel commands do not corrupt my managed JDKs.

## Acceptance Criteria

- [ ] Exclusive lock acquisition prevents more than one process from installing the same canonical coordinate at a time.
- [ ] Lock keys derive from canonicalised coordinates after alias resolution, ensuring equivalent inputs share the same lock file.
- [ ] Locks release on both success and failure paths, returning the system to an unlocked state.
- [ ] OS-managed cleanup releases locks automatically when a process crashes or is terminated.
- [ ] Installations for different coordinates proceed in parallel without unnecessary blocking.

## Technical Details (if applicable)

### Functional Requirement Details

- Use `std::fs::File::lock_exclusive()` for blocking acquisition and `try_lock_exclusive()` for optional non-blocking flows.
- Place lock files at `$KOPI_HOME/locks/{vendor}-{version}-{os}-{arch}.lock` with canonicalised components.
- Acquire the lock before any filesystem mutations and hold it until the installation completes, rolls back, or aborts.
- Expose telemetry describing acquisition latency, contention, and fallback behaviour.

### Non-Functional Requirement Details

N/A – No additional non-functional constraints beyond related NFRs.

## Platform Considerations

### Unix

- Implement locking via the standard library (backed by `flock(2)`); ensure lock files use owner-only permissions.

### Windows

- Use `LockFileEx` through the standard library; store locks in `%KOPI_HOME%\locks\` and clean up via RAII guards.

### Cross-Platform

- Lock file naming must be identical across platforms after normalisation.
- Canonicalise coordinates using shared helper logic to avoid divergence between Windows and Unix paths.

## Risks & Mitigation

| Risk                            | Impact | Likelihood | Mitigation                                                     | Validation                                |
| ------------------------------- | ------ | ---------- | -------------------------------------------------------------- | ----------------------------------------- |
| Filesystem lacks advisory locks | High   | Low        | Detect and fall back to atomic staging                         | Test on network filesystems               |
| Lock file permissions incorrect | Medium | Medium     | Create with restricted umask and ACLs                          | Verify permissions in tests               |
| Stale lock appears after crash  | Low    | Low        | Rely on kernel cleanup; add startup sweep for legacy artefacts | Integration tests covering crash recovery |

## Implementation Notes

- Provide configurable blocking vs. timeout-driven behaviour via [FR-gbsz6](../requirements/FR-gbsz6-lock-timeout-recovery.md).
- Canonicalise coordinates after resolving aliases, architecture defaults, and vendor synonyms.
- Ensure logging captures acquisition, contention, and release events with actionable English messaging.

## External References

- [Rust std::fs::File locking API](https://doc.rust-lang.org/std/fs/struct.File.html)
- [Cargo locking behaviour](https://github.com/rust-lang/cargo/blob/master/src/cargo/util/flock.rs)

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd) in the templates README.
