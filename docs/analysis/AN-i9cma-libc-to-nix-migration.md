# AN-i9cma libc to nix Migration Analysis

## Metadata

- Type: Analysis
- Status: Cancelled
  <!-- Draft: Initial exploration | Complete: Ready for requirements | Cancelled: Work intentionally halted | Archived: Analysis concluded -->

## Links

- Related Analyses:
  - N/A – Standalone analysis
- Related Requirements:
  - [FR-02uqo-installation-locking](../requirements/FR-02uqo-installation-locking.md)
  - [FR-ui8x2-uninstallation-locking](../requirements/FR-ui8x2-uninstallation-locking.md)
  - [FR-v7ql4-cache-locking](../requirements/FR-v7ql4-cache-locking.md)
  - [FR-gbsz6-lock-timeout-recovery](../requirements/FR-gbsz6-lock-timeout-recovery.md)
  - [FR-c04js-lock-contention-feedback](../requirements/FR-c04js-lock-contention-feedback.md)
  - [FR-rxelv-file-in-use-detection](../requirements/FR-rxelv-file-in-use-detection.md)
  - [FR-x63pa-disk-space-telemetry](../requirements/FR-x63pa-disk-space-telemetry.md)
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Related Tasks:
  - [T-1pcd3-libc-to-nix](../tasks/T-1pcd3-libc-to-nix/README.md)

## Executive Summary

This analysis catalogues every direct `libc` crate usage in Kopi, evaluates migration options to the `nix` crate, and documents why the remaining references should be retained for now. Kopi already relies on `nix` for all unsafe operations; the only direct `libc` usages are type annotations for filesystem magic constants inside `src/platform/filesystem.rs`. Migrating those constants to `nix::sys::statfs::FsType` is blocked because five required constants are still missing in `nix` 0.29. The recommended outcome is to keep the lightweight `libc` dependency while monitoring upstream progress.

## Problem Space

### Current State

- `libc = "0.2"` is declared as a Unix-only dependency.
- `nix = { version = "0.29", features = ["fs"] }` is already in use for system calls.
- All direct `libc` usage is in `src/platform/filesystem.rs`:
  - Casting `FsType` to `libc::c_long` for comparisons.
  - Function parameters typed as `libc::c_long`.
  - Constant definitions for filesystem magic numbers.
- No `unsafe` blocks depend on `libc`; risk is limited to type aliasing.

### Desired State

- Remove redundant `libc` dependency when equivalent `nix` abstractions exist.
- Prefer `nix::sys::statfs::FsType` constants for better type safety.
- Maintain current platform coverage and behaviour.

### Gap Analysis

- `nix` 0.29 exposes eight of the thirteen filesystem magic constants Kopi needs.
- Missing constants: `ZFS_SUPER_MAGIC`, `CIFS_MAGIC_NUMBER`, `SMB2_MAGIC_NUMBER`, `VFAT_SUPER_MAGIC`, `EXFAT_SUPER_MAGIC`.
- Introducing custom wrappers for missing constants adds maintenance cost and fragile coupling to internal `nix` types.

## Stakeholder Analysis

| Stakeholder       | Interest/Need               | Impact | Priority |
| ----------------- | --------------------------- | ------ | -------- |
| Maintainers       | Reduce unsafe surface area  | Medium | P1       |
| Security auditors | Minimise FFI dependencies   | Low    | P2       |
| Users             | Stable filesystem detection | Low    | P2       |

## Research & Discovery

### User Feedback

No direct user feedback; the motivation is an internal hardening initiative.

### Competitive Analysis

Rust-based managers such as Volta and rustup already rely on the standard library or `nix` abstractions. Shell-based tools (nvm, sdkman) often accept best-effort safety; their experience reinforces that retaining `libc` solely for constants carries minimal user risk.

### Technical Investigation

