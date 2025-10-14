# T-ec5ew Cross-Platform Locking Foundation Implementation Plan

## Metadata

- Type: Implementation Plan
- Status: Complete
  <!-- Draft: Planning complete, awaiting start | Phase X In Progress: Actively working | Cancelled: Work intentionally halted before completion | Complete: All phases done and verified -->

## Links

- Associated Design Document:
  - [T-ec5ew-locking-foundation-design](./design.md)

## Overview

Deliver the locking foundation described in ADR-8mnaz: filesystem-aware advisory locking with deterministic cleanup, atomic fallbacks, configuration defaults, and hygiene routines that downstream tasks can rely on.

## Success Metrics

- [x] Advisory locking API passes lifecycle tests on Linux, macOS, Windows, and WSL (WSL suite added to CI matrix).
- [x] Hygiene sweep deletes 100% of synthetic fallback artefacts across 1,000 crash simulations.
- [x] Network filesystem detection downgrades to fallback with INFO warnings and no panics in automated scenarios.
- [x] `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --lib --quiet` pass before completion.

## Scope

- Goal: Implement locking components (controller, filesystem inspector, fallback strategy, hygiene runner) and supporting configuration while exposing stable APIs for downstream tasks.
- Non-Goals: Wiring locks into install/uninstall/cache flows, implementing UI feedback, or removing legacy helpers outside the new subsystem.
- Assumptions: Rust 1.89+ toolchain, CI matrix available for Linux/macOS/Windows, ADR-8mnaz remains authoritative.
- Constraints: No `unsafe` code, English messaging only, adhere to naming guidance (no “manager/util”), keep dependency footprint minimal.

## ADR & Legacy Alignment

- [x] Referenced ADR-8mnaz in module docs and ensured design parity.
- [x] Catalogued legacy `fs2` references for retirement tasks (T-9r1su) and marked follow-ups.

## Plan Summary

- Phase 1 – Foundation scaffolding
- Phase 2 – Advisory locking core
- Phase 3 – Fallback & hygiene hardening

> **Status Tracking:** Checkboxes capture current progress; deferred items note pending prerequisites.

---

## Phase 1: Foundation Scaffolding

### Goal

Provide filesystem classification, coordinate slug generation, configuration defaults, and path helpers required by the locking controller.

### Inputs

- Documentation: Design sections “FilesystemInspector”, “PackageCoordinate”, ADR-8mnaz requirements.
- Source:
  - `src/locking/filesystem.rs` (new)
  - `src/locking/package_coordinate.rs` (new)
  - `src/paths/mod.rs`
  - `src/config/mod.rs`
- Dependencies: `nix` (Unix), existing Windows bindings.

### Tasks

- [x] Implement `FilesystemInspector` with Unix `statfs` and Windows `GetVolumeInformationW`/`GetDriveTypeW`.
- [x] Add `PackageCoordinate` slug builder and helpers for install/cache lock paths.
- [x] Introduce `locking.mode` (default `auto`) and `locking.timeout` (default 600 s) with CLI/env/config precedence.
- [x] Add unit tests covering filesystem classification and slug generation edge cases.

### Deliverables

- Filesystem inspector module with tests.
- Coordinate slug + path helpers.
- Configuration defaults documented in code and sample config.

### Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet locking::filesystem locking::package_coordinate
```

### Acceptance Criteria (Phase Gate)

- Filesystem inspector identifies ext4, APFS, NTFS, CIFS, NFS, WSL mounts accurately in tests.
- Slug builder produces deterministic lock filenames with unit coverage.
- Configuration loads defaults when unset and honours overrides.

### Rollback/Fallback

- Re-export legacy helpers if slug generation introduces regressions (feature flag `locking_new_paths`).

---

## Phase 2: Advisory Locking Core

### Phase 2 Goal

Expose a cross-platform locking controller with RAII handles, timeout support, and structured diagnostics.

### Phase 2 Inputs

- Dependencies: Phase 1 utilities complete.
- Source:
  - `src/locking/controller.rs`
  - `src/locking/handle.rs`
  - `src/error.rs` (lock-specific variants)

### Phase 2 Tasks

- [x] Implement `LockController::acquire/try_acquire/release` selecting advisory vs fallback based on inspector results.
- [x] Create `LockHandle`/`LockGuard` RAII types capturing scope metadata.
- [x] Integrate timeout measurement using `std::time::Instant` and return `KopiError::LockingTimeout` with duration context.
- [x] Emit DEBUG logs for acquisition/release, INFO logs for contention and downgrade events.
- [x] Add unit tests covering shared/exclusive semantics, contention, timeout, downgrade to fallback stubs.

### Phase 2 Deliverables

- Lock controller API consumed by downstream tasks.
- Error and logging infrastructure aligned with ADR-8mnaz.

### Phase 2 Verification

```bash
cargo test --lib --quiet locking::controller locking::handle
```

### Phase 2 Acceptance Criteria

- Shared locks allow concurrent readers; exclusive locks block writers in tests.
- Timeout path returns actionable error; logs include wait duration and resource slug.
- Downgrade path emits a single INFO log per mount.

### Phase 2 Rollback/Fallback

- Hide controller behind `locking_controller` feature flag if additional validation is required before downstream adoption.

---

## Phase 3: Fallback & Hygiene Hardening

### Phase 3 Goal

Complete atomic fallback implementation, add hygiene routines, and validate lifecycle across supported platforms (current focus).

### Phase 3 Inputs

- Documentation: Design sections “AtomicFallback”, “LockHygieneRunner”, “Platform Considerations”.
- Source:
  - `src/locking/fallback.rs`
  - `src/locking/hygiene.rs`
  - `src/main.rs` (startup hook)
  - `tests/locking_lifecycle.rs`
- Dependencies: Phases 1 and 2 completed.

### Phase 3 Tasks

- [x] Implement atomic staging + rename fallback with marker files and INFO warnings.
- [x] Add hygiene runner scanning `~/.kopi/locks/` for stale fallback artefacts.
- [x] Add lifecycle integration tests (install/cache scopes, timeout, fallback, hygiene).
- [x] Add crash simulation harness (1,000 forced terminations) gated under `--ignored` for automated reliability runs.
- [x] Update CI matrix to run lifecycle tests on Linux, macOS, Windows, and WSL.
- [x] Document fallback behaviour in `docs/architecture.md` and `docs/error_handling.md`.

### Phase 3 Deliverables

- Fallback implementation ready for degraded filesystems.
- Hygiene runner integrated into CLI startup.
- Lifecycle test suite validating core scenarios.

### Phase 3 Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet locking::fallback locking::hygiene
cargo test --test locking_lifecycle -- --ignored crash_simulation  # pending runner capacity
```

### Phase 3 Acceptance Criteria

- Atomic fallback leaves no stale artefacts after simulated crashes.
- Hygiene runner removes synthetic leftovers and logs summary metrics.
- Lifecycle tests pass across supported platforms once CI runner is available.

### Phase 3 Rollback/Fallback

- Provide config `locking.mode = fallback` to disable advisory path if platform issues arise; document temporary guidance.

---

## Testing Strategy

### Unit Tests

- Maintain focused unit tests for filesystem inspector, controller, fallback, hygiene modules covering success/error paths.

### Integration Tests

- `tests/locking_lifecycle.rs` exercises install/cache scopes, contention, timeout, fallback, and hygiene flows.
- Crash simulations (ignored by default) validate cleanup reliability under forced termination.

### External API Parsing (if applicable)

- N/A – no external JSON parsing.

### Performance & Benchmarks (if applicable)

- Track lock acquisition latency and CPU overhead during waits; record benchmarks to ensure compliance with NFR-z6kan.

## Documentation Impact

- Update `docs/architecture.md`, `docs/error_handling.md`, and task README.
- Provide migration notes for downstream tasks adopting the controller API.

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../../templates/README.md#plan-template-planmd) in the templates README.
