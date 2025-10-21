# AN-wn8p3 Path Registry Consolidation Analysis

## Metadata

- Type: Analysis
- Status: Draft
  <!-- Draft: Initial exploration | Complete: Ready for requirements | Cancelled: Work intentionally halted | Archived: Analysis concluded -->

## Links

- Related Analyses:
  - N/A – Standalone analysis
- Related Requirements:
  - N/A – None yet
- Related ADRs:
  - N/A – None yet
- Related Tasks:
  - [T-wn8p3-path-registry](../tasks/T-wn8p3-path-registry/README.md)

## Executive Summary

Kopi’s filesystem paths are defined ad hoc across configuration, storage, shim, locking, and doctor modules. Hard-coded directory segments such as "jdks", "cache", and "shims" appear in dozens of locations, and responsibility for creating directories is split between `KopiConfig` and individual subsystems. The lack of a canonical path registry makes it difficult to audit on-disk locations, reason about lock placement, or document platform-specific behaviours. This analysis inventories the current state and recommends introducing a consolidated `src/paths/` module that owns path derivation, normalisation, and directory provisioning for all Kopi home subdirectories while preserving existing on-disk layouts.

## Problem Space

### Current State

- `KopiConfig` exposes helpers (`jdks_dir`, `cache_dir`, `bin_dir`, `shims_dir`) that each create directories and return `PathBuf`, but most callers still join subdirectories manually (e.g., `storage::repository::jdk_install_path`, `shim::installer::new`).
- Lock-related helpers already exist under `src/paths/locking.rs`, yet other components (doctor checks, metadata cache, shim security, version resolver) bypass `src/paths` entirely.
- String literals for Kopi home children are duplicated across modules ("locks", "install", "metadata.json", etc.), increasing the risk of drift when renaming directories or standardising new layout rules.
- Tests replicate the same literals, making refactors noisy and error prone.
- Documentation and developer onboarding material cannot point to a single authoritative source for filesystem structure, complicating support for new contributors.

### Desired State

- A dedicated `src/paths/` tree provides typed helpers for every Kopi home subdirectory and file of interest (install roots, cache artefacts, shim binaries, lock files, version markers).
- Directory naming and normalisation rules (slugging distributions, sanitising user input) are centralised, exposing simple APIs that accept `&Path` for Kopi home plus rich identifiers (e.g., `PackageCoordinate`).
- Directory creation semantics are explicit and consolidated: modules either receive already-prepared directories from the paths layer or consistently request on-demand creation through the same API.
- Developer documentation references the paths module as the canonical contract, keeping code and docs in sync.

### Gap Analysis

- No comprehensive inventory exists for path construction today; each subsystem implements its own joining logic.
- Inconsistent ownership of directory creation (sometimes `KopiConfig`, sometimes caller) leads to duplicated error handling and logging.
- Lack of shared abstractions makes it difficult to reason about locking scope propagation and concurrency guarantees across new features.
- Absent documentation of path invariants hinders regression detection when modifying on-disk structure.

## Stakeholder Analysis

| Stakeholder          | Interest/Need                                          | Impact | Priority |
| -------------------- | ------------------------------------------------------ | ------ | -------- |
| Kopi CLI Users       | Stable on-disk layout across upgrades and new features | High   | P0       |
| Core Maintainers     | Faster, safer refactors of filesystem responsibilities | High   | P0       |
| Documentation Team   | Single source of truth for developer docs and diagrams | Medium | P1       |
| Support & Operations | Ability to diagnose path-related issues from telemetry | Medium | P1       |

## Research & Discovery

### User Feedback

- Internal maintainers report frequent uncertainty about where new artefacts should live, especially when aligning with locking hygiene tasks. No external end-user tickets reference path confusion, but developer friction slows delivery.

### Competitive Analysis

- **Volta** exposes a `paths` module returning typed directories for toolchains, reducing duplication.
- **Rustup** maintains a single `Cfg::cargo_home` structure that centralises path derivation and directory provisioning.
- **pyenv** and **sdkman** rely on shell string concatenation, which Kopi seeks to improve upon for reliability and maintainability.

### Technical Investigation

- `rg "\.join\("jdks"" src` surfaces 39 direct joins, indicating pervasive duplication.
- `src/paths/mod.rs` currently exports only locking helpers, leaving other domains unrepresented.
- `shim::installer`, `storage::repository`, `doctor::checks`, and `metadata::provider` each contain bespoke path-normalisation logic, including sanitisation and canonicalisation steps that should be shared.
- Directory creation side effects are split among `KopiConfig`, installers, and doctor checks, leading to inconsistent error propagation (`KopiError::ConfigError`, `std::io::Error`, or ad hoc strings).

### Data Analysis

- Manual inspection confirms all path strings map to subdirectories under `$KOPI_HOME`, with no current abstractions for platform-specific overrides (e.g., Windows vs. Unix naming). Consolidation would make it easier to instrument metrics on directory creation failures.

## Discovered Requirements

> Capture potential requirements as solution-agnostic problem statements focused on the problem to solve rather than any specific implementation.

### Functional Requirements (Potential)