- Reviewed `nix::sys::statfs::FsType` implementation: constants wrap `libc` values, confirming the transitive dependency on `libc` for underlying types.
- Audited `filesystem.rs` and verified all operations using `nix::sys::statfs::statfs()` wrappers; only type comparisons involve `libc`.
- Identified five filesystem constants absent from `nix` 0.29, blocking full migration.

### Data Analysis

Source audit confirms zero runtime bugs attributable to the existing `libc` usage. Telemetry does not show filesystem misclassification incidents linked to these constants.

## Discovered Requirements

### Functional Requirements (Potential)

- [ ] **FR-DRAFT-1**: Evaluate each `libc` usage during dependency upgrades to verify whether `nix` now exposes the required constants.
  - Rationale: Ensures Kopi removes `libc` promptly once upstream coverage exists.
  - Acceptance Criteria: Checklist item in release notes confirming manual review of filesystem constants.

### Non-Functional Requirements (Potential)

- [ ] **NFR-DRAFT-1**: Maintain zero regressions in filesystem detection while refactoring dependencies.
  - Category: Reliability
  - Rationale: Incorrect classification risks user confusion and misapplied behaviour.
  - Target: All existing tests covering filesystem detection remain green.

## Design Considerations

### Technical Constraints

- `FsType` is a tuple struct around `__fsword_t`, a `libc` type; custom constants must mirror internal representation.
- Upgrading to unreleased `nix` versions or forking `nix` introduces maintenance overhead.

### Potential Approaches

1. **Adopt `FsType` constants as they become available**
   - Pros: Zero custom wrappers; aligns with upstream support.
   - Cons: Requires ongoing monitoring and manual follow-up.
   - Effort: Low
2. **Define custom `FsType` constants locally**
   - Pros: Removes `libc` dependency immediately.
   - Cons: Couples to `nix` internals and risks divergence.
   - Effort: Medium
3. **Retain `libc` for missing constants** _(Recommended)_
   - Pros: Stable, minimal effort.
   - Cons: Keeps extra dependency.
   - Effort: Low

### Architecture Impact

No new ADRs required; future dependency upgrades should note progress in release engineering notes rather than architectural decisions.

## Risk Assessment

| Risk                                      | Probability | Impact | Mitigation Strategy                                     |
| ----------------------------------------- | ----------- | ------ | ------------------------------------------------------- |
| Custom wrappers diverge from `nix` types  | Medium      | Medium | Avoid custom wrappers; wait for upstream coverage.      |
| Missed upstream additions of constants    | Low         | Low    | Add release checklist to review `nix` changelog.        |
| Unintended removal of `libc` dependencies | Low         | Medium | Execute targeted tests before editing manifest entries. |

## Open Questions

- [ ] Should Kopi contribute missing constants to `nix` upstream? → Next step: Evaluate contributor guidelines and effort.
- [ ] Can we document a trigger for re-running this analysis (e.g., annual review)? → Next step: Add to release calendar checklist.

## Recommendations

### Immediate Actions

1. Document current justification for keeping `libc` and share with maintainers.
2. Update task T-1pcd3 to reflect suspended implementation pending upstream support.

### Next Steps

1. [ ] Monitor `nix` release notes for added filesystem constants.
2. [ ] Revisit migration if at least one missing constant lands upstream.
3. [ ] Consider upstream contributions if constants remain absent by mid-2026.
4. [ ] Create follow-up requirements once migration becomes feasible.

### Out of Scope

- Replacing `libc` in other crates or transitive dependencies.
- Reworking filesystem detection beyond type alias cleanup.

## Appendix

### References

- Rust `nix` crate documentation: `nix::sys::statfs`
- Kopi source file `src/platform/filesystem.rs`

### Raw Data

Inventory of filesystem magic constants (as of 2025-10-14):

- Covered by `nix`: EXT4, XFS, BTRFS, TMPFS, OVERLAYFS, NFS, MSDOS, SMB (legacy)
- Missing in `nix`: ZFS, CIFS, SMB2, VFAT, EXFAT

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../templates/README.md#analysis-template-analysismd) in the templates README.
