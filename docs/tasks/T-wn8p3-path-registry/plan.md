# T-wn8p3 Path Registry Consolidation Plan

## Metadata

- Type: Implementation Plan
- Status: Complete
  <!-- Draft: Planning complete, awaiting start | Phase X In Progress: Actively working | Cancelled: Work intentionally halted before completion | Complete: All phases done and verified -->

## Links

- Associated Design Document:
  - [T-wn8p3-path-registry-design](./design.md)
- Related Requirements:
  - [FR-hq1ns-canonical-path-registry](../../requirements/FR-hq1ns-canonical-path-registry.md)
  - [NFR-4sxdr-path-layout-compatibility](../../requirements/NFR-4sxdr-path-layout-compatibility.md)
- Related Analysis:
  - [AN-uqva7-path-registry-consolidation](../../analysis/AN-uqva7-path-registry-consolidation.md)

## Overview

Execute the path registry redesign by introducing a consolidated `src/paths/` hierarchy, migrating all consumers onto the new helpers, and validating that Kopi’s on-disk layout remains unchanged across platforms. Work is staged to minimise regression risk and to maintain traceability with FR-hq1ns and NFR-4sxdr.

## Success Metrics

- [x] All Kopi home path joins across installation, cache, shim, locking, and configuration align with the new helpers (guarded by `tests/paths_enforcement.rs`).
- [x] Compatibility regression suite demonstrates identical path outputs and CLI behaviour before and after migration (maintained from Phase 2; no deviations introduced in Phase 3).
- [x] Documentation (internal architecture reference and task README) highlights the `paths` module as the canonical source.
- [x] All standard checks pass (`cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test --lib --quiet`).

## Scope

- Goal: Centralise Kopi home path derivation via `src/paths/` while preserving behaviour.
- Non-Goals: Introducing new directory structures, altering user-facing configuration schema, or modifying external documentation beyond references to the new module.
- Assumptions: `PackageCoordinate` slugging remains authoritative; no additional external dependencies are required.
- Constraints: Must align with lock strategy (ADR-8mnaz) and project naming conventions; no `unsafe` allowed.

## ADR & Legacy Alignment

- [x] Confirm alignment with ADR-8mnaz (locking paths) and reference it in design.
- [x] During implementation, retire remaining `PathBuf::join("locks")` patterns; track via Phase 2 checklist. (Tests updated on 2025-10-21 to remove direct joins.)

## Plan Summary

- Phase 1 – Foundations & Shared Utilities
- Phase 2 – Consumer Migrations & Feature Parity
- Phase 3 – Hardening, Enforcement, and Documentation

> **Status Tracking:** Mark checkboxes (`[x]`) immediately after completing each task or subtask. If an item is intentionally skipped or deferred, annotate it (e.g., strike-through with a brief note) instead of leaving it unchecked.

---

## Phase 1: Foundations & Shared Utilities

### Goal

Lay down the `src/paths/` module structure, extract shared sanitisation helpers, and provide baseline APIs for consumers without yet migrating call sites.

### Inputs

- Documentation:
  - `docs/tasks/T-wn8p3-path-registry/design.md` – Architectural blueprint
  - `docs/analysis/AN-uqva7-path-registry-consolidation.md` – Gap inventory
- Source Code to Modify:
  - `src/paths/mod.rs` – Re-export surface
  - New domain modules under `src/paths/`
  - `src/locking/package_coordinate.rs` – Reuse sanitisation
- Dependencies:
  - Internal: `crate::locking`, `crate::platform` for shim extensions

### Tasks

- [x] **Module scaffolding**
  - [x] Create `src/paths/shared.rs` with slugging and directory utilities.
  - [x] Create domain modules (`install.rs`, `cache.rs`, `shims.rs`, `home.rs`) with placeholder functions returning existing paths.
- [x] **Baseline tests**
  - [x] Add unit tests covering slugging parity with `sanitize_segment`.
  - [x] Add golden-path tests for new helper functions using temp directories.

### Deliverables

- New `src/paths/` hierarchy with shared utilities and public API surface.
- Unit tests verifying foundational helpers.

### Verification

```bash
cargo check
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet paths::
```

### Acceptance Criteria (Phase Gate)

- Public helpers mirror existing path outputs for at least one representative case per domain (install, cache, shims, locks).
- Tests demonstrate sanitisation parity with legacy logic.

### Rollback/Fallback

- Remove new modules and revert `paths::mod.rs` to locking-only export; legacy behaviour remains untouched.

---

## Phase 2: Consumer Migrations & Feature Parity

### Phase 2 Goal

Switch core subsystems to the canonical helpers while ensuring behaviour parity and collecting path regression data.

### Phase 2 Inputs

- Dependencies:
  - Phase 1 helpers and tests
  - Requirements FR-hq1ns & NFR-4sxdr for acceptance criteria
- Source Code to Modify:
  - `src/storage/` modules (repository, installation)
  - `src/shim/installer.rs`, `src/shim/security.rs`
  - `src/doctor/checks/*`
  - `src/commands/*` referencing Kopi home paths
- Tooling:
  - `rg` inventories captured in analysis raw data

### Phase 2 Tasks

- [x] **Subsystem migrations**
  - [x] Replace installation path joins with `paths::install` helpers; update tests.
  - [x] Migrate cache metadata, temp files, and doctor checks to `paths::cache`.
  - [x] Update shim logic to rely on `paths::shims` for directories and tool filenames.
  - [x] Ensure locking scope reuses refactored helpers.
- [x] **Compatibility validation**
  - [x] Capture before/after path snapshots using integration tests.
  - [x] Run CLI smoke suite to confirm unchanged output.

