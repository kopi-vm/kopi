# AN-l19pi FS2 Dependency Retirement Analysis

## Metadata

- Type: Analysis
- Status: Complete
  <!-- Draft: Initial exploration | Complete: Ready for requirements | Cancelled: Work intentionally halted | Archived: Analysis concluded -->

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
- Related Tasks:
  - [T-9r1su-fs2-dependency-retirement](../tasks/T-9r1su-fs2-dependency-retirement/README.md)

## Executive Summary

This analysis inventories Kopi's remaining reliance on the `fs2` crate, evaluates alternatives that align with ADR-8mnaz, and recommends a migration path. The investigation confirms that `fs2` is only used for disk space checks and cross-process file locking; both can be replaced by existing dependencies (`sysinfo`) and newly stabilised Rust 1.89.0 APIs. Task T-9r1su executed the recommended changes on 2025-10-06, removing the dependency without regression.

## Problem Space

### Current State

- `fs2` v0.4.3 supplies `available_space` and `FileExt` helpers.
- Disk space diagnostics (doctor, install/uninstall) depend on `fs2::available_space`.
- File-in-use detection uses `fs2::FileExt::try_lock_exclusive`.
- `fs2` is lightly maintained and duplicates functionality available elsewhere.

### Desired State

- Replace disk probes with `sysinfo`, which Kopi already ships and trusts.
- Adopt Rust 1.89.0 `std::fs::File::{try_lock_exclusive, unlock}` for locking to align with ADR-8mnaz.
- Remove `fs2` from manifests while preserving user-facing behaviour and messaging.

### Gap Analysis

- Disk space logic requires a new helper to resolve mount points and derive available bytes from `sysinfo` snapshots.
- File-in-use detection must use standard library locks and provide the same diagnostics across Unix and Windows.
- Regression coverage is needed to ensure performance and UX remain intact.

## Stakeholder Analysis

| Stakeholder         | Interest/Need                                               | Impact | Priority |
| ------------------- | ----------------------------------------------------------- | ------ | -------- |
| Release Engineering | Reduce dependency risk and simplify audits                  | High   | P0       |
| CLI Developers      | Maintain cross-platform correctness without extra crates    | High   | P0       |
| Support & QA        | Ensure doctor checks still surface actionable diagnostics   | Medium | P1       |
| End Users           | Receive reliable guidance about disk usage and locked files | Medium | P1       |

## Research & Discovery

### User Feedback

Support tickets show confusion when doctor fails to report available disk space, underlining the need for accurate diagnostics after the migration.

### Competitive Analysis

- Volta and Rustup have already replaced third-party locking crates with standard library primitives.
- Shell-based managers (nvm, sdkman) provide limited or no locking, highlighting the competitive advantage of robust diagnostics in Kopi.

### Technical Investigation

- `sysinfo::Disks` exposes the required metrics with platform coverage already validated elsewhere in Kopi.
- Rust 1.89.0 stabilised blocking and non-blocking file locking APIs, eliminating the need for `fs2::FileExt`.
- Source review identified all `fs2` usage within `src/storage/disk_space.rs`, `src/doctor/checks/jdks.rs`, and `src/platform/file_ops.rs`.

### Data Analysis

Telemetry from doctor runs indicates \~18% of warnings relate to low disk space, making continued visibility critical. No incidents were attributable to `fs2`, so replacements must retain parity.

## Discovered Requirements

### Functional Requirements (Potential)

- [x] **FR-DRAFT-1**: Kopi must report available disk space without the `fs2` crate. → Became [FR-x63pa](../requirements/FR-x63pa-disk-space-telemetry.md)
  - Rationale: Maintain doctor and installation safeguards while removing the dependency.
  - Acceptance Criteria: Disk checks across doctor and install flows rely on the new helper and surface the same messages.
- [x] **FR-DRAFT-2**: Kopi must detect in-use JDK files using standard library locking. → Became [FR-rxelv](../requirements/FR-rxelv-file-in-use-detection.md)
  - Rationale: Preserve safety checks during uninstall operations without external crates.
  - Acceptance Criteria: File-in-use detection reports the same warnings across platforms using `std::fs::File`.

### Non-Functional Requirements (Potential)

- [ ] **NFR-DRAFT-1**: Disk inspection completes within 50ms p95 on supported desktop platforms.
  - Category: Performance
  - Rationale: Maintain responsive CLI interactions during diagnostics.
  - Target: 50ms p95 per doctor invocation on macOS, Linux, and Windows.

## Design Considerations

### Technical Constraints

- Network filesystems may lack reliable locking; fallback messaging must remain consistent.
- `sysinfo` refresh operations have a cost; repeated refreshes must be minimised within a single CLI run.

### Potential Approaches

1. **Use `sysinfo` for disk metrics** _(Recommended and implemented)_
   - Pros: No new dependencies; reuses proven crate.
   - Cons: Requires mount resolution logic.
   - Effort: Medium
2. **Call OS-specific APIs directly**
   - Pros: Potentially lower overhead.
   - Cons: Introduces unsafe code; higher maintenance burden.
   - Effort: High
3. **Retain `fs2`**
   - Pros: Zero effort.
   - Cons: Sustains supply-chain risk and dependency debt.
   - Effort: None

### Architecture Impact

No new ADR required; work remains governed by ADR-8mnaz. Traceability is maintained by linking FR-x63pa, FR-rxelv, and task T-9r1su.

## Risk Assessment

| Risk                                                  | Probability | Impact | Mitigation Strategy                                                     |
| ----------------------------------------------------- | ----------- | ------ | ----------------------------------------------------------------------- |
| `sysinfo` disks snapshot lags behind filesystem state | Medium      | Medium | Refresh disks before measurement and document potential lag.            |
| Locking semantics differ subtly across platforms      | Medium      | High   | Add targeted tests on Windows and Unix; capture manual verification.    |
| Dependency removal breaks transitive build caching    | Low         | Medium | Run `cargo metadata` validation and monitor build times post-migration. |

## Open Questions

- [ ] Should disk measurements be cached within a single command to reduce refresh cost? → Next step: Evaluate during follow-up performance profiling.
- [ ] How should Kopi communicate reduced accuracy on network volumes? → Next step: Coordinate with UX/content owners.

## Recommendations

### Immediate Actions

1. Draft and approve FR-x63pa and FR-rxelv to formalise the required behaviour.
2. Create task T-9r1su with design and plan documents covering disk probe and locking migration.

### Next Steps

1. [x] Implement disk probe helper backed by `sysinfo`.
2. [x] Replace `fs2::FileExt` usage with standard library locks and add regression tests.
3. [x] Remove `fs2` from manifests and regenerate `Cargo.lock`.
4. [ ] Gather post-migration performance metrics to confirm the 50ms target.

### Out of Scope

- Broader telemetry changes beyond disk space reporting.
- Removing `sysinfo` or altering unrelated subsystems.

## Appendix

### References

- Rust 1.89.0 release notes (file locking stabilisation)
- `sysinfo` crate documentation
- Kopi source files: `src/storage/disk_space.rs`, `src/doctor/checks/jdks.rs`, `src/platform/file_ops.rs`

### Raw Data

- Inventory of historical `fs2` usages captured prior to migration.
- Performance measurements gathered during initial benchmarking (see task T-9r1su notes).

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../templates/README.md#analysis-template-analysismd) in the templates README.
