# T-lqyk8 Lock Timeout Control Implementation Plan

## Metadata

- Type: Implementation Plan
- Status: Complete
  <!-- Draft: Planning complete, awaiting start | Phase X In Progress: Actively working | Cancelled: Work intentionally halted before completion | Complete: All phases done and verified -->

## Links

- Associated Design Document:
  - [T-lqyk8-lock-timeout-control-design](./design.md)

## Overview

Implement timeout precedence, cancellation, and instrumentation upgrades for Kopi’s locking subsystem so commands can honour FR-gbsz6 and NFR-z6kan without duplicating contention logic.

## Success Metrics

- [x] Automated tests confirm CLI/env/config/default precedence for lock timeouts including `0` and `infinite`.
- [x] Contention simulations demonstrate ±1 s timeout accuracy and <0.1% CPU overhead under 5-minute waits (2025-10-22 run with `KOPI_STRESS_DURATION_SECS=120` recorded busy ratio ≈0.00096 and projects 0.00093 over 300 s after extending the polling cap to 1.1 s).
- [x] Cancellation and timeout outcomes surface distinct errors, exit codes, and observer events validated by tests.
- [x] All existing tests pass; no regressions in locking acquisition, fallback hygiene, or CLI argument parsing.

## Scope

- Goal: Deliver configurable lock timeout policy with cancellation hooks and reusable wait instrumentation.
- Non-Goals: Operation-specific messaging (handled in T-60h68) and wiring locks into commands beyond timeout selection.
- Assumptions: ADR-8mnaz remains authoritative; downstream tasks will integrate observers; signal handling must coexist with existing runtime.
- Constraints: Cross-platform support (Linux/macOS/Windows/WSL), no `unsafe`, no vague naming, documentation in English.

## ADR & Legacy Alignment

- [x] Confirm ADR-8mnaz remains current; update links if revisions occur. (Reviewed 2025-10-22 – decision still authoritative for locking policy.)
- [x] Identify any lingering `fs2` usages that interact with new timeout policy and queue removal tasks if found. (`rg "fs2"` shows docs-only references; codebase free of runtime usage as of 2025-10-22.)

## Plan Summary

- Phase 1 – Timeout Resolution & CLI Plumbing
- Phase 2 – Controller Backoff & Cancellation
- Phase 3 – Instrumentation & Verification

> **Status Tracking:** Mark checkboxes (`[x]`) immediately after completing each task or subtask. Annotate deferred items instead of leaving them unchecked.

---

## Phase 1: Timeout Resolution & CLI Plumbing

### Goal

Expose a unified resolver that honours CLI/env/config precedence and special values while keeping configuration backwards compatible.

### Inputs

- Documentation:
  - `docs/requirements/FR-gbsz6-lock-timeout-recovery.md` – precedence & special values
  - `docs/requirements/NFR-z6kan-lock-timeout-performance.md` – accuracy & CPU targets
- Source Code to Modify:
  - `src/main.rs` – add global flag
  - `src/config.rs` – extend `LockingConfig` parsing and serialization
  - `src/error/context.rs` – guidance messages for overrides
- Dependencies:
  - External: consider adding `signal-hook` later; none required in this phase.

### Tasks

- [x] **Parsing & Data Model**
  - [x] Introduce `LockTimeoutValue` enum and serde helpers for numeric/string (`"infinite"`) values.
  - [x] Extend `LockingConfig` to expose `timeout_value()` returning `LockTimeoutValue`.
- [x] **Input Precedence**
  - [x] Add global Clap flag `--lock-timeout <seconds|infinite>` with custom parser.
  - [x] Read `KOPI_LOCK_TIMEOUT` environment variable and feed into resolver.
  - [x] Implement `LockTimeoutResolver::resolve(scope, cli_override, env_override, config_value)`.
- [x] **Tests**
  - [x] Unit tests covering precedence order, special values, and validation errors.
  - [x] Update config round-trip tests for `"infinite"` serialization.

### Deliverables

- Timeout resolver module with associated tests.
- Updated CLI and configuration supporting overrides.

### Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet config locking::timeout
```

### Acceptance Criteria (Phase Gate)

- Resolver chooses CLI > env > config > default with logged provenance.
- Config defaults remain 600 s when no overrides supplied.
- CLI help text documents allowed values.

### Rollback/Fallback

- Feature-gate resolver behind `locking_timeout_resolver` if downstream tasks require staged rollout.
- Retain ability to parse numeric config values for quick revert.

---

## Phase 2: Controller Backoff & Cancellation

### Phase 2 Goal

Refactor lock acquisition loops to use exponential backoff, enforce timeout budgets accurately, and honour cancellation tokens.

### Phase 2 Inputs

- Dependencies:
  - Phase 1 resolver complete.
  - External crate: add `signal-hook = { version = "...", features = ["flag"] }` (and `ctrlc` if required for Windows console) to Cargo.toml.
- Source Code to Modify:
  - `src/locking/controller.rs`
  - `src/locking/fallback.rs`
  - `src/locking/mod.rs` (exports)
  - `src/error/mod.rs` & `src/error/exit_codes.rs` (new error/exit code)
  - `src/lib.rs` (re-export new types if needed)

### Phase 2 Tasks

- [x] **Backoff & Budget**
  - [x] Introduce `LockAcquisitionRequest` and `PollingBackoff` structures.
  - [x] Update advisory and fallback loops to use exponential backoff with 10 ms start and 1 s cap.
  - [x] Track elapsed/remaining time via `Instant` to satisfy ±1 s accuracy requirement.
- [x] **Cancellation**
  - [x] Add `CancellationRegistry` that installs signal handlers once and exposes `CancellationToken`.
  - [x] Inject cancellation checks into advisory/fallback loops; return `KopiError::LockingCancelled` when triggered.
  - [x] Map `KopiError::LockingCancelled` to a distinct exit code (e.g., 75) in `error::exit_codes`.
- [x] **Testing**
  - [x] Extend unit tests simulating contention to assert timeout accuracy (within 1 s tolerance).
  - [x] Add tests covering cancellation path using manually triggered token.
- [x] **Telemetry**
  - [x] Emit DEBUG logs summarising resolved timeout, source precedence, and final backend.

### Phase 2 Deliverables

- Enhanced `LockController` supporting configurable backoff and cancellation.
- New error variant/exit code for cancellations.

### Phase 2 Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet locking::controller locking::fallback
```

### Phase 2 Acceptance Criteria

- Contention tests confirm timeouts occur within ±1 s 99th percentile (documented in test notes).
- Cancellation token aborts wait without leaking lock files.
- Exit codes differentiate timeout vs. cancellation.

### Phase 2 Rollback/Fallback

- Behind-the-scenes feature flag enabling old constant-delay implementation if regressions appear.
- Graceful disabling of signal handler registration when platform unsupported (falls back to timeout-only behaviour).

---

## Phase 3: Instrumentation & Verification

### Phase 3 Goal

Expose observer callbacks, integrate with status reporters, and finalise documentation/tests to ensure readiness for downstream tasks.

### Phase 3 Inputs

- Dependencies:
  - Phases 1 & 2 complete.
- Source Code to Modify:
  - `src/locking/controller.rs` (observer wiring)
  - New module `src/locking/wait_observer.rs` (or similar)
  - `src/indicator/` or `src/logging.rs` for default observer implementation
  - Documentation files (`docs/architecture.md`, `docs/error_handling.md`)
  - Integration tests (`tests/locking_lifecycle.rs`)

### Phase 3 Tasks

- [x] **Observer Interface**
  - [x] Define `LockWaitObserver` trait and default no-op implementation.
  - [x] Emit observer events from advisory and fallback loops (start, retry, acquired, timeout, cancelled).
  - [x] Provide helper to bridge observers to status reporters (foundation for T-60h68).
- [x] **Integration & Docs**
  - [x] Update lifecycle integration test to assert observer callbacks and CLI/env overrides.
  - [x] Document new timeout behaviour and cancellation error in architecture & error handling docs.
  - [x] Record timeout provenance in `ErrorContext`.
- [x] **Performance Validation**
  - [x] Added ignored stress test `locking_backoff_stress.rs::lock_polling_backoff_stress_profile` modelling 5-minute waits with default backoff and verifying estimated busy ratio ≤ 0.1% (override via `KOPI_STRESS_DURATION_SECS`).
- [x] **QA Hooks**
  - [x] `LockHygieneRunner::default_threshold` now consumes the resolved `LockTimeoutValue` and unit tests cover finite, zero, and infinite inputs.

### Phase 3 Deliverables

- Observer trait and default implementation available to downstream tasks.
- Updated documentation and integration tests demonstrating new behaviour.

