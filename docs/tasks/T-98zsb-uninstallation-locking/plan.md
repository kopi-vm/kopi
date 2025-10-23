# T-98zsb Uninstallation Lock Integration Plan

## Metadata

- Type: Implementation Plan
- Status: Draft
  <!-- Draft: Planning complete, awaiting start | Phase X In Progress: Actively working | Cancelled: Work intentionally halted before completion | Complete: All phases done and verified -->

## Links

- Associated Design Document:
  - [T-98zsb-uninstallation-locking-design](./design.md)

## Overview

Coordinate the changes required to bring per-coordinate locking, timeout-aware feedback, and telemetry parity to all uninstall flows. The plan tracks the work from foundational guard refactors through single and batch uninstall integration, ending with concurrency validation and documentation updates.

## Success Metrics

- [ ] Install/uninstall contention tests complete without filesystem corruption or orphaned directories.
- [ ] Batch uninstall under contention reports accurate failures while other entries proceed.
- [ ] No new clippy warnings or failing tests across `cargo test --lib --quiet` and targeted integration suites.

## Scope

- Goal: Acquire and release the shared installation lock for every uninstall execution path (single, batch, recovery) while reusing existing feedback observers.
- Non-Goals: Changing CLI flags, modifying install/cache locking semantics, or introducing distributed coordination mechanisms.
- Assumptions: Locking foundation (T-ec5ew) and timeout controls (T-lqyk8) remain stable; metadata files are generated during install but fallback logic must handle gaps.
- Constraints: Keep code in Rust stable channel, avoid unsafe blocks, maintain English-only output, and respect architecture layout described in `docs/architecture.md`.

## ADR & Legacy Alignment

- [ ] Confirm ADR-8mnaz remains the authoritative reference for locking backend selection.
- [ ] Verify no legacy `fs2` remnants interfere with new guard abstractions.

## Plan Summary

- Phase 1 – Guard refactor and scope resolution primitives
- Phase 2 – Single uninstall integration
- Phase 3 – Batch & cleanup locking coverage
- Phase 4 – Concurrency validation, docs, and polish

### Phase Status Tracking

Mark checkboxes (`[x]`) immediately after completing each task or subtask. Annotate deferred items instead of leaving them unchecked.

---

## Phase 1: Guard Refactor and Scope Resolution Primitives

### Goal

Provide shared locking primitives for installed artifacts so later phases can focus on wiring rather than mechanics.

### Inputs

- Documentation: `docs/tasks/T-98zsb-uninstallation-locking/design.md`
- Source: `src/locking/installation.rs`, `src/locking/mod.rs`, `src/storage/mod.rs`, `src/storage/listing.rs`, `src/paths/install.rs`
- Dependencies: existing lock controller, package coordinate builder, installation metadata types

### Tasks

- [ ] **Scoped guard extraction**
  - [ ] Move `InstallationLockGuard` into a neutral module (e.g., `locking/scoped_guard.rs`) and rename to `ScopedPackageLockGuard`.
  - [ ] Update existing installation call sites to use the new module and confirm API compatibility.
- [ ] **Installed scope resolver**
  - [ ] Add helper that strictly parses `JdkMetadataWithInstallation` for an `InstalledJdk`, deriving `LockScope::installation` and variant tags.
  - [ ] Implement fallback slug generation leveraging `InstallationMetadata::platform` and directory naming when metadata is absent or parsing fails (no recovery attempt).
  - [ ] Unit-test metadata parsing, platform splitting, and slug sanitisation cases, including corrupted JSON that triggers the fallback without modifying files.
- [ ] **Repository accessors**
  - [ ] Expose read-only access to `KopiConfig` or introduce a dedicated metadata loader on `JdkRepository` with security checks.

### Deliverables

- Reusable guard type and module-level documentation.
- `InstalledScopeResolver` + supporting tests.
- Updated `locking::mod` exports and any necessary architectural notes.

### Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet locking
```

### Acceptance Criteria

- Installation continues to compile and pass tests using the new guard.
- Scope resolver returns deterministic lock paths across OS/arch/libc combinations and warns (not panics) on fallback.
- No regressions introduced in existing locking unit tests.

### Rollback/Fallback

- Revert guard extraction and keep uninstall-specific helpers local if unforeseen coupling appears.
- Feature-flag resolver fallback to allow rapid disablement if legacy installs misbehave.

---

## Phase 2: Single Uninstall Integration

### Goal

Wrap `UninstallHandler::uninstall_jdk` in the scoped lock guard with user-visible feedback and timeout handling.

### Inputs

- Phase 1 guard and resolver
- Source: `src/uninstall/mod.rs`, `src/commands/uninstall.rs`, `src/uninstall/safety.rs`, `src/indicator/status.rs`
- Dependencies: `StatusReporter`, `LockController`, `LockStatusSink`

### Tasks

- [ ] Acquire lock controller within `UninstallHandler`, reusing `KopiConfig::kopi_home()` and locking config.
- [ ] Integrate `ScopedPackageLockGuard` around safety checks, atomic rename, metadata removal, and rollback paths.
- [ ] Route wait messaging through `StatusReporterObserver`, ensuring progress output remains intact (`StatusReporter::step/success/error`).
- [ ] Surface backend (`advisory`/`fallback`) via `info!` logging and append scope labels for diagnostics.
- [ ] Update unit tests for `UninstallHandler` to account for locking, including success, timeout, and rollback scenarios with mock controllers.

### Deliverables

- Lock-aware single uninstall flow with explicit guard release on success and safe drop semantics on error.
- Tests covering lock acquisition, timeout propagation, and rollback.

### Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet uninstall::mod
```

### Acceptance Criteria

- `kopi uninstall <spec>` operates normally when uncontended and emits clear wait/timeout messaging under contention.
- Timeouts raise `KopiError::LockingTimeout` with descriptive text, and directories remain untouched when lock acquisition fails.
- Progress indicators and status output remain legible in both `--no-progress` and default modes.

### Rollback/Fallback

- Keep guard wiring behind an internal feature flag if needed to disable locking quickly while preserving resolver code.

---

## Phase 3: Batch and Cleanup Coverage

### Goal

Ensure batch uninstall and recovery paths adopt the same locking guarantees without regressing UX.

### Inputs

- Phase 2 implementation
- Source: `src/uninstall/batch.rs`, `src/uninstall/cleanup.rs`, `src/uninstall/progress.rs`
- Dependencies: progress bars (`indicatif`), `StatusReporter`

### Tasks

- [ ] Acquire/release scoped locks per entry inside `BatchUninstaller::execute_batch_removal`, integrating wait observers with progress suspension.
- [ ] Handle lock acquisition failures by recording actionable errors and continuing with remaining entries.
- [ ] Ensure cleanup routines reuse per-coordinate locks only when scopes are identifiable, while keeping `force_cleanup_jdk` intentionally lock-free; document the behaviour.
- [ ] Add tests covering batch contention (mixed success/failure) and cleanup locking around temporary directories.

### Deliverables

- Batch uninstall respects per-coordinate locks with informative reporting, and cleanup documents the lock-free `force_cleanup_jdk` path.
- Test coverage ensuring partial successes and no double-deletion.

### Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet uninstall::batch uninstall::cleanup
```

### Acceptance Criteria

- Batch uninstall logs lock wait states without corrupting progress bars and skips destructive steps when locks cannot be obtained.
- Cleanup tasks avoid racing with active installs when scopes are known and confirm the lock-free force cleanup path leaves no additional artefacts.

### Rollback/Fallback

- Allow disabling cleanup locking if it proves too noisy, leaving primary uninstall locking intact.

---

## Phase 4: Concurrency Validation, Docs, and Polish

### Goal

Validate behaviour under contention, update documentation, and finalise traceability.

### Tasks

- [ ] Add integration suite (`tests/uninstall_locking.rs`) spawning concurrent install/uninstall processes and validating serialization + cleanup.
- [ ] Expand existing batch/uninstall integration tests to assert logging/exit codes for lock failures.
- [ ] Update `docs/architecture.md` and task README to reflect new locking coverage; coordinate external docs note if required.
- [ ] Run `bun format` and `bun lint` for modified Markdown, regenerate traceability via `bun scripts/trace-status.ts --write`.

### Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet
cargo test --test uninstall_locking --quiet
bun format
bun lint
bun scripts/trace-status.ts --write
```

### Acceptance Criteria

- Integration tests consistently pass on local and CI platforms (Unix + Windows runners).
- Documentation accurately describes uninstall locking behaviour; traceability matrix shows updated links.
- No unresolved TODOs or unchecked success metrics remain.

### Rollback/Fallback

- Integration tests can be marked `ignore` temporarily if environment-specific race conditions arise, with follow-up ticket noted in traceability.

---

## Definition of Done

- [ ] `cargo fmt`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --lib --quiet`
- [ ] Targeted integration suites (`cargo test --test uninstall_locking --quiet`, batch-specific modules)
- [ ] Updated documentation (`docs/architecture.md`, task README) with `bun format && bun lint`
- [ ] `bun scripts/trace-status.ts --write`