### Phase 2 Deliverables

- All major subsystems relying on canonical helpers.
- Regression evidence (snapshot logs, test output) demonstrating unchanged paths.

### Phase 2 Verification

```bash
cargo test --lib --quiet
cargo test --quiet storage::repository::tests
cargo test --quiet shim::installer::tests
```

### Phase 2 Acceptance Criteria

- No direct `PathBuf::join("jdks")` / `join("shims")` / `join("cache")` invocations remain outside the paths module.
- Compatibility tests pass across Unix and Windows runners (CI confirmation required).

### Rollback/Fallback

- Revert subsystem migrations individually, leaving helpers in place for future attempts; maintain branch with snapshots for later comparison.

---

## Phase 3: Hardening, Enforcement, and Documentation

### Phase 3 Goal

Finalize documentation, enforce usage of helpers, and remove transitional code or instrumentation.

### Phase 3 Inputs

- Dependencies:
  - Completed Phase 2 migrations
  - Documentation references requiring updates (`docs/architecture.md`, `docs/reference.md`)
- Source Code to Modify:
  - `docs/architecture.md`, `docs/reference.md`
  - Optional lint configuration (Clippy allow/deny lists)

### Phase 3 Tasks

- [x] **Documentation updates**
  - [x] Document the `paths` module hierarchy and responsibilities. (Updated `docs/architecture.md` and `docs/reference.md` on 2025-10-21.)
  - [x] Update task README links to include design and plan references. (Links verified on 2025-10-21.)
- [x] **Enforcement tooling**
  - [x] Add a CI check or Clippy lint guard (e.g., deny `join("jdks")`) or document code review checklist if lint infeasible. (Added `tests/paths_enforcement.rs`.)
- [x] **Cleanup**
  - [x] Remove temporary instrumentation or compatibility logging. (No temporary logging remained after Phase 2; verified via repository scan on 2025-10-21.)
  - [x] Ensure traceability matrices reference FR-hq1ns and NFR-4sxdr. (Refreshed with `bun scripts/trace-status.ts --write` on 2025-10-21.)

### Phase 3 Deliverables

- Updated documentation and linting/check guidance ensuring ongoing compliance.
- Final traceability updates (task README, trace matrix, release notes).

### Phase 3 Verification

```bash
bun format
bun lint
bun scripts/trace-status.ts --write
```

### Phase 3 Acceptance Criteria

- Documentation and traceability artifacts accurately describe the canonical paths module.
- Helper usage is enforced via lint/checklist with no remaining direct path literals in new code.

### Rollback/Fallback

- If lint proves noisy, document expectations and rely on review checklist while tracking a follow-up to implement automation later.

---

## Platform Matrix (if applicable)

### Unix

- Validate shim symlink behaviour and permissions remain unchanged.

### Windows

- Verify `.exe` shim generation persists and UNC path handling remains correct.

### Filesystem

- Exercise tests against case-sensitive (ext4) and case-insensitive (NTFS) environments; confirm temp directory cleanup.

---

## Dependencies

### External Crates

- None beyond existing dependencies; rely on standard library path utilities.

### Internal Modules

- `src/locking` – Reuse sanitisation logic.
- `src/platform` – Determine executable extensions and symlink creation functions.

---

## Risks & Mitigations

1. Risk: Missed call sites continue using ad hoc joins.
   - Mitigation: Scripted `rg` sweeps per subsystem; review checklist.
   - Validation: CI fails if lints/checklist not satisfied; manual diff review.
   - Fallback: Add legacy helper wrappers emitting warnings and track deprecation.

2. Risk: Path compatibility divergence on Windows due to slug differences.
   - Mitigation: Cross-platform snapshot tests in Phase 2; ensure sanitisation matches existing implementation byte-for-byte.
   - Validation: Compare outputs from Windows CI runs; manual verification on local Windows environment.
   - Fallback: Gate Windows rollout behind feature flag until fixed.

---

## Documentation & Change Management

- Update `docs/architecture.md` and `docs/reference.md` to reflect the new module once Phase 3 completes.
- Notify the documentation repository maintainers (`../kopi-vm.github.io/`) if any developer-facing references need syncing.
- Ensure release notes emphasise compatibility guarantee.

---

## Implementation Guidelines

- Maintain English error messages and leverage `KopiError` consistently.
- Avoid generic naming like "manager" or "util"; prefer descriptive module/function names (e.g., `paths::cache::metadata_cache_file`).
- Follow memory safety conventions; no `unsafe`, no `Box::leak()`.

---

## Definition of Done

- [x] `cargo check`
- [x] `cargo fmt`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib --quiet`
- [x] Platform verification complete (Unix + Windows regression evidence — no new platform behaviour introduced in Phase 3.)
- [x] Documentation updated (`docs/architecture.md`, `docs/reference.md`, task README)
- [x] Traceability matrix refreshed (`bun scripts/trace-status.ts --write`)

---

## External References (optional)

- [Rust Path API documentation](https://doc.rust-lang.org/std/path/) – Path joining semantics relied upon for helpers.

## Open Questions

- [ ] Should we automate detection of new ad hoc path joins via a custom Clippy lint or git hook? → Investigate during Phase 3 enforcement task.
- [ ] Do we need migration guidance for third-party plugins consuming internal modules? → Confirm during subsystem migration reviews.

---

## Visual/UI Reference (optional)

```text
Phase 1: Build helpers ──▶ Phase 2: Migrate consumers ──▶ Phase 3: Harden & document
```

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../../templates/README.md#plan-template-planmd) in the templates README.
