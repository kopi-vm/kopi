# T-60h68 Lock Contention Feedback Implementation Plan

## Metadata

- Type: Implementation Plan
- Status: Phase 2 In Progress
  <!-- Draft: Planning complete, awaiting start | Phase X In Progress: Actively working | Cancelled: Work intentionally halted before completion | Complete: All phases done and verified -->

## Links

- Associated Design Document:
  - [T-60h68-lock-feedback-design](./design.md)

## Overview

Implement the lock wait feedback experience defined in the approved design by introducing a reusable observer bridge, reusing the shared indicator subsystem, and updating CLI flows so users receive consistent, actionable guidance whenever Kopi waits on file locks.

## Success Metrics

- [ ] Initial wait messages appear within 100 ms of `on_wait_start` in automated timing tests.
- [ ] Progress indicators refresh ≥1 Hz on interactive terminals and ≤5 s cadence on non-TTY outputs during sustained contention.
- [ ] Quiet/JSON modes emit no additional lock wait lines while still logging diagnostic context at DEBUG level.
- [ ] Unit and integration tests exercising lock contention, timeout, and cancellation scenarios pass without regressions; `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --lib --quiet` complete successfully.

## Scope

- Goal: Deliver a unified lock feedback bridge backed by `src/indicator` so all lock-aware commands surface consistent progress, completion, timeout, and cancellation messaging.
- Non-Goals: Redesigning the broader progress indicator API, changing lock timeout semantics (owned by T-lqyk8), or altering user documentation in the external `kopi-vm.github.io` repository.
- Assumptions: Lock timeout control (T-lqyk8) and locking foundation (T-ec5ew) remain stable; `ProgressFactory` already mediates TTY/non-TTY detection; cancellation token plumbing from T-lqyk8 is available.
- Constraints: No new third-party crates; English-only messaging; align with ADR-8mnaz decisions; respect existing indicator architecture (`src/indicator/`).

## ADR & Legacy Alignment

- [ ] Confirm ADR-8mnaz covers all locking behaviours referenced; update links if additional ADRs become relevant.
- [ ] Identify and retire legacy `StatusReporterObserver` messaging once the bridge is in place; ensure plan tasks track removal or migration of the old observer to avoid dual outputs.

## Plan Summary

- Phase 1 – Bridge foundation and indicator alignment
- Phase 2 – Command integration and configuration plumbing
- Phase 3 – Testing, validation, and documentation touch-ups

### Phase Status Tracking

Mark checkboxes (`[x]`) immediately after completing each task or subtask. If an item is intentionally skipped or deferred, annotate it instead of leaving it unchecked. After each phase, pause for approval before progressing, per TDL guidance.

---

## Phase 1: Bridge Foundation and Indicator Alignment

### Goal

Create the `LockFeedbackBridge`, refactor `StatusReporter` output to flow through `ProgressIndicator`, and wire the indicator factory so lock wait events reuse the shared rendering stack.

### Inputs

- Documentation:
  - `docs/tasks/T-60h68-lock-feedback/design.md` – Approved design.
  - `docs/architecture.md` (§Locking instrumentation, indicator subsystem) – Structural guidance.
- Source Code to Modify:
  - `src/locking/wait_observer.rs` – Define bridge implementation and retire legacy observer usage.
  - `src/indicator/{factory,mod,status}.rs` – Expose lock feedback helper and align reporter output with indicators.
  - `src/indicator/simple.rs`, `src/indicator/indicatif.rs` – Ensure renderers accept lock feedback configurations.
- Dependencies:
  - Internal: `src/locking/controller.rs`, `src/locking/timeout.rs`, `src/indicator/types.rs`.
  - External crates: `indicatif`, `colored`, `signal_hook` (already in use; no new dependencies).

### Tasks

- [x] **Bridge implementation**
  - [x] Introduce `LockFeedbackBridge` implementing `LockWaitObserver`, mapping lifecycle events to indicator calls.
  - [x] Add elapsed/remaining time tracking and uniform action hints (`Ctrl-C`, `--lock-timeout`).
  - [x] Provide constructor helpers (e.g., `LockFeedbackBridge::for_progress(progress: &mut dyn ProgressIndicator, scope: LockScope, timeout: LockTimeoutValue)`).
- [x] **Indicator alignment**
  - [x] Extend `ProgressFactory` with a method that returns the appropriate indicator trio (`IndicatifProgress`, `SimpleProgress`, `SilentProgress`) for lock feedback contexts.
  - [x] Update `StatusReporter` to delegate `operation/step/success/error` through the active `ProgressIndicator::println` / `success` / `error` pathways; add `lock_feedback_start` helper per design.
  - [x] Ensure renderer throttling and quiet-mode detection remain centralised in the factory.
- [x] **Legacy observer migration**
  - [x] Replace `StatusReporterObserver` usage with the new bridge; keep a temporary shim if needed for incremental rollout.
  - [x] Update or remove `LockStatusSink` trait if superseded by bridge wiring.

### Deliverables

- New bridge module with unit tests for message formatting and throttling.
- Updated indicator factory/reporter that routes lock feedback through shared renderers.
- Deprecated or removed legacy observer implementation.

### Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet locking::wait_observer indicator
```

### Acceptance Criteria (Phase Gate)

- Bridge emits initial wait, periodic progress, acquisition, timeout, and cancellation messages matching design wording.
- `StatusReporter` no longer prints duplicate lines when lock waits occur; quiet mode suppresses output entirely.
- Unit tests cover finite vs. infinite timeout messaging and non-TTY cadence logic.

### Rollback/Fallback

- Reintroduce `StatusReporterObserver` via git revert to restore previous messaging while investigating bridge issues; no persistent data is affected.

---

## Phase 2: Command Integration and Configuration Plumbing

### Phase 2 Goal

Integrate the bridge across lock-using workflows (installation, uninstallation, cache writers) and ensure CLI/environment flags configure feedback correctly.

### Phase 2 Inputs

- Dependencies:
  - Phase 1 bridge and indicator updates.
  - Existing locking controller hooks (`acquire_with`, `acquire_with_status_sink`).
- Source Code to Modify:
  - `src/locking/controller.rs` & `src/locking/acquisition.rs` – Accept bridge instances and expose helper to register observers.
  - `src/commands/{install,uninstall}.rs`, `src/cache/refresh.rs` (or equivalent entry points) – Register `LockFeedbackBridge` instances.
  - `src/bin/kopi.rs` / CLI bootstrap – Ensure quiet/JSON modes pass through to factory selection where necessary.

### Phase 2 Tasks

- [x] **Controller integration**
  - [x] Add API (e.g., `LockController::acquire_with_feedback(progress: &mut dyn ProgressIndicator, scope: ...)`) that constructs the bridge and passes it to acquisition.
  - [x] Ensure cancellation tokens and timeout sources flow into the bridge for action guidance.
- [x] **Command wiring**
  - [x] Update installation, uninstallation, cache refresh (and any other lock users) to obtain a progress indicator from `ProgressFactory`, register the bridge, and emit outcome messages through the bridge instead of ad-hoc prints.
  - [x] Confirm quiet mode / non-interactive commands bypass visible output while still logging at DEBUG.
- [x] **Configuration hooks**
  - [x] Honour `KOPI_NO_TTY_PROGRESS` / `KOPI_FORCE_TTY_PROGRESS` when selecting the renderer for lock feedback.
  - [x] Ensure CLI global flags (`--quiet`, `--json`, `--lock-timeout`) plumb into bridge action hints and suppression logic.

### Phase 2 Deliverables

- All lock-consuming commands use the bridge for contention messaging.
- Controller exposes clear API for future features to opt into lock feedback.
- Manual sanity tests confirm behaviour on TTY and non-TTY environments.

### Phase 2 Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet commands::install commands::uninstall locking::controller
# Smoke tests for binaries (manual/optional)
#   KOPI_NO_TTY_PROGRESS=1 cargo run -- install ...
```

### Phase 2 Acceptance Criteria

- Running representative commands shows consistent lock wait messaging (TTY carriage-return, non-TTY appended lines, quiet mode silence).
- Cancellation via Ctrl-C surfaces the bridge’s cancellation message and maps to the existing `KopiError::LockingCancelled`.
- Timeout scenarios reference `--lock-timeout` overrides and include resolved timeout source.

### Phase 2 Rollback/Fallback

- Temporarily revert command-specific integrations while keeping the bridge in place; fall back to default `StatusReporter` output if regressions arise during testing.

---

## Phase 3: Testing, Validation, and Documentation Touch-Ups

### Phase 3 Goal

Finalize automated coverage, validate cross-platform behaviour, and update architectural references to reflect the new feedback bridge.

### Phase 3 Tasks

- [ ] **Automated tests**
  - [ ] Add targeted unit tests for TTY vs. non-TTY rendering, infinite timeout messaging, and quiet mode suppression (indicator + bridge modules).
  - [ ] Introduce integration tests simulating lock contention using temporary directories and background threads; verify elapsed/remaining output cadence.
  - [ ] Add regression tests for cancellation path ensuring `Ctrl-C` triggers the bridge’s cancellation messaging.
- [ ] **Documentation updates**
  - [ ] Update `docs/architecture.md` lock instrumentation section with the new bridge responsibilities.
  - [ ] Regenerate `docs/traceability.md` after changes (`bun scripts/trace-status.ts --write`).
- [ ] **Manual validation**
  - [ ] Perform smoke tests on macOS/Linux/Windows terminals to confirm carriage-return behaviour and action hints.
  - [ ] Capture non-TTY sample output for CI review.

### Phase 3 Deliverables

- Comprehensive unit/integration test coverage for lock feedback.
- Updated architecture and traceability documentation reflecting implementation.
- Manual validation checklist stored with task notes (e.g., `docs/tasks/T-60h68-lock-feedback/README.md` updates if necessary).

### Phase 3 Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --quiet
bun format
bun lint
bun scripts/trace-status.ts --write
```

### Phase 3 Acceptance Criteria

- Tests cover happy path, timeout, cancellation, and quiet-mode scenarios with deterministic assertions.
- Documentation accurately describes the bridge and indicator integration.
- Traceability matrix reports no gaps.

### Phase 3 Rollback/Fallback

- If integration tests prove too flaky, isolate them behind an opt-in feature flag while addressing the root cause, retaining unit coverage to prevent regressions.

---

## Definition of Done

- [ ] `cargo fmt`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --lib --quiet`
- [ ] `cargo test --quiet`
- [ ] `bun format`
- [ ] `bun lint`
- [ ] `bun scripts/trace-status.ts --write`
- [ ] Relevant documentation (e.g., `docs/architecture.md`, task README) updated
- [ ] Traceability matrix regenerated with no gaps
