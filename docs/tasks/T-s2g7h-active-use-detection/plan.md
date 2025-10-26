# T-s2g7h Active-Use Detection

## Metadata

- Type: Implementation Plan
- Status: Draft
  <!-- Draft: Planning complete, awaiting start | Phase X In Progress: Actively working | Cancelled: Work intentionally halted before completion | Complete: All phases done and verified -->

## Links

- Associated Design Document:
  - [T-s2g7h-active-use-detection-design](./design.md)

## Overview

Implement active-use detection across uninstall flows so Kopi blocks removal of JDKs that are currently configured as the global default or nearest project default, unless `--force` is supplied. The work touches safety checks, CLI wiring, and automated tests to verify single and batch uninstall behavior.

## Success Metrics

- [ ] Active-use guard triggers for global and project defaults in single uninstall without `--force`.
- [ ] Batch uninstall reports blocked entries while continuing to remove safe targets.
- [ ] `cargo test --lib --quiet uninstall` and `tests/uninstall_integration.rs` exercise the new paths.
- [ ] All existing checks (`cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test --lib --quiet`) pass.

## Scope

- Goal: Enforce safe uninstall by detecting globally/project-active JDKs and surfacing actionable messaging with optional override.
- Non-Goals: Detect running Java processes, alter behavior of other commands, or redesign locking/timeouts.
- Assumptions: Version files are UTF-8; repo listing already yields accurate `InstalledJdk` metadata; environment variable overrides are out of scope unless clarified.
- Constraints: Maintain compatibility across Unix/Windows paths; avoid introducing new dependencies.

## ADR & Legacy Alignment

- [x] Confirm alignment with `/docs/adr/ADR-8mnaz-concurrent-process-locking-strategy.md` (no locking changes required).
- [ ] Evaluate whether environment variable overrides should count as active use; document decision in Phase 1 checklist.

## Plan Summary

- Phase 1 – Detector Foundations
- Phase 2 – CLI Integration & Messaging
- Phase 3 – Testing & Verification

### Phase Status Tracking

Mark checkboxes (`[x]`) after completing each item or annotate skipped tasks.

---

## Phase 1: Detector Foundations

### Goal

Implement reusable helpers in `uninstall::safety` to evaluate global and project configuration against a target `InstalledJdk`.

### Inputs

- Documentation:
  - `/docs/tasks/T-s2g7h-active-use-detection/design.md` – architectural direction
- Source Code to Modify:
  - `/src/uninstall/safety.rs` – replace stubs with real detection logic
  - `/src/version/resolver.rs` & related modules – reference patterns for reading version files
- Dependencies:
  - Internal: `src/storage/` for `InstalledJdk` metadata
  - Internal: `src/version/` for `VersionRequest` parsing

### Tasks

- [ ] **`ActiveUseDetector scaffolding`**
  - [ ] Decide and document whether `KOPI_JAVA_VERSION` should be treated as active (per open question).
  - [ ] Introduce helpers to read global and project version files with error handling.
  - [ ] Implement `request_matches_jdk` comparison covering distribution, version, and JavaFX suffix.
- [ ] **`Safety check integration`**
  - [ ] Update `perform_safety_checks` signature to accept config, repository, target `InstalledJdk`, and a `force` flag.
  - [ ] Ensure validation errors include actionable guidance (switch or `--force`).

### Deliverables

- Updated `safety.rs` with fully implemented active-use detection helpers.
- Documented decision about environment variable scope in code comments or plan notes.

### Verification

```bash
cargo check --lib uninstall
cargo fmt
cargo clippy --all-targets -- -D warnings
```

### Acceptance Criteria (Phase Gate)

- Safety checks return `ValidationError` for matching global/project defaults when `force == false`.
- Helper functions covered by unit tests verifying true/false cases.

### Rollback/Fallback

- Revert to stub implementations via git if detection proves unstable; note follow-up task to revisit.

---

## Phase 2: CLI Integration & Messaging

### Phase 2 Goal

Propagate the force flag and detection results through single and batch uninstall flows with consistent user feedback.

### Phase 2 Inputs

- Dependencies:
  - Phase 1 helpers must be complete.
- Source Code to Modify:
  - `/src/uninstall/mod.rs` – pass `force` flag into safety checks and adjust messaging.
  - `/src/uninstall/batch.rs` – propagate `force`, handle per-JDK failures, ensure reporter output.
  - `/src/commands/uninstall.rs` – extend handler API usage to include `force`.

### Phase 2 Tasks

- [ ] **`API adjustments`**
  - [ ] Update `UninstallHandler::uninstall_jdk` signature and callers to include `force`.
  - [ ] Modify batch execution to carry `force` into `perform_safety_checks`.
- [ ] **`User feedback`**
  - [ ] Add reporter/log messages acknowledging forced removal when applicable.
  - [ ] Ensure batch summary highlights blocked entries due to active-use detection.

### Phase 2 Deliverables

- CLI flows honoring new safety checks with descriptive messages.

### Phase 2 Verification

```bash
cargo check
cargo fmt
cargo clippy --all-targets -- -D warnings
```

### Phase 2 Acceptance Criteria

- Single uninstall aborts without `--force` and succeeds with `--force` (manual/integration verification).
- Batch uninstall skips active defaults but continues processing others, reporting results.

### Phase 2 Rollback/Fallback

- If force propagation introduces regressions, restore previous handler signatures and flag detection for follow-up.

---

## Phase 3: Testing & Integration

### Phase 3 Goal

Add comprehensive automated coverage validating global/project guard behavior in single and batch uninstalls.

### Phase 3 Tasks

- [ ] Test utilities
  - [ ] Extend test fixtures to create `.kopi-version` / `.java-version` files.
  - [ ] Provide helper to write global version file via `InstalledJdk::write_to`.
- [ ] Scenarios
  - [ ] Single uninstall blocked by global default, unblocked by `--force`.
  - [ ] Single uninstall blocked by project default from nested directory.
  - [ ] Batch uninstall where only subset is blocked.
- [ ] Concurrency & cleanup
  - [ ] Confirm tests clean up version files to avoid cross-test pollution.

### Phase 3 Deliverables

- Updated `tests/uninstall_integration.rs` (and unit tests in `safety.rs`) covering new cases.

### Phase 3 Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet uninstall
cargo test --quiet tests::uninstall_integration
```

### Phase 3 Acceptance Criteria

- New tests fail on stubbed detection and pass with implementation; no regressions in existing suites.

---

## Definition of Done

- [ ] `cargo check`
- [ ] `cargo fmt`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --lib --quiet`
- [ ] Integration tests: `cargo test --quiet tests::uninstall_integration`
- [ ] Documentation updates complete; follow-up PR filed for user docs if messaging changes
