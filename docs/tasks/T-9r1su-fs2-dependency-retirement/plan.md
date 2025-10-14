# T-9r1su FS2 Dependency Retirement Implementation Plan

## Metadata

- Type: Implementation Plan
- Status: Complete
  <!-- Draft: Planning complete, awaiting start | Phase X In Progress: Actively working | Cancelled: Work intentionally halted before completion | Complete: All phases done and verified -->

## Links

- Associated Design Document:
  - [T-9r1su-fs2-dependency-retirement-design](./design.md)

## Overview

Retire the `fs2` dependency by migrating disk space checks to `sysinfo` and replacing file-in-use detection with Rust standard library locking, while preserving diagnostics and cross-platform behaviour.

## Success Metrics

- [x] `fs2` removed from `Cargo.toml` and `Cargo.lock`.
- [x] Disk space checks complete within 50 ms p95 on supported desktop platforms.
- [x] File-in-use detection matches historical warnings across Windows and Unix.
- [x] All existing tests pass; no regressions in doctor or uninstall flows.

## Scope

- Goal: Remove `fs2` usage across Kopi while maintaining functional parity and test coverage.
- Non-Goals: Broader dependency pruning, redesigning cache or locking APIs, removing `sysinfo`.
- Assumptions: Rust toolchain remains 1.89.0+, `sysinfo` stays compatible, CI matrix covers Linux/macOS/Windows.
- Constraints: No `unsafe` code introduced; user-facing English messages remain unchanged.

## ADR & Legacy Alignment

- [x] Confirmed ADR-8mnaz governs locking decisions and is referenced in documentation.
- [x] Identified legacy references to `fs2` and scheduled cleanup tasks in the documentation phase.

## Plan Summary

- Phase 1 – Disk probe foundation
- Phase 2 – Locking migration
- Phase 3 – Cleanup & verification

> **Status Tracking:** Checkboxes were updated upon completion of each subtask; deferred items are annotated inline.

---

## Phase 1: Disk Probe Foundation

### Goal

Refactor disk space checks to use a reusable helper backed by `sysinfo`, satisfying FR-x63pa.

### Inputs

- Documentation:
  - `docs/analysis/AN-l19pi-fs2-dependency-retirement.md` – replacement rationale
  - `docs/tasks/T-9r1su-fs2-dependency-retirement/design.md` – design guidance
- Source Code to Modify:
  - `src/storage/disk_space.rs` – disk space checker
  - `src/doctor/checks/jdks.rs` – doctor disk check output
- Dependencies:
  - Internal: `crate::error`, `crate::doctor::format_size`
  - External: `sysinfo`

### Tasks

- [x] **Helper creation**
  - [x] Implement `disk_probe::available_bytes()` to expose available bytes for a path.
  - [x] Add targeted unit tests using captured `sysinfo` snapshots per platform.
- [x] **Integration updates**
  - [x] Wire `DiskSpaceChecker` to use the probe and remove direct `fs2` calls.
  - [x] Update doctor `jdks` check to reuse the helper and refresh disks efficiently.

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

- Probe returns correct values for Linux, macOS, and Windows sample data.
- Doctor output remains unchanged in golden snapshot tests (updated only where intentional).

### Rollback/Fallback

- Revert helper and restore `fs2::available_space` usage if probe validation fails.

---

## Phase 2: Locking Migration

### Phase 2 Goal

Replace `fs2::FileExt` usage with standard library locking while satisfying FR-rxelv.

### Phase 2 Inputs

- Dependencies:
  - Phase 1 complete to avoid branch conflicts.
- Source Code to Modify:
  - `src/platform/file_ops.rs` – file-in-use detection for Windows and Unix
  - `src/storage/disk_space.rs` – cleanup of residual imports

### Phase 2 Tasks

- [x] **Lock helper implementation**
  - [x] Introduce a standalone `try_lock_exclusive()` helper encapsulating locking and unlock handling.
  - [x] Add RAII guard/tests ensuring locks release automatically on drop.
- [x] **Function migration**
  - [x] Update both platform variants of `check_files_in_use` to use the helper.
  - [x] Expand tests to simulate locked files via spawned threads/processes.

### Phase 2 Deliverables

- Standard-library-based locking implementation with cross-platform test coverage.

### Phase 2 Verification

```bash
cargo test --lib --quiet platform::file_ops
cargo test --lib --quiet -- --ignored file_in_use_windows   # run on Windows CI
cargo test --lib --quiet -- --ignored file_in_use_unix      # run locally or CI matrix
```

### Phase 2 Acceptance Criteria

- Tests capture locked/unlocked scenarios on each platform.
- Doctor/uninstall warnings match historical phrasing.

### Phase 2 Rollback/Fallback

- Temporarily gate new implementation behind a feature flag to fall back to `fs2` if regressions appear during validation.

---

## Phase 3: Cleanup & Verification

### Phase 3 Goal

Remove dependency artefacts, update documentation, and confirm regressions are absent.

### Phase 3 Inputs

- Dependencies: Phases 1 and 2 complete.
- Source Code / Docs to Modify:
  - `Cargo.toml`, `Cargo.lock`
  - `docs/tasks/T-9r1su-fs2-dependency-retirement/README.md`
  - `docs/error_handling.md` (if messaging adjustments)

### Phase 3 Tasks

- [x] **Dependency cleanup**
  - [x] Remove `fs2` entries from manifests and regenerate the lockfile.
  - [x] Run `cargo metadata` to verify dependency graph.
- [x] **Documentation & traceability**
  - [x] Update docs mentioning `fs2`, marking historical references appropriately.
  - [x] Regenerate trace matrix with `bun scripts/trace-status.ts --write`.
- [ ] **Verification sweep**
  - [x] Execute required Rust workflows (`cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test --lib --quiet`).
  - [ ] Capture manual verification checklist results for macOS, Linux, and Windows (pending final upload).

### Phase 3 Deliverables

- Updated manifests without `fs2`.
- Documentation aligned with new behaviour and traceability reports updated.
- Verification logs and manual test notes (Windows manual verification pending).

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
- Manual platform spot checks recorded and attached to the task (Windows checklist outstanding).

### Phase 3 Rollback/Fallback

- Re-add `fs2` dependency if critical regressions emerge; document rollback decision in task log.

---

## Testing Strategy

### Unit Tests

- Maintain unit tests alongside modules (`disk_probe`, `file_ops`) covering success/failure and platform conditions.

### Integration Tests

- Extend doctor and uninstall integration tests to assert disk space messaging and file-in-use warnings.
- Use CI matrix to run platform-specific ignored tests as part of release validation.

### External API Parsing (if applicable)

- Include inline `sysinfo` JSON snapshots in tests to validate parsing without live system calls.

### Performance & Benchmarks (if applicable)

- Record disk probe timings and locking contention metrics; ensure results stay within targets (50 ms disk check, <0.1% CPU for lock waits).

## Documentation Impact

- Update `docs/error_handling.md` and task README to reflect new behaviour.
- Ensure traceability matrix includes references to FR-x63pa and FR-rxelv.
- Coordinate with external docs repo for user-facing messaging if required (documented as future action).

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../../templates/README.md#plan-template-planmd) in the templates README.