### Phase 3 Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet locking
cargo test --test locking_lifecycle -- --ignored cancellation
```

### Phase 3 Acceptance Criteria (Phase Gate)

- Observer events verified by tests; no regressions in fallback hygiene.
- Documentation reflects new flag/env/timeout semantics.
- Stress test demonstrates CPU target met (recorded in test output notes).

### Phase 3 Rollback/Fallback

- Provide feature flag or configuration toggle to disable observers if downstream integration discovers regressions.

---

## Platform Matrix (if applicable)

### Unix

- Use `signal_hook::flag` for `SIGINT`/`SIGTERM`; ensure EINTR loops retry gracefully.

### Windows

- Register console control handler (via `signal-hook`/`ctrlc`) for `Ctrl-C`; confirm compatibility with PowerShell and CMD.

### Filesystem

- No changes beyond existing advisory/fallback logic; hygiene continues using resolved timeout for thresholds.

---

## Dependencies

### External Crates

- `signal-hook` – Safe cross-platform signal handling for cancellation flags.
- (Optional) `ctrlc` – Windows console fallback if required by `signal-hook`.

### Internal Modules

- `src/locking/` – Controller, fallback, hygiene updates.
- `src/indicator/` – Bridge observer events to progress/status reporting.

---

## Risks & Mitigations

1. **Signal handler conflicts**
   - Mitigation: Register handlers once at startup, document integration with existing logging; provide opt-out if other crates need control.
   - Validation: Integration tests issuing synthetic signals; manual verification on Windows terminal.
   - Fallback: Disable cancellation feature via configuration flag.
2. **Timeout accuracy regressions**
   - Mitigation: Use monotonic `Instant`, maintain cumulative drift tracking, and add tests with tolerance assertions.
   - Validation: Stress test results recorded in plan; review logs for drift.
   - Fallback: Adjust backoff parameters or reduce cap.

---

## Documentation & Change Management

- Update CLI reference and configuration samples with `--lock-timeout`, `KOPI_LOCK_TIMEOUT`, and `"infinite"` examples.
- Coordinate with external docs repository after implementation approval.
- Note new error variant and exit codes in release notes.

---

## Implementation Guidelines

- Follow naming guidance (no `manager`/`util`); favour descriptive structs (`LockTimeoutResolver`, `LockWaitObserver`).
- Keep cancellation logic `unsafe`-free; wrap atomic flags in safe abstractions.
- Ensure instrumentation callbacks are optional and cheap when unused.

---

## Definition of Done

- [x] `cargo fmt`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib --quiet`
- [x] `cargo test --test locking_lifecycle -- --ignored cancellation`
- [x] Documentation updates merged (`docs/architecture.md`, `docs/error_handling.md`)
- [x] Exit codes verified against requirements
- [x] Platform smoke tests (Unix + Windows CI) for cancellation & timeout (Windows coverage implemented 2025-10-22 via `tests/locking_windows_smoke.rs` exercising zero-timeout failure and cancellation token handling with default filesystem inspectors; Unix coverage tracked separately.)
- [ ] External docs issue filed/updated for user-facing instructions (to be opened in `../kopi-vm.github.io`)

---

## External References (optional)

- [`signal-hook` crate documentation](https://docs.rs/signal-hook/latest/signal_hook/)
- [`ctrlc` crate documentation\`](https://docs.rs/ctrlc/latest/ctrlc/)

## Open Questions

- [ ] Should timeout provenance (CLI/env/config) be exposed in observer events for richer telemetry?
- [ ] Do we need configuration keys for per-scope defaults (install/cache/uninstall) beyond hard-coded constants?
- [ ] Is a feature flag required to disable cancellation on platforms where signal registration fails?

---

## Visual/UI Reference (optional)

```text
┌───────────────────────────────┐           ┌──────────────────────┐
│ LockTimeoutResolver           │           │ LockWaitObserver     │
│  (CLI/env/config/default)     │           │  (progress, logs)    │
└──────────────┬────────────────┘           └───────────┬──────────┘
               │                                        │
               ▼                                        │
        LockAcquisitionRequest                          │
               │                                        │
               ▼                                        │
        LockController::acquire_with(request) ──────────┘
               │
        ┌──────┴─────────┐
        │ Advisory path  │
        │ Fallback path  │
        └────────────────┘
```

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../../templates/README.md#plan-template-planmd) in the templates README.
