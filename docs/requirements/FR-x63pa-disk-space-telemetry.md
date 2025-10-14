# FR-x63pa Disk Space Reporting Without fs2

## Metadata

- Type: Functional Requirement
- Status: Approved
  <!-- Draft: Under discussion | Approved: Ready for implementation | Rejected: Decision made not to pursue this requirement -->

## Links

- Prerequisite Requirements:
  - [FR-v7ql4-cache-locking](../requirements/FR-v7ql4-cache-locking.md)
- Dependent Requirements:
  - N/A – No dependent requirements recorded
- Related Tasks:
  - [T-9r1su-fs2-dependency-retirement](../tasks/T-9r1su-fs2-dependency-retirement/README.md)

## Requirement Statement

Kopi SHALL calculate and report available disk space for directories involved in JDK management using only first-party crates and standard library APIs, eliminating any dependency on the `fs2` crate.

## Rationale

Removing `fs2` reduces supply-chain exposure while preserving actionable diagnostics for installs and doctor checks. Reusing existing dependencies prevents binary size regressions and simplifies maintenance.

## User Story (if applicable)

As a Kopi user running doctor or installation flows, I want accurate free-space guidance so that I can proactively clean up storage and avoid failed operations.

## Acceptance Criteria

- [ ] Disk space checks during install, cache validation, and doctor output derive their figures from first-party APIs (e.g., `sysinfo`) rather than `fs2`.
- [ ] Doctor `jdks` check renders human-readable strings (for example, `2.4 GB available`) consistently across Linux, macOS, and Windows when free space differs by more than 1 MB.
- [ ] Failure to obtain disk data emits an actionable warning without referencing `fs2`, instructing users on manual verification.
- [ ] Automated regression coverage validates parsing of representative `sysinfo` disk snapshots for each supported platform.

## Technical Details (if applicable)

### Functional Requirement Details

- Refresh disk metadata immediately before sampling to minimise stale readings.
- Map the target directory to its underlying mount point when multiple disks are present.
- Reuse Kopi's formatting helpers to convert bytes to human-readable units.

### Non-Functional Requirement Details

N/A – Performance expectations captured under related NFRs.

## Platform Considerations

### Unix

- Use `sysinfo` (or equivalent native approach) to read `available_space`; validate behaviour on APFS, ext4, and Btrfs.

### Windows

- Ensure the chosen API returns accurate values for NTFS volumes and provides informative warnings for UNC paths when data is unavailable.

### Cross-Platform

- Guarantee consistent units and rounding rules across platforms; discrepancies greater than 1% require documentation.

## Risks & Mitigation

| Risk                                               | Impact | Likelihood | Mitigation                                                     | Validation                                  |
| -------------------------------------------------- | ------ | ---------- | -------------------------------------------------------------- | ------------------------------------------- |
| Disk refresh costs degrade CLI responsiveness      | Medium | Medium     | Limit refresh scope to relevant disks and benchmark execution. | Add timing assertions to regression tests.  |
| API fails for network-mounted directories          | Medium | Medium     | Detect UNC/NFS mounts and emit actionable warnings.            | Manual verification checklist in task plan. |
| Data formatting diverges from historical doctor UI | Low    | Low        | Reuse existing formatting helpers with snapshot tests.         | Update unit tests in `disk_space.rs`.       |

## Implementation Notes

- Extend `DiskSpaceChecker` to depend on a helper/trait that abstracts disk queries for easier testing.
- Store canned JSON/struct dumps from `sysinfo` in unit tests to validate parsing without live system calls.
- Update documentation referencing `fs2` to explain the new measurement approach in both internal and user-facing docs.

## External References

- [sysinfo crate documentation](https://docs.rs/sysinfo/) – Disk and filesystem statistics

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd) in the templates README.
