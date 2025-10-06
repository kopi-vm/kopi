# FS2 Dependency Retirement Implementation Plan

## Metadata

- Type: Implementation Plan
- Status: Not Started

## Links

- Related Requirements:
  - [FR-x63pa-disk-space-telemetry](../../requirements/FR-x63pa-disk-space-telemetry.md)
  - [FR-rxelv-file-in-use-detection](../../requirements/FR-rxelv-file-in-use-detection.md)
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)

## Overview

Retire the `fs2` dependency by replacing disk space and file lock helpers with `sysinfo` and the Rust 1.89.0 standard library. The plan preserves doctor diagnostics, uninstall safeguards, and meets ADR-8mnaz guidance while reducing supply-chain exposure.

## Success Metrics

- [ ] `fs2` removed from `Cargo.toml` and `Cargo.lock`.
- [ ] Disk space checks complete within 50ms p95 on supported desktop platforms.
- [ ] File-in-use detection reports the same warnings as current implementation across Windows and Unix.
- [ ] All existing tests pass; no regressions in doctor or uninstall flows.

## Scope

- Goal: Replace `fs2` functionality, update tests/documentation, and validate behaviour on macOS, Linux, and Windows.
- Non-Goals: Broader refactors unrelated to disk checks or locking, removal of `sysinfo` from other subsystems.
- Assumptions: Rust toolchain remains 1.89.0 or newer; `sysinfo` stays on v0.31 without breaking API changes.
- Constraints: No `unsafe` code; keep user-facing English messages stable.

## ADR & Legacy Alignment

- [x] Confirmed ADR-8mnaz guides locking decisions.
- [ ] Track legacy references to `fs2` in archived docs and mark them as historical in Phase 3 documentation tasks.

## Plan Summary

- Phase 1 - Disk probe foundation
- Phase 2 - Locking migration
- Phase 3 - Cleanup & verification

---

## Phase 1: Disk Probe Foundation

### Goal

Refactor disk space checks to use a reusable helper backed by `sysinfo`, meeting FR-x63pa.

### Inputs

- Documentation:
  - `docs/analysis/AN-l19pi-fs2-dependency-retirement.md` – replacement rationale
  - `docs/tasks/T-9r1su-fs2-dependency-retirement/design.md` – design guidance
- Source Code to Modify:
  - `src/storage/disk_space.rs` – disk space checker
  - `src/doctor/checks/jdks.rs` – doctor disk check output
- Dependencies:
  - Internal: `crate::error`, `crate::doctor::format_size`
  - External crates: `sysinfo` – disk metrics

### Tasks

- [ ] **`Helper creation`**
  - [ ] Implement `SysinfoDiskProbe` (or equivalent) to expose available bytes for a path.
  - [ ] Add targeted unit tests using captured `sysinfo` snapshots per platform.
- [ ] **`Integration updates`**
  - [ ] Wire `DiskSpaceChecker` to use the probe and remove direct `fs2` calls.
  - [ ] Update doctor `jdks` check to reuse the helper and refresh disks efficiently.

### Deliverables

- New helper module with unit tests.
- Updated disk space logic without `fs2` imports.

### Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet storage::disk_space
cargo test --lib --quiet doctor::checks::jdks
```

### Acceptance Criteria (Phase Gate)

- Probe returns correct values for Linux, macOS, Windows sample data.
- Doctor output remains unchanged in golden snapshot tests (update if necessary).

### Rollback/Fallback

- Revert helper to previous commit and restore `fs2::available_space` usage if probe fails validation.

---

## Phase 2: Locking Migration

### Phase 2 Goal

Replace `fs2::FileExt` usage with standard library locking while keeping warnings and behaviour intact to satisfy FR-rxelv.

### Phase 2 Inputs

- Dependencies:
  - Phase 1: Disk probe ready to avoid cross-branch conflicts.
- Source Code to Modify:
  - `src/platform/file_ops.rs` – file-in-use detection (Windows and Unix sections)
  - `src/storage/disk_space.rs` (final cleanup of imports, if any)

### Phase 2 Tasks

- [ ] **`Adapter implementation`**
  - [ ] Introduce `StdFileLockAdapter` (or equivalent) encapsulating `try_lock_exclusive` and `unlock`.
  - [ ] Add RAII guard/tests ensuring locks release automatically on drop.
- [ ] **`Function migration`**
  - [ ] Update both platform variants of `check_files_in_use` to use the adapter.
  - [ ] Expand tests to simulate locked files via spawned threads/processes.

### Phase 2 Deliverables

- Standard-library-based locking implementation with test coverage.

### Phase 2 Verification

```bash
cargo test --lib --quiet platform::file_ops
cargo test --lib --quiet -- --ignored file_in_use_windows   # run on Windows CI
cargo test --lib --quiet -- --ignored file_in_use_unix      # run locally or in CI matrix
```

### Phase 2 Acceptance Criteria

- Tests capture locked/unlocked scenarios on each platform.
- Doctor/uninstall warnings match historical phrasing.

### Phase 2 Rollback/Fallback

- Temporarily gate new implementation behind a feature flag to fall back to `fs2` if standard library behaviour diverges during testing.

---

## Phase 3: Cleanup & Verification

### Phase 3 Goal

Remove dependency artifacts, update documentation, and confirm regressions are absent.

### Phase 3 Inputs

- Dependencies:
  - Phases 1 and 2 complete.
- Source Code to Modify:
  - `Cargo.toml`, `Cargo.lock`
  - `docs/tasks/T-9r1su-fs2-dependency-retirement/README.md`
  - `docs/error_handling.md` (if messaging adjustments)

### Phase 3 Tasks

- [ ] **`Dependency cleanup`**
  - [ ] Remove `fs2` entries from manifests and regenerate lockfile.
  - [ ] Run `cargo metadata` to verify dependency graph.
- [ ] **`Documentation & traceability`**
  - [ ] Update docs mentioning `fs2`, including archived references with historical context notes.
  - [ ] Regenerate trace matrix with `bun scripts/trace-status.ts --write` if documentation files change.
- [ ] **`Verification sweep`**
  - [ ] Execute required Rust workflows (`cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test --lib --quiet`).
  - [ ] Capture manual verification checklist results for macOS, Linux, and Windows.

### Phase 3 Deliverables

- Updated manifests without `fs2`.
- Documentation aligned with new behaviour.
- Verification logs and manual test notes.

### Phase 3 Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet
bun format
bun lint
bun scripts/trace-status.ts --check
```

