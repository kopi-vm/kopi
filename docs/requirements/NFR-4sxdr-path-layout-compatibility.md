# NFR-4sxdr Kopi Path Layout Compatibility

## Metadata

- Type: Non-Functional Requirement
- Status: Draft
  <!-- Draft: Under discussion | Approved: Ready for implementation | Rejected: Decision made not to pursue this requirement -->

## Links

- Prerequisite Requirements:
  - N/A – None yet
- Dependent Requirements:
  - N/A – None yet
- Related Tasks:
  - [T-wn8p3-path-registry](../tasks/T-wn8p3-path-registry/README.md)
- Related Analyses:
  - [AN-wn8p3-path-registry-consolidation](../analysis/AN-wn8p3-path-registry-consolidation.md)

## Requirement Statement

Kopi SHALL preserve the effective on-disk layout of all directories and files under `$KOPI_HOME` when migrating to the consolidated paths module, ensuring that relative and absolute outputs remain identical across supported platforms.

## Rationale

Users automate workflows against Kopi’s directory structure (backups, CI cache warmups, custom shims). Any change in path naming or layout would break those workflows. Guaranteeing compatibility allows the refactor to ship without destabilising existing installations.

## User Story (if applicable)

The system shall maintain path compatibility so that scripts referencing `$KOPI_HOME/jdks`, `$KOPI_HOME/cache`, `$KOPI_HOME/locks`, and related artefacts continue working after the consolidation.

## Acceptance Criteria

- [ ] Snapshot tests or golden comparisons confirm that paths returned by the new helpers match the pre-refactor paths for all supported artefacts on Unix, Windows, and WSL environments.
- [ ] No migrations, renames, or directory deletions are required for existing Kopi installations when adopting the new module.
- [ ] Telemetry or logging (if enabled) shows zero instances of fallback behaviour triggered by missing legacy directories in beta rollout environments.
- [ ] Documentation and release notes explicitly state that on-disk paths are unchanged, with verification steps for operators.

## Technical Details (if applicable)

### Non-Functional Requirement Details

- Reliability: Provide automated regression coverage comparing generated paths before and after the consolidation using representative fixtures.
- Compatibility: Ensure sanitisation outputs remain byte-for-byte identical, including case normalisation and separator usage, for both POSIX and Windows filesystems.
- Usability: Developer documentation must include a checklist for validating compatibility in future refactors touching the same module.

## Platform Considerations

### Unix

- Tests MUST run against case-sensitive filesystems and ensure symlink-based shims remain untouched.

### Windows

- Validation MUST include NTFS paths with spaces and UNC prefixes to confirm helper behaviour and sanitisation outcomes.

### Cross-Platform

- Continuous integration MUST exercise the compatibility suite on at least one Unix runner and one Windows runner before marking this requirement complete.

## Risks & Mitigation

| Risk                                                  | Impact | Likelihood | Mitigation                                                                       | Validation                                     |
| ----------------------------------------------------- | ------ | ---------- | -------------------------------------------------------------------------------- | ---------------------------------------------- |
| Hidden consumer relies on undocumented directory name | High   | Medium     | Publish release notes and offer configurable aliases if issues emerge            | Support feedback monitoring post-release       |
| Platform-specific slug differences surface            | Medium | Medium     | Add cross-platform tests comparing slugs; review sanitisation for Windows rules  | Unit + integration tests with known edge cases |
| Duplicate directory creation causes performance hit   | Low    | Medium     | Cache directory existence checks or reuse `std::fs::create_dir_all` idempotently | Benchmark during testing                       |

## Implementation Notes

- Capture baseline snapshots before refactor to use as regression targets.
- Engage documentation team to verify compatibility messaging aligns with user expectations.
- Consider feature flagging the new module for staged rollout if unexpected compatibility issues arise.
- Coordinate fulfilment with [FR-hq1ns-canonical-path-registry](../requirements/FR-hq1ns-canonical-path-registry.md) to ensure path APIs and compatibility tests evolve together.

## External References

- N/A – No external references.

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd) in the templates README.
