# Disk Space Reporting Without fs2

## Metadata

- Type: Functional Requirement
- Status: Proposed

## Links

- Related Analyses:
  - [AN-l19pi-fs2-dependency-retirement](../analysis/AN-l19pi-fs2-dependency-retirement.md)
- Prerequisite Requirements:
  - [FR-v7ql4-cache-locking](../requirements/FR-v7ql4-cache-locking.md)
- Dependent Requirements:
  - N/A – Not yet assigned
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Related Tasks:
  - [T-9r1su-fs2-dependency-retirement](../tasks/T-9r1su-fs2-dependency-retirement/README.md)

## Requirement Statement

Kopi shall calculate and report available disk space for directories involved in JDK management using only first-party crates and standard library APIs, with no dependency on the `fs2` crate.

## Rationale

Removing `fs2` decreases supply-chain exposure while preserving actionable diagnostics for installs and doctor checks. Using existing dependencies prevents binary size regressions and keeps maintenance overhead low.

## User Story (if applicable)

As a Kopi user running doctor or installation flows, I want accurate free-space guidance so that I can proactively clean up storage and avoid failed operations.

## Acceptance Criteria

- [ ] Disk space checks used during install, cache validation, and doctor output derive their figures from APIs other than `fs2`.
- [ ] Doctor `jdks` check renders human-readable strings (e.g., `2.4 GB available`) consistent across Linux, macOS, and Windows when free space differs by more than 1MB.
- [ ] Failing to obtain disk data emits a warning message explaining the inability to measure space without mentioning `fs2` and instructs users about manual verification.
- [ ] Automated regression coverage validates parsing of representative `sysinfo` disk snapshots for each supported platform.

## Technical Details (if applicable)

### Functional Requirement Details

- Refresh disk metadata immediately before sampling to minimize stale readings.
- Map the target directory to its underlying mount point when multiple disks are present.
- Ensure conversions between bytes and human-readable units match existing formatting utilities in `disk_space.rs`.

## Platform Considerations

### Unix

- Use `sysinfo` (or equivalent first-party approach) to read `available_space` and confirm behavior on APFS, ext4, and Btrfs where test coverage exists.

### Windows

- Confirm `sysinfo` or alternative API returns accurate values for NTFS volumes and handle UNC paths by falling back to informative warnings.

### Cross-Platform

- Formatting should produce consistent units and rounding rules; discrepancies greater than 1% require documentation.

## Risks & Mitigation

| Risk                                               | Impact | Likelihood | Mitigation                                                     | Validation                                     |
| -------------------------------------------------- | ------ | ---------- | -------------------------------------------------------------- | ---------------------------------------------- |
| Disk refresh costs degrade CLI responsiveness      | Medium | Medium     | Limit refresh scope to relevant disks and benchmark execution. | Include timing assertions in regression tests. |
| API fails for network-mounted directories          | Medium | Medium     | Detect UNC/NFS mounts and emit actionable warnings.            | Manual verification checklist in plan.         |
| Data formatting diverges from historical doctor UI | Low    | Low        | Reuse existing formatting helpers with snapshot tests.         | Update unit tests in `disk_space.rs`.          |

## Implementation Notes

- Prefer augmenting `DiskSpaceChecker` to accept a trait or helper that abstracts disk queries for easier testing.
- Store canned JSON/struct dumps from `sysinfo` in tests to validate parsing without live system calls.
- Update documentation referencing `fs2` to explain the new measurement approach.

## External References

- [sysinfo crate documentation](https://docs.rs/sysinfo/) - Disk and filesystem statistics

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd) in the templates README.