### Phase 3 Acceptance Criteria

- No references to `fs2` remain in the repository.
- Traceability script reports no missing links or placeholders.
- Manual platform spot checks recorded and attached to task.

### Phase 3 Rollback/Fallback

- Re-add `fs2` dependency if critical regressions emerge; document rollback decision in task log.

---

## Platform Matrix

### Unix

- Validate disk probe on ext4 and APFS; ensure permission errors for locks are handled gracefully.

### Windows

- Confirm locking catches `java.exe` in use and handles UNC paths with warnings.

### Filesystem

- Document behaviour on network shares; warn users when accuracy cannot be guaranteed.

---

## Dependencies

### External Crates

- `sysinfo` – disk statistics (already present).

### Internal Modules

- `src/storage/disk_space.rs` – disk checks
- `src/platform/file_ops.rs` – lock detection
- `src/doctor/checks/jdks.rs` – doctor reporting

---

## Risks & Mitigations

1. Risk: `sysinfo` disk data lags behind real-time state.
   - Mitigation: Refresh scopes before sampling and note limitations in docs.
   - Validation: Unit tests with updated snapshots; manual verification on real disks.
   - Fallback: Provide optional direct OS calls if accuracy insufficient.

2. Risk: Standard library locks behave inconsistently across platforms.
   - Mitigation: Expand test matrix and add manual verification steps.
   - Validation: Windows/macOS smoke tests plus CI gating.
   - Fallback: Feature flag fallback to `fs2` until discrepancies resolved.

---

## Documentation & Change Management

### CLI/Behavior Changes

- None expected; document subtle differences in doctor appendix if observed.

### ADR Impact

- No new ADRs. Ensure ADR-8mnaz references are updated with completion status once implemented.

---

## Implementation Guidelines

### Error Handling

- Continue using `KopiError::DiskSpaceError` and contextual `SystemError` messages.
- Provide actionable instructions when disk data unavailable or locks fail.

### Naming & Structure

- Use descriptive helper names (`SysinfoDiskProbe`, `StdFileLockAdapter`); avoid "manager" or "util" suffixes.
- Prefer small functions over stateful structs unless trait implementations are needed.

### Safety & Clarity

- No `unsafe` blocks; rely on standard library and safe crate APIs.
- Prioritize readability and maintainability over micro-optimizations.

---

## Definition of Done

- [ ] `cargo check`
- [ ] `cargo fmt`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --lib --quiet`
- [ ] Integration/perf/bench (as applicable): `cargo it`, `cargo perf`, `cargo bench`
- [ ] Documentation updates completed in repo and external docs
- [ ] Traceability regenerated with no missing links
- [ ] Platform verification recorded (Linux, macOS, Windows)
- [ ] No `unsafe` usage; naming guidelines satisfied

---

## Status Tracking

- Not Started: Work hasn't begun
- Phase X In Progress: Currently working on a specific phase
- Phase X Completed: Phase finished; moving to next
- Blocked: Waiting on external dependency
- Under Review: Implementation complete; awaiting review
- Completed: All phases done and verified

---

## External References

- [sysinfo crate](https://docs.rs/sysinfo/) – Disk statistics API
- [Rust 1.89.0 release notes](https://blog.rust-lang.org/2025/08/07/Rust-1.89.0/index.html) – File locking stabilization

## Open Questions

- [ ] Should the probe reuse the existing `System` instance from shell detection to reduce overhead? → Evaluate during Phase 1 implementation.
- [ ] What manual verification steps are required on Windows for UNC paths? → Capture during Phase 3 documentation.
- [ ] Do we need feature flags to roll back the locking change? → Decide after Phase 2 validation.

---

## Visual/UI Reference

```text
Doctor output example:
JDKs using 3.2 GB, 1.4 GB available
  - temurin-21.0.1: 450.3 MB
  - corretto-17.0.9: 330.2 MB
```

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../../templates/README.md#plan-template-planmd) in the templates README.