- [ ] **FR-DRAFT-1**: Provide a canonical API under `src/paths/` that returns Kopi home subdirectories and key files while encapsulating normalisation (slugging distributions, sanitising vendor strings) and directory creation semantics where needed.
  - Rationale: Central accessors eliminate hard-coded strings, simplify audits, and remove duplicated sanitisation and directory provisioning logic.
  - Acceptance Criteria: Every path join that currently references Kopi home directories routes through the new API; migration removes redundant sanitisation code and directory creation helpers from modules such as `storage::repository`, `shim::installer`, and `locking::scope`; unit tests cover each helper.

- [ ] **FR-DRAFT-2**: Document the canonical filesystem layout in developer docs referencing the new paths module.
  - Rationale: Support teams need authoritative diagrams and descriptions.
  - Acceptance Criteria: Documentation updates link to the module, and traceability ties requirements to T-wn8p3.

### Non-Functional Requirements (Potential)

- [ ] **NFR-DRAFT-1**: Preserve backward-compatible on-disk layout across Unix, Windows, and WSL environments.
  - Category: Reliability
  - Rationale: Users rely on existing directory names for scripts and backups.
  - Target: No change to effective paths emitted by integration tests across supported platforms.

## Design Considerations

### Technical Constraints

- Directory creation must respect existing error handling contracts (`KopiError::ConfigError` vs. subsystem-specific errors).
- Locking helpers depend on `PackageCoordinate` and `sanitize_segment`; the new module must avoid circular dependencies.
- Any change must remain compatible with `$KOPI_HOME` overrides and relative-to-absolute resolution logic in `KopiConfig`.

### Potential Approaches

1. **Expand `src/paths/` into a namespace of free functions grouped by domain** _(Recommended)_
   - Pros: Minimal disruption; mirrors existing locking helpers; easy to unit test.
   - Cons: Requires careful module organisation to avoid super-mod imports.
   - Effort: Medium.

2. **Introduce a `PathsRegistry` struct initialised with Kopi home**
   - Pros: Encapsulates state, enabling lazy directory creation policies.
   - Cons: Adds a new struct to pass around; may duplicate `KopiConfig` responsibilities.
   - Effort: Medium-High.

3. **Augment `KopiConfig` with comprehensive path helpers**
   - Pros: Leverages existing configuration ownership of Kopi home.
   - Cons: Keeps filesystem logic tied to configuration, limiting reuse in tests and utilities that lack full config context.
   - Effort: Medium.

### Architecture Impact

- Consolidation likely warrants an ADR clarifying ownership boundaries between configuration, paths, and locking modules.
- The implementation may affect module imports across storage, shim, and doctor components, requiring updates to maintain layering described in `docs/architecture.md`.

## Risk Assessment

| Risk                                                     | Probability | Impact | Mitigation Strategy                                          |
| -------------------------------------------------------- | ----------- | ------ | ------------------------------------------------------------ |
| Regression in directory creation leading to missing dirs | Medium      | High   | Provide dedicated tests for directory provisioning helpers.  |
| Circular dependencies between `paths` and other modules  | Medium      | Medium | Factor sanitisation utilities into shared submodules first.  |
| Overlooked call sites continue using hard-coded paths    | High        | Medium | Use `rg`-based audit and add Clippy lint to forbid literals. |
| Documentation drifts again after refactor                | Medium      | Medium | Tie documentation updates to traceability gates and CI.      |

## Open Questions

- [ ] Should directory creation remain in `KopiConfig` or shift entirely into the paths module? → Next step: Prototype both options and document the trade-offs in a draft ADR.
- [ ] How will integration tests verify that effective on-disk paths remain unchanged across platforms? → Method: Extend existing smoke tests with snapshot assertions.
- [ ] Can we enforce usage of the new helpers via linting or a Clippy allowlist? → Next step: Investigate custom lint or build-time check.

## Recommendations

### Immediate Actions

1. Catalogue all path joins under `$KOPI_HOME` and classify them by domain (installation, cache, shims, locking, config, telemetry).
2. Draft functional and non-functional requirements based on the discovered needs above.
3. Author an ADR defining the ownership and layering strategy for the forthcoming `src/paths` module.

### Next Steps

1. [ ] Create formal requirements: FR for the canonical path API (including normalisation and directory provisioning coverage) and NFR for compatibility.
2. [ ] Draft an ADR covering module boundaries and dependency expectations.
3. [ ] Prepare task design and plan documents under `T-wn8p3` once upstream artefacts are approved.
4. [ ] Evaluate automation (lint/tests) that enforce centralised path usage.

### Out of Scope

- Introducing new directories or altering Kopi’s on-disk layout beyond consolidating existing behaviour.
- Changing user-facing configuration schema for path overrides.
- Implementing telemetry or metrics beyond what is required to verify consolidation outcomes.

## Appendix

### References

- `src/config.rs` for existing directory helper implementations.
- `src/paths/locking.rs` for current centralised locking paths.
- `src/shim/installer.rs`, `src/storage/repository.rs`, `src/doctor/checks/*` for representative ad hoc path joins.
- Volta and Rustup path helper patterns (internal knowledge base).

### Raw Data

- `rg "\.join\("jdks"" src` → 39 matches (snapshot 2025-10-21).
- `rg "\.join\("shims"" src` → 12 matches (snapshot 2025-10-21).
- `rg "\.join\("cache"" src` → 7 matches (snapshot 2025-10-21).

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../templates/README.md#analysis-template-analysismd) in the templates README.
