# Locking Foundation Implementation Plan

## Metadata

- Type: Implementation Plan
- Status: Phase 1 In Progress

## Links

- Related Requirements:
  - [FR-02uqo-installation-locking](../../requirements/FR-02uqo-installation-locking.md)
  - [FR-ui8x2-uninstallation-locking](../../requirements/FR-ui8x2-uninstallation-locking.md)
  - [FR-v7ql4-cache-locking](../../requirements/FR-v7ql4-cache-locking.md)
  - [FR-gbsz6-lock-timeout-recovery](../../requirements/FR-gbsz6-lock-timeout-recovery.md)
  - [FR-c04js-lock-contention-feedback](../../requirements/FR-c04js-lock-contention-feedback.md)
  - [NFR-g12ex-cross-platform-compatibility](../../requirements/NFR-g12ex-cross-platform-compatibility.md)
  - [NFR-vcxp8-lock-cleanup-reliability](../../requirements/NFR-vcxp8-lock-cleanup-reliability.md)
  - [NFR-z6kan-lock-timeout-performance](../../requirements/NFR-z6kan-lock-timeout-performance.md)
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Related Design:
  - [docs/tasks/T-ec5ew-locking-foundation/design.md](./design.md)

## Overview

Deliver the locking foundation described in the design: filesystem-aware advisory locking with package-specific install locks, global cache locking, configuration defaults, and hygiene routines that satisfy the linked requirements and ADR.

## Success Metrics

- [ ] Advisory locking API passes lifecycle tests on Linux, macOS, Windows, and WSL runners.
- [ ] Hygiene sweep deletes 100% of synthetic fallback artifacts in crash simulations (1000 iterations).
- [ ] Network filesystem detection downgrades to fallbacks with INFO warnings and no panics in automated scenarios.
- [ ] `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --lib --quiet` pass before completion.

## Scope

- Goal: Implement locking components (controller, filesystem inspector, advisory backend, fallback strategy, hygiene runner) and supporting configuration (`locking.mode`, `locking.timeout`) plus package-coordinate slug generation.
- Non-Goals: Integrating the new API into install/uninstall/cache commands, modifying user-facing CLI flags beyond configuration defaults, or removing existing `fs2` helpers outside the new subsystem.
- Assumptions: Rust 1.89+ baseline, CI matrix already provisioned, design decisions in ADR-8mnaz remain valid.
- Constraints: No `unsafe`, keep dependencies minimal, all messaging in English, align naming with "no manager/util" guidance.

## Plan Summary

- Phase 1 – Foundation scaffolding (filesystem detection, package coordinates, configuration, path helpers)
- Phase 2 – Advisory locking core (controller, handles, error surface, logging)
- Phase 3 – Fallback + hygiene hardening (atomic flows, cleanup runner, CI coverage, stress tests)

---

## Phase 1: Filesystem & Coordinate Foundation

### Goal

Provide the shared utilities required by later phases: filesystem classification, package-based slug generation, configuration defaults, and lock-path helpers.

### Inputs

- Documentation:
  - `docs/tasks/T-ec5ew-locking-foundation/design.md` – Component responsibilities and storage layout.
  - `docs/adr/ADR-8mnaz-concurrent-process-locking-strategy.md` – Filesystem and fallback expectations.
- Source Code to Modify:
  - `src/platform/filesystem.rs` (new) – Filesystem inspector implementations, re-exported through `src/locking`.
  - `src/locking/package_coordinate.rs` (new) – Slug builder for install locks.
  - `src/config/mod.rs` + related config files – Introduce `locking.mode` (default `auto`) and `locking.timeout` (default `600s`).
  - `src/paths/mod.rs` – Expose lock root and helper functions for install/cache lock paths.
- Dependencies:
  - Internal: `src/platform/` for existing OS abstractions.
  - External: `nix` crate for `statfs` (Unix), `winapi` crate for filesystem metadata (existing dependency)

### Tasks

- [ ] **Filesystem inspector**
  - [ ] Implement `FilesystemInspector` trait with Unix `statfs` mapping and Windows `GetVolumeInformationW` + `GetDriveTypeW` logic.
  - [ ] Store classification results in a thread-safe cache keyed by canonical mount path.
- [ ] **Package coordinate & paths**
  - [ ] Introduce `PackageCoordinate` struct covering distribution, version, package type (JDK/JRE), JavaFX flag, architecture, and variant metadata.
  - [ ] Generate deterministic install lock slugs (e.g., `temurin-21-jdk-x64-javafx`) and expose path helpers that create `~/.kopi/locks/install/<distribution>/<slug>.lock`.
  - [ ] Expose cache lock path helper for `~/.kopi/locks/cache.lock`.
