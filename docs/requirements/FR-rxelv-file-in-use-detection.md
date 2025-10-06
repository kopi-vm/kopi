# File-In-Use Detection Without fs2

## Metadata

- Type: Functional Requirement
- Status: Proposed

## Links

- Related Analyses:
  - [AN-l19pi-fs2-dependency-retirement](../analysis/AN-l19pi-fs2-dependency-retirement.md)
- Prerequisite Requirements:
  - [FR-02uqo-installation-locking](../requirements/FR-02uqo-installation-locking.md)
  - [FR-ui8x2-uninstallation-locking](../requirements/FR-ui8x2-uninstallation-locking.md)
- Dependent Requirements:
  - N/A – Not yet assigned
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Related Tasks:
  - [T-9r1su-fs2-dependency-retirement](../tasks/T-9r1su-fs2-dependency-retirement/README.md)

## Requirement Statement

Kopi shall detect critical JDK binaries that remain in use by attempting non-blocking exclusive locks using only standard library primitives, removing any dependency on the `fs2` crate while preserving existing diagnostics.

## Rationale

Adopting `std::fs::File` locking aligns file-in-use detection with Kopi's broader locking strategy, eliminates an unmaintained dependency, and improves reliability through kernel-managed locks.

## User Story (if applicable)

As a Kopi user uninstalling or updating a JDK, I want the CLI to warn me when java-related executables are still running so that I can close them before retrying the operation.

## Acceptance Criteria

- [ ] The `check_files_in_use` function uses `std::fs::File::{try_lock_exclusive, unlock}` (or equivalent standard library APIs) without importing `fs2`.
- [ ] Windows and Unix variants of `check_files_in_use` return the same warning messages as the current implementation when locks cannot be acquired.
- [ ] Automated tests simulate locked and unlocked files, confirming error paths across supported platforms using platform-specific harnesses or mock helpers.
- [ ] Manual verification checklist documents Windows Explorer and macOS Activity Monitor scenarios where files remain open, ensuring messages remain actionable.

## Technical Details (if applicable)

### Functional Requirement Details

- Attempt to open critical binaries with shared read access before acquiring an exclusive lock to avoid permission errors.
- Release file handles explicitly after successful lock acquisition using the standard library API.
- Capture lock acquisition failures and surface them as `KopiError` variants consistent with existing error handling.

## Platform Considerations

### Unix

- Use `try_lock_exclusive` on descriptors; ensure behaviour on APFS and ext4 is validated through integration tests.

### Windows

- Leverage `try_lock_exclusive` to wrap `LockFileEx`; confirm compatibility with NTFS semantics and ensure handles close before returning.

### Cross-Platform

- Maintain the list of critical binaries (`java`, `javac`, `javaw`, `jar`) and ensure messages use consistent phrasing and formatting.

## Risks & Mitigation

| Risk                                             | Impact | Likelihood | Mitigation                                                       | Validation                                      |
| ------------------------------------------------ | ------ | ---------- | ---------------------------------------------------------------- | ----------------------------------------------- |
| Standard library lock semantics differ subtly    | High   | Medium     | Prototype cross-platform behaviour and capture findings in tests | Platform smoke tests in CI (planned in design). |
| Lock attempts crash when files missing or unread | Medium | Medium     | Gracefully handle `NotFound` and `PermissionDenied` errors       | Unit tests covering missing file scenarios.     |
| Unlocking omitted causing lingering locks        | Low    | Low        | Use RAII wrappers or drop handles immediately after unlocking    | Code review checklist plus unit assertions.     |

## Implementation Notes

- Introduce a small helper to wrap lock acquisition/release per platform, enabling dependency injection in tests.
- Document known limitations for network filesystems in the doctor output and user docs repository.
- Ensure telemetry or debug logging indicates when locks fail for later analysis.

## External References

- [Rust std::fs::File locking documentation](https://doc.rust-lang.org/std/fs/struct.File.html)

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd) in the templates README.
