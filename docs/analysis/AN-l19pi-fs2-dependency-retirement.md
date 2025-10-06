# FS2 Dependency Retirement Analysis

## Metadata

- Type: Analysis
- Status: Active

## Links

- Related Analyses:
  - [AN-m9efc-concurrent-process-locking](../analysis/AN-m9efc-concurrent-process-locking.md)
- Related Requirements:
  - [FR-02uqo-installation-locking](../requirements/FR-02uqo-installation-locking.md)
  - [FR-ui8x2-uninstallation-locking](../requirements/FR-ui8x2-uninstallation-locking.md)
  - [FR-v7ql4-cache-locking](../requirements/FR-v7ql4-cache-locking.md)
  - [FR-x63pa-disk-space-telemetry](../requirements/FR-x63pa-disk-space-telemetry.md)
  - [FR-rxelv-file-in-use-detection](../requirements/FR-rxelv-file-in-use-detection.md)
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)

## Executive Summary

This analysis catalogues all remaining usages of the `fs2` crate inside Kopi, evaluates replacement options aligned with ADR-8mnaz, and recommends a migration plan that preserves existing functionality while improving supply-chain posture. Disk space checks and file-in-use detection currently rely on `fs2`; both responsibilities can be fulfilled via already-adopted dependencies (`sysinfo`) and the Rust 1.89.0 standard library. The proposed approach removes a maintenance liability without reducing platform coverage.

> Status update (2025-10-06): Task T-9r1su implemented the recommended migration. References to `fs2` in this document now describe the pre-migration state for historical context.

## Problem Space

### Current State

Kopi depends on `fs2` v0.4.3 for two core capabilities:

- Disk space inspection within `DiskSpaceChecker` and doctor diagnostics.
- Cross-platform file locking helpers (`FileExt`) used for JDK in-use detection.

The crate is lightly maintained, adds indirect dependencies, and duplicates functionality now available in the Rust standard library or first-party crates already in Kopi.

### Desired State

- Disk space checks reuse an actively supported crate such as `sysinfo`, which Kopi already ships for shell detection, providing consistent metrics across Linux, macOS, and Windows.
- File-in-use detection adopts `std::fs::File::{try_lock_exclusive, unlock}` so Kopi follows ADR-8mnaz guidance and reduces external surface area.
- The `fs2` dependency is removed from manifests, eliminating redundant supply-chain risk.

### Gap Analysis

The remaining gap is implementing replacements for `fs2::available_space` and `fs2::FileExt` traits, plus adding regression coverage to confirm behaviour parity. No architectural blockers exist because runtime prerequisites (Rust 1.89.0 and `sysinfo`) are already in use.

## Stakeholder Analysis

| Stakeholder         | Interest/Need                                               | Impact | Priority |
| ------------------- | ----------------------------------------------------------- | ------ | -------- |
| Release Engineering | Reduce dependency risk and simplify audits                  | High   | P0       |
| CLI Developers      | Maintain cross-platform correctness without extra crates    | High   | P0       |
| Support & QA        | Ensure doctor checks still surface actionable diagnostics   | Medium | P1       |
| End Users           | Receive reliable guidance about disk usage and locked files | Medium | P1       |

## Research & Discovery

### User Feedback

Support tickets highlight confusion when doctor cannot report available disk space on network paths; users expect actionable guidance when space is low.

### Competitive Analysis

- Volta migrated away from `fs2` once `std::fs::File` locking stabilised, using native advisory locks for concurrency.
- Rustup similarly relies on kernel-managed locks and internally calculates disk usage to inform cleanup recommendations.

### Technical Investigation

- `sysinfo::Disks` exposes per-mount statistics, including available space, without additional OS bindings.
- Rust 1.89.0 introduces `std::fs::File::try_lock_exclusive` and `unlock`, rendering `fs2::FileExt` redundant.
- Existing Kopi modules (`src/platform/shell.rs`) already initialise a long-lived `sysinfo::System`, demonstrating compatibility with shipped toolchains.

### Data Analysis

Temporary telemetry from doctor runs shows 18% of warnings relate to low disk space; continuing to surface this insight is critical for upgrade readiness.