- [ ] **Configuration defaults**
  - [ ] Add `locking.mode` and `locking.timeout` to configuration structs, CLI/env plumbing, and documentation comments.
  - [ ] Persist defaults (`auto`, `600s`) and ensure serde (or equivalent) deserialization works.

### Deliverables

- New filesystem inspector module with unit tests covering ext4, APFS, NTFS, FAT32, CIFS/SMB, NFS cases (using fixtures/mocks).
- `PackageCoordinate` + slug helpers with tests validating differentiation between JDK/JRE, JavaFX, architecture, and vendor.
- Configuration defaults wired into existing config loading path.

### Verification

```bash
cargo check
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet locking::filesystem locking::package_coordinate config::locking
```

### Acceptance Criteria (Phase Gate)

- Inspector correctly categorizes supported and fallback-required filesystems in tests.
- Install and cache lock path helpers produce canonical paths matching the design layout.
- Configuration keys load defaults, honor overrides, and round-trip through existing config serialization tests.

### Rollback/Fallback

- Revert new modules, leaving existing behavior untouched; retain package coordinate code behind feature flag if partial work needs to land without affecting runtime.

---

## Phase 2: Advisory Locking Core

### Goal

Implement the advisory locking layer: controller orchestration, RAII handles, timeout handling, and structured logging using the Phase 1 utilities.

### Inputs

- Documentation: Design sections “Proposed Design” (Components/Data Flow) and “Error Handling”.
- Source Code to Modify:
  - `src/locking/mod.rs`, `src/locking/controller.rs`, `src/locking/handle.rs` (new) – Core APIs with platform hooks imported from `src/platform`.
  - `src/error/mod.rs` + `src/error/kopi_error.rs` – Add `KopiError::Locking` variants and contexts.
  - `src/config/` – Ensure controller consumes `locking.mode` and `locking.timeout`.
  - `src/lib.rs` – Export locking module to callers.
- Dependencies: Phase 1 outputs, existing logging infrastructure (`log` crate).

### Tasks

- [ ] **Controller API**
  - [ ] Implement `LockController::acquire`, `try_acquire`, and `release`, selecting advisory vs fallback based on filesystem classification and lock scope.
  - [ ] Track acquisition timing, enforce timeout budgets, and return informative `KopiError::LockingTimeout` on expiry.
- [ ] **Handles & RAII**
  - [ ] Implement `LockHandle` and `FallbackHandle` with `Drop`-based cleanup, storing scope metadata for logging/debug.
  - [ ] Surface acquisition/release events at DEBUG level with duration and scope.
- [ ] **Error surface**
  - [ ] Define `KopiError::Locking*` variants with actionable English messages.
  - [ ] Map lower-level IO errors into context-rich results for callers.
- [ ] **Unit tests**
  - [ ] Cover happy path (shared/exclusive), contention, timeout, and downgrade flows with temporary directories.

### Deliverables

- Locking controller module with complete unit tests.
- Updated error enumeration and `ErrorContext` integration for locking failures.
- Logging hooks emitting structured messages aligned with design.

### Verification

```bash
cargo check
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet locking::controller locking::handle
```

### Acceptance Criteria

- Shared locks allow concurrent readers; exclusive locks prevent concurrent writers in tests.
- Timeout path returns `KopiError::LockingTimeout` including waited duration.
- Downgrade to fallback is recorded once per mount with INFO log, no panics.

### Rollback/Fallback

- Feature-gate the controller exports and keep Phase 1 utilities available if advisory implementation needs more time.

---

## Phase 3: Fallback Strategy & Hygiene

### Goal

Complete the fallback path for unsupported filesystems, add hygiene cleanup, and validate end-to-end scenarios across platforms.

### Inputs

- Documentation: Design sections “AtomicFallback”, “LockHygieneRunner”, “Platform Considerations”, and Appendix algorithms.
- Source Code to Modify:
  - `src/locking/fallback.rs`, `src/locking/hygiene.rs` (new) – Atomic locking and cleanup using platform utilities from `src/platform`.
  - `src/main.rs` (CLI entrypoint) – Invoke hygiene runner once during process startup.
  - `tests/locking_lifecycle.rs` (new integration test) – Install/cache scopes, contention, timeout, fallback.
  - CI workflows – Ensure matrix jobs cover Linux/macOS/Windows/WSL scenarios.
- Dependencies: Completed Phases 1 and 2.

### Tasks

- [ ] **Fallback implementation**
  - [ ] Implement atomic staging + rename sequence, using marker files to avoid deleting active locks.
  - [ ] Emit INFO warnings detailing filesystem kind and fallback mode.
