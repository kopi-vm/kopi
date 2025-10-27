# FR-rxelv File-In-Use Detection Without fs2

## Metadata

- Type: Functional Requirement
- Status: Approved
  <!-- Draft: Under discussion | Approved: Ready for implementation | Rejected: Decision made not to pursue this requirement -->

## Links

- Prerequisite Requirements:
  - [FR-02uqo-installation-locking](../requirements/FR-02uqo-installation-locking.md)
  - [FR-ui8x2-uninstallation-locking](../requirements/FR-ui8x2-uninstallation-locking.md)
- Dependent Requirements:
  - N/A – No dependent requirements recorded
- Related Tasks:
  - [T-9r1su-fs2-dependency-retirement](../tasks/T-9r1su-fs2-dependency-retirement/README.md)
  - [T-q5f2p-process-activity-detection](../tasks/T-q5f2p-process-activity-detection/README.md)
  - [T-wm2zx-winapi-to-windows-migration](../tasks/T-wm2zx-winapi-to-windows-migration/README.md)

## Requirement Statement

Kopi SHALL detect critical JDK binaries that remain in use by attempting non-blocking exclusive locks using only standard library primitives, eliminating the `fs2` crate while preserving existing diagnostics.

## Rationale

Adopting `std::fs::File` locking aligns the file-in-use check with Kopi's broader locking strategy, removes an unmaintained dependency, and improves reliability by relying on kernel-managed locks.

## User Story (if applicable)

As a Kopi user uninstalling or updating a JDK, I want the CLI to warn me when java-related executables are still running so that I can close them before retrying the operation.

## Acceptance Criteria

- [ ] `check_files_in_use` uses standard library locking (`std::fs::File::{try_lock_exclusive, unlock}`) without importing `fs2`.
- [ ] Windows and Unix variants return the same warning messages as the prior implementation when locks cannot be acquired.
- [ ] Automated tests simulate locked and unlocked files across supported platforms, covering missing file and permission-denied scenarios.
- [ ] Manual verification checklist documents Windows Explorer and macOS Activity Monitor scenarios for in-use binaries with actionable guidance.

## Technical Details (if applicable)

### Functional Requirement Details

- Attempt to open critical binaries with shared read access before exclusive locking to avoid permission errors.
- Release file handles explicitly after successful lock acquisition; prefer RAII wrappers to guarantee unlock on drop.
- Surface lock acquisition failures via existing `KopiError` variants with clear, English messaging.

### Non-Functional Requirement Details

N/A – Behavioural constraints captured by related NFRs.

## Platform Considerations

### Unix

- Use `try_lock_exclusive` on descriptors; validate behaviour on ext4, APFS, and other common filesystems via integration tests.

### Windows

- Ensure `try_lock_exclusive` correctly wraps `LockFileEx`; confirm compatibility with NTFS semantics and that handles close before returning.

### Cross-Platform

- Maintain the critical binary list (`java`, `javac`, `javaw`, `jar`) and keep warning text identical across platforms.
- Document known limitations for network filesystems where locks may succeed without guaranteeing exclusivity.

## Risks & Mitigation

| Risk                                             | Impact | Likelihood | Mitigation                                                       | Validation                      |
| ------------------------------------------------ | ------ | ---------- | ---------------------------------------------------------------- | ------------------------------- |
| Standard library lock semantics differ subtly    | High   | Medium     | Prototype cross-platform behaviour and capture findings in tests | Platform smoke tests in CI      |
| Lock attempts crash when files missing or unread | Medium | Medium     | Gracefully handle `NotFound` and `PermissionDenied` errors       | Unit tests covering edge cases  |
| Unlock omitted causing lingering locks           | Low    | Low        | Use RAII helpers or immediately drop handles after unlocking     | Code review checklist and tests |

## Implementation Notes

- Introduce a helper (`try_lock_exclusive`) to wrap platform differences and aid testing.
- Emit debug logs indicating when locks fail for follow-up analysis.
- Update user documentation (external repo) to explain how to resolve in-use warnings.

## External References

- [Rust std::fs::File locking documentation](https://doc.rust-lang.org/std/fs/struct.File.html)

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd) in the templates README.