## Discovered Requirements

### Functional Requirements (Potential)

- [x] **FR-DRAFT-1**: Kopi must report available disk space for directories relevant to JDK management without relying on `fs2`. → Now FR-x63pa
  - Rationale: Maintains doctor guidance while enabling dependency retirement.
  - Priority: P0
  - Acceptance Criteria: Disk checks in installation flows and doctor output display human-readable free space derived from `sysinfo` snapshots.

- [x] **FR-DRAFT-2**: Kopi must detect critical JDK executables that are still running using only standard library locks. → Now FR-rxelv
  - Rationale: Preserve safety checks without third-party locking abstractions.
  - Priority: P0
  - Acceptance Criteria: Doctor and uninstall flows flag in-use binaries by attempting `std::fs::File::try_lock_exclusive` and releasing the handle correctly on success.

### Non-Functional Requirements (Potential)

- [ ] **NFR-DRAFT-1**: Disk inspection must complete within 50ms on local SSDs. → Needs validation
  - Category: Performance
  - Target: 50ms p95 on macOS, Linux, Windows for doctor check runs.
  - Rationale: Preserve CLI responsiveness when collecting diagnostics.

## Design Considerations

### Technical Constraints

- Network-mounted paths may yield incomplete locking support; fallback messaging must remain consistent.
- `sysinfo` initializes lazily; repeated refresh calls incur cost and must be scoped carefully to avoid regressions.

### Potential Approaches

1. **Option A**: Use `sysinfo::System::refresh_disks_list` then query `available_space` for relevant mount.
   - Pros: Zero new dependencies, already packaged.
   - Cons: Requires mapping target paths to the correct disk snapshot.
   - Effort: Medium

2. **Option B**: Implement OS-specific `statvfs`/`GetDiskFreeSpaceEx` wrappers directly.
   - Pros: Potentially lower overhead, less ambient data gathering.
   - Cons: Duplicates battle-tested logic, adds unsafe FFI.
   - Effort: High

Option A is preferred because it aligns with existing crate usage and avoids unsafe code.

### Architecture Impact

No new ADRs required; work executes under ADR-8mnaz. Requirements FR-x63pa and FR-rxelv cover functional obligations.

## Risk Assessment

| Risk                                                 | Probability | Impact | Mitigation Strategy                                                     |
| ---------------------------------------------------- | ----------- | ------ | ----------------------------------------------------------------------- |
| `sysinfo` disk snapshots lag behind filesystem state | Medium      | Medium | Refresh disk data before measurement and document potential lag.        |
| Locking API differs subtly across platforms          | Medium      | High   | Add targeted tests on Windows and Unix, document manual verification.   |
| Dependency removal breaks transitive build caching   | Low         | Medium | Run `cargo metadata` verification and adjust docs if build size shifts. |

## Open Questions

- [ ] Do we need to cache disk measurements across a single CLI run to avoid refresh costs? → Next step: Prototype during design.
- [ ] Should doctor surface mount identifiers when disk space is low? → Next step: Capture user feedback post-implementation.

## Recommendations

### Immediate Actions

1. Draft FR-x63pa and FR-rxelv to formalize obligations.
2. Prepare design and plan documents within T-9r1su detailing migration strategy and validation steps.

### Next Steps

1. [x] Create formal requirements: FR-x63pa, FR-rxelv
2. [ ] Draft ADR for: N/A – covered by ADR-8mnaz
3. [ ] Create task for: Implementation follows existing task scope
4. [ ] Further investigation: Benchmark `sysinfo` refresh cost vs. direct OS calls

### Out of Scope

- Removing `sysinfo` itself or revisiting earlier shell detection work.
- Implementing broader telemetry adjustments beyond disk space reporting.

## Appendix

### References

- Rust 1.89.0 release notes on file locking support
- Sysinfo crate documentation for disk statistics

### Raw Data

- Doctor telemetry sample (2025-09-17): 312 runs, 56 disk space warnings, 9 lock conflicts.

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../templates/README.md#analysis-template-analysismd) in the templates README.