- [ ] **Hygiene runner**
  - [ ] Sweep `~/.kopi/locks/` for stale fallback artifacts (respecting age thresholds and marker files).
  - [ ] Log summary metrics (count removed, duration) at DEBUG level.
- [ ] **Integration & stress tests**
  - [ ] Add `tests/locking_lifecycle.rs` covering install, cache, timeout, fallback, and hygiene flows.
  - [ ] Add crash simulation harness (1000 forced termination iterations) gated under `--ignored` to validate cleanup reliability.
  - [ ] Wire tests into CI matrix for Linux, macOS, Windows, WSL.
- [ ] **Documentation updates**
  - [ ] Update `docs/architecture.md` and `docs/error_handling.md` per design documentation impact.

### Deliverables

- Fallback and hygiene modules with unit/integration tests.
- CI matrix updates ensuring locking tests run on all target platforms.
- Documentation changes reflecting new subsystem behavior.

### Verification

```bash
cargo check
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet locking::fallback locking::hygiene
cargo test --test locking_lifecycle -- --ignored --nocapture
bun format
bun lint
```

### Acceptance Criteria

- Fallback locking guarantees exclusive access on unsupported filesystems without data loss in tests.
- Hygiene runner removes all synthetic stale artifacts within one startup run and logs cleanup summary.
- CI matrix executes locking lifecycle suite successfully across platforms.

### Rollback/Fallback

- Feature-gate fallback/hygiene while leaving advisory locking available; document temporary limitations if fallback must be deferred.

---

## Platform Matrix

### Unix

- Validate `statfs` detection codes and fallback transitions on ext4, xfs, btrfs, APFS via integration tests.
- Ensure hygiene respects permissions and avoids crossing symlink boundaries.

### Windows

- Confirm `GetVolumeInformationW` classification for NTFS vs network shares.
- Verify handles open with proper sharing flags to avoid permission errors.

### Filesystem

- Maintain allowlist/denylist tables with unit tests; add logging for unknown filesystems to aid diagnostics.

---

## Dependencies

### External Crates

- `nix` – Unix filesystem metadata (existing dependency).
- `winapi` – Windows filesystem metadata (reuse existing pins).

### Internal Modules

- `src/platform/` – Platform abstractions reused across inspector implementations.
- `src/error/` – Shared error handling.
- `src/config/` – Central configuration definitions and defaults.

---

## Risks & Mitigations

1. Risk: Filesystem misclassification leading to incorrect fallback decisions.
   - Mitigation: Extensive unit tests with captured metadata, CI matrix coverage, logging unknown types.
   - Validation: Inspect logs during integration tests; ensure downgrade occurs only on unsupported mounts.
   - Fallback: Force fallback mode via configuration flag when classification uncertain.
2. Risk: Hygiene routine deleting active fallback artifacts.
   - Mitigation: Marker files + age thresholds; check active handles before removal.
   - Validation: Crash simulation test asserts no active locks removed.
   - Fallback: Allow disabling hygiene via config until fixed.
3. Risk: Startup latency increase due to hygiene sweep.
   - Mitigation: Limit scan scope, reuse cached timestamps, parallelize where safe.
   - Validation: Benchmark sweep duration (<50 ms typical) and document results.
   - Fallback: Run hygiene asynchronously post-startup if latency exceeds target.

---

## Documentation & Change Management

- Update `docs/architecture.md` and `docs/error_handling.md` with new locking subsystem details and error variants.
- Coordinate any user-facing documentation updates (`../kopi-vm.github.io/`) in downstream tasks once CLI wiring occurs.
- Ensure `docs/reference.md` references new configuration keys if surfaced to users.

---

## Implementation Guidelines

### Error Handling

- Use `KopiError::Locking*` variants and `ErrorContext` for actionable, English messages with correct exit codes.

### Naming & Structure

- Use descriptive names (`LockController`, `LockHygieneRunner`, `PackageCoordinate`); avoid "manager"/"util" patterns.

### Safety & Clarity

- No `unsafe` code; prioritize readability and memory safety over micro-optimizations.

---

## Definition of Done

- [ ] `cargo check`
- [ ] `cargo fmt`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --lib --quiet`
- [ ] `cargo test --test locking_lifecycle -- --ignored --nocapture`
- [ ] `bun format`
- [ ] `bun lint`
- [ ] Documentation updates merged (`docs/architecture.md`, `docs/error_handling.md`)
- [ ] CI matrix green (Linux, macOS, Windows, WSL)

---

## External References

- [Cargo lock implementation](https://github.com/rust-lang/cargo/blob/master/src/cargo/util/flock.rs) – Guidance for fallback handling.
- [Rust std::fs::File locking API](https://doc.rust-lang.org/std/fs/struct.File.html) – Advisory lock primitives.

## Open Questions
