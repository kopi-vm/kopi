# T-5msmf Installation Lock Integration Plan

## Metadata

- Type: Implementation Plan
- Status: Draft
  <!-- Draft: Planning complete, awaiting start | Phase X In Progress: Actively working | Cancelled: Work intentionally halted before completion | Complete: All phases done and verified -->

## Links

- Associated Design Document:
  - [T-5msmf-installation-locking-design](./design.md)

## Overview

Introduce lock-aware coordination into the installation pipeline so each `kopi install` invocation acquires and releases a scoped lock around filesystem mutations, delivering exclusive access per canonical coordinate while preserving throughput for distinct installs.

## Success Metrics

- [ ] Parallel installs of the same coordinate serialize without filesystem conflicts in automated concurrency tests.
- [ ] Lock acquisition adds at most 100 ms to uncontended installs in local benchmark runs.
- [ ] No stale lock or marker files remain after forced termination scenarios.
- [ ] All existing tests pass; no regressions in installation behaviour or CLI output beyond expected lock messaging.

## Scope

- Goal: Wire `LockController` and `StatusReporterObserver` into `InstallCommand` so installation work executes under an `InstallationLockGuard`.
- Non-Goals: Extending locking to uninstall or cache flows (covered by separate tasks); altering CLI arguments or default timeout semantics.
- Assumptions: Locking foundation (T-ec5ew) and timeout controls (T-lqyk8) remain stable; installation pipeline continues to stage via `JdkRepository`.
- Constraints: No new third-party crates; maintain English-only user messaging; honour ADR-8mnaz decisions.

## ADR & Legacy Alignment

- [ ] Confirm ADR-8mnaz and related design documents govern the locking approach referenced above.
- [ ] Identify any residual references to legacy `fs2` locking helpers; ensure plan includes their removal or safe coexistence.

## Plan Summary

- Phase 1 – Lock scaffolding and guard plumbing
- Phase 2 – Pipeline integration and progress feedback
- Phase 3 – Concurrency validation & regression tests

### Phase Status Tracking

Mark checkboxes (`[x]`) immediately after completing each task or subtask. If an item is intentionally skipped or deferred, annotate it (e.g., strike-through with a brief note) instead of leaving it unchecked.

---

## Phase 1: Lock scaffolding and guard plumbing

### Goal

Create the coordination primitives that transform resolved packages into lock scopes and ensure RAII release semantics for installation locks.

### Inputs

- Documentation:
  - `docs/tasks/T-5msmf-installation-locking/design.md` – Approved design details.
- Source Code to Modify:
  - `src/commands/install.rs` – Install command orchestration.
  - `src/locking/` – Existing controller, handle, and observer types.
- Dependencies:
  - Internal: `src/locking/controller.rs`, `src/locking/wait_observer.rs`.
  - External crates: `log` for diagnostics.

### Tasks

- [ ] **Guard implementation**
  - [ ] Add `InstallationLockGuard` struct wrapping `LockController` + `LockAcquisition`.
  - [ ] Provide explicit `release()` returning `Result<()>` and `backend()` accessor for logging.
- [ ] **Scope derivation**
  - [ ] Write helper that converts `Package` metadata into `PackageCoordinate` and then `LockScope::installation`.
  - [ ] Unit-test slug canonicalisation cases (JavaFX, libc variant, architecture).

### Deliverables

- New guard type with documented behaviour.
- Unit tests validating coordinate derivation and guard drop safety.

### Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet locking
```

### Acceptance Criteria (Phase Gate)

- Guard ensures drop-based release without panics.
- Helper returns deterministic lock paths for representative package shapes.
- No compilation errors or clippy findings in touched modules.

### Rollback/Fallback

- Revert guard introduction commits; reinstall previous `InstallCommand` behaviour (no locking). No persisted data changes.

---

## Phase 2: Pipeline integration and progress feedback

### Phase 2 Goal

Integrate the guard into the installation workflow, adjust progress accounting, and connect lock wait feedback to CLI output.

### Phase 2 Inputs

- Dependencies:
  - Phase 1 guard and scope helpers.
  - `src/indicator::ProgressIndicator` implementations for progress suspension.
- Source Code to Modify:
  - `src/commands/install.rs` – restructure to acquire locks before mutations.
  - `src/indicator/status.rs` or new sink glue (if required).

### Phase 2 Tasks

- [ ] **Command wiring**
  - [ ] Instantiate `LockController::with_default_inspector` and acquire guard after package resolution.
  - [ ] Move install directory checks, forced removal, staging, extraction, metadata writes, and shim creation inside the guarded block.
- [ ] **User feedback**
  - [ ] Insert a progress step for “Acquiring installation lock”.
  - [ ] Route lock wait messages through `StatusReporterObserver`, ensuring `progress.suspend` prevents bar corruption.
  - [ ] Log final backend (`Advisory` vs `Fallback`) at INFO level.

### Phase 2 Deliverables

- Lock-aware install command with clear contention messaging.
- Logging confirming backend selection.

### Phase 2 Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet commands::install
```

### Phase 2 Acceptance Criteria

- Running `kopi install` on fresh coordinate produces identical functional output plus lock messaging.
- Forced reinstall (`--force`) executes without race warnings and under guard coverage.
- Progress indicators remain stable (no duplicated lines or broken formatting) when contention occurs.

### Phase 2 Rollback/Fallback

- Optionally keep guard implementation but feature-flag integration inside `InstallCommand` by gating acquisition; revert command changes if necessary.

---

## Phase 3: Concurrency validation & regression tests

### Phase 3 Goal

Create automated coverage for contention scenarios, fallback behaviour, and regression protection for release handling.

### Phase 3 Tasks

- [ ] Test utilities
  - [ ] Introduce helper to spawn simultaneous `kopi install` commands in integration tests with temporary `KOPI_HOME`.
  - [ ] Provide fixture forcing fallback path (e.g., mock inspector or mount classification).
- [ ] Scenarios
  - [ ] Happy path: single install asserts lock acquired quickly.
  - [ ] Contention: second install waits and completes after the first releases.
  - [ ] Timeout/error: configure tiny lock timeout, assert `KopiError::LockingTimeout` surfaced cleanly.
- [ ] Concurrency & cleanup
  - [ ] Validate no leftover `.lock`/`.marker` files after aborting one process mid-install.
  - [ ] Ensure `--dry-run` bypasses locking without creating artefacts.

### Phase 3 Deliverables

- Integration test suite in `tests/install_locking.rs`.
- Documented known limitations (if any) appended to `docs/tasks/T-5msmf-installation-locking/README.md`.

### Phase 3 Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --quiet tests::install_locking
```

### Phase 3 Acceptance Criteria

- Integration tests pass on Unix and Windows CI targets.
- No orphaned lock files detected after running the suite repeatedly.
- Timeout scenario emits English, user-friendly error output.

### Phase 3 Rollback/Fallback

- Disable new tests while retaining implementation, or feature-gate fallback-specific checks if environment coverage becomes a blocker.

---

## Definition of Done

- [ ] `cargo fmt`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --lib --quiet`
- [ ] Integration suites (`cargo test --quiet tests::install_locking`) added/executed
- [ ] Documentation updated (`docs/architecture.md`, task README) and `bun format && bun lint` run for markdown artifacts
- [ ] Traceability regenerated via `bun scripts/trace-status.ts --write`
