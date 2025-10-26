# T-q5f2p Process Activity Detection Plan

## Metadata

- Type: Implementation Plan
- Status: Phase 2 In Progress
  <!-- Draft: Planning complete, awaiting start | Phase X In Progress: Actively working | Cancelled: Work intentionally halted before completion | Complete: All phases done and verified -->

## Links

- Associated Design Document:
  - [T-q5f2p-process-activity-detection-design](design.md)

## Overview

Deliver a safe Rust API that enumerates operating system processes with open handles inside a target JDK directory. The plan sequences the backend scaffolding, platform-specific implementations, and integration with uninstall safeguards, concluding with verification and documentation updates required by the Kopi TDL.

## Success Metrics

- [ ] `processes_using_path` returns accurate `ProcessInfo` data for Linux, macOS, and Windows in manual smoke tests.
- [ ] New functionality integrates with uninstall safeguards without introducing regressions in existing locking tests.
- [ ] End-to-end uninstall flow prints actionable PID/executable diagnostics when files are in use.
- [ ] All existing tests continue to pass across supported platforms.

## Scope

- Goal: Build cross-platform process enumeration plumbing and surface it through `src/platform/process.rs` for uninstall callers.
- Non-Goals: Redesign uninstall UX beyond inserting additional diagnostics; telemetry and analytics remain deferred.
- Assumptions: `/proc` is available on Linux hosts, macOS exposes `proc_pidinfo`, and Windows permits handle duplication for non-system processes without admin rights.
- Constraints: No `unsafe` code, no shelling out, and keep new dependencies lightweight (`procfs`, `libproc`, `windows`).

## ADR & Legacy Alignment

- [ ] Confirm ADR-8mnaz governs concurrency expectations and update if deviations emerge.
- [ ] Identify any deprecated `fs2` usage that conflicts with the new approach and queue cleanup subtasks where necessary.

## Plan Summary

- Phase 1 – Platform Facade & Data Structures
- Phase 2 – Platform Backends & Fixtures
- Phase 3 – Integration, Testing, and Documentation

### Phase Status Tracking

Mark checkboxes when completing tasks. Stop after each phase for approval before proceeding per TDL.

---

## Phase 1: Platform Facade & Data Structures

### Goal

Create shared data types, helper functions, and the public `processes_using_path` API surface without touching platform-specific enumeration logic.

### Inputs

- Documentation:
  - `/docs/tasks/T-q5f2p-process-activity-detection/design.md` – Approved design details.
- Source Code to Modify:
  - `src/platform/process.rs` – Public API surface and platform-specific logic.
  - `src/platform/mod.rs` – Re-export configuration if needed.
- Dependencies:
  - Internal: `crate::error`, `crate::fs` modules for canonicalization utilities.
  - External crates: Evaluate adding `procfs`, `libproc`, `windows` in later phases (no use yet).

### Tasks

- [x] **API definition**
  - [x] Introduce `ProcessInfo` struct and associated helper enums in `process.rs`.
  - [x] Stub `processes_using_path` with `cfg`-dispatched backend calls returning `todo!()`.
- [x] **Path normalization helpers**
  - [x] Implement `normalize_target` using `std::fs::canonicalize` with descriptive error mapping.
  - [x] Add tests covering symlinked directories and missing paths.

### Deliverables

- Compilable facade returning placeholder values guarded by `unimplemented!()` in platform stubs.
- Unit tests for helpers to guide later implementations.

### Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet platform::process
```

### Acceptance Criteria (Phase Gate)

- API compiles on all platforms with placeholder backends.
- Tests for normalization utilities pass.
- Design assumptions unchanged; document follow-ups if scope expands.

### Rollback/Fallback

- Revert facade additions if API shape conflicts with downstream requirements; defer implementation until clarified.

---

## Phase 2: Platform Backends & Fixtures

### Phase 2 Goal

Implement Linux/Unix, macOS, and Windows backends plus captured fixtures that enable deterministic testing of handle enumeration logic.

### Phase 2 Inputs

- Dependencies:
  - Phase 1 facade and helpers.
  - Platform crates added to `Cargo.toml` as required (e.g., `procfs`, `libproc`, `windows`).
- Source Code to Modify:
  - `src/platform/process.rs` – Add `cfg`-scoped helpers for each platform alongside shared facade.
  - `Cargo.toml` – Dependency declarations with platform-specific cfg.

### Phase 2 Tasks

- [x] **Linux / Unix backend**
  - [x] Add `cfg(target_os = "linux")` helper inside `process.rs` that walks `/proc` using standard library iterators and filters descriptor symlinks under the target.
  - [x] Populate `ProcessInfo` with executable paths and handle lists.
  - [x] Handle permission errors gracefully with warnings.
- [ ] **macOS backend**
  - [ ] Implement a `cfg(target_os = "macos")` helper inside `process.rs` that uses `libproc` to inspect open file descriptors and convert Mach paths to `PathBuf`.
  - [ ] Capture fixture JSON or plist from `lsof -F` for unit tests and document provenance.
- [ ] **Windows backend**
  - [ ] Implement a `cfg(windows)` helper inside `process.rs` that enumerates handles via `NtQuerySystemInformation` and filters `FILE` types.
  - [ ] Resolve paths with `GetFinalPathNameByHandleW`; normalize case-insensitive comparisons.
  - [ ] Ensure duplicated handles close reliably to avoid leaks.
- [ ] **Fixtures & tests**
  - [ ] Store recorded API responses (e.g., handle dumps, mocked `/proc`) under `tests/fixtures/` and cite collection commands in comments.
  - [ ] Add unit tests for each backend using fixtures.

### Phase 2 Deliverables

- Platform-specific modules returning populated `ProcessInfo` collections.
- Unit tests validating enumeration logic against fixtures.

### Phase 2 Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet platform::process
```

### Phase 2 Acceptance Criteria

- Each backend passes unit tests with fixtures.
- No `unsafe` code introduced; permission errors handled gracefully.
- Conditional compilation builds cleanly on all platforms (verified via `cargo check --target` as needed).

### Phase 2 Rollback/Fallback

- If a backend proves infeasible (e.g., macOS permissions), document limitation and gate functionality behind feature flags until resolved.

---

## Phase 3: Integration, Testing, and Documentation

### Phase 3 Goal

Wire the new API into uninstall safeguards, add integration tests, and update documentation before exiting the task.

### Phase 3 Tasks

- [ ] Integration
  - [ ] Update uninstall logic to call `processes_using_path` and extend diagnostics with PID/executable info.
  - [ ] Ensure CLI messaging follows existing style guides and remains localized in English.
- [ ] Test coverage
  - [ ] Add integration tests (per platform) that open a file within a temp JDK directory and assert detection.
  - [ ] Extend existing locking-related tests to confirm no regressions.
- [ ] Documentation & traceability
  - [ ] Update task README, traceability matrix, and downstream docs references per TDL.
  - [ ] Capture manual verification steps for Windows/macOS permission edge cases.

### Phase 3 Deliverables

- Integrated uninstall workflows with new diagnostics.
- Updated documentation and traceability artifacts.

### Phase 3 Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet
cargo test --quiet
bun format
bun lint
```

### Phase 3 Acceptance Criteria

- All tests pass across platforms; uninstall flow verified manually on at least one platform per backend.
- Traceability updated with new links; README reflects task progress.
- Known limitations documented and communicated to stakeholders.

### Phase 3 Rollback/Fallback

- Feature flag uninstall integration; revert to previous behavior if regressions surface while retaining enumeration API for future reuse.

---

## Definition of Done

- [ ] `cargo check`
- [ ] `cargo fmt`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --lib --quiet`
- [ ] Broader test suites (integration/perf) executed when relevant
- [ ] Documentation and traceability artifacts updated per TDL
- [ ] Error messages actionable in English; exit codes preserved
- [ ] No `unsafe` code and no vague naming such as "manager" or "util"

## Open Questions

- [ ] Determine tooling support or custom FFI required for macOS `proc_pidinfo`; spike during Phase 2 backlog grooming.
- [ ] Define fallback messaging when Windows denies handle duplication (Phase 2 deliverable).
- [ ] Decide whether to gate uninstall integration behind a feature flag for initial release (Phase 3 planning).

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../../templates/README.md#plan-template-planmd) in the templates README.
