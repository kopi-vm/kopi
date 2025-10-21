# T-wn8p3 Path Registry Redesign Task

## Metadata

- Type: Task
- Status: Draft
  <!-- Draft: Under discussion | In Progress: Actively working | Complete: Code complete | Cancelled: Work intentionally halted -->

## Links

- Related Analyses:
  - [AN-wn8p3-path-registry-consolidation](../../analysis/AN-wn8p3-path-registry-consolidation.md)
- Related Requirements:
  - [FR-hq1ns-canonical-path-registry](../../requirements/FR-hq1ns-canonical-path-registry.md)
  - [NFR-4sxdr-path-layout-compatibility](../../requirements/NFR-4sxdr-path-layout-compatibility.md)
- Related ADRs:
  - N/A – None
- Associated Design Document:
  - [T-wn8p3-path-registry/design.md](./design.md)
- Associated Plan Document:
  - [T-wn8p3-path-registry/plan.md](./plan.md)

## Summary

Redesign Kopi’s filesystem path handling so installation, cache, shim, and locking directories are derived from a single documented module (`src/paths/`) instead of ad-hoc constructors.

## Scope

- In scope:
  - Catalogue existing path construction across the codebase.
  - Create shared helpers in `src/paths` and migrate call sites.
  - Update documentation to reference the shared helpers.
  - Review and restructure locking workflow in `src/paths/locking.rs`.
- Out of scope:
  - Introducing new directories.
  - Changing on-disk defaults.
  - Altering user-facing configuration schema beyond using shared helpers.

## Success Metrics

- Centralised helpers: All path-building code for Kopi home subdirectories routes through `src/paths` with unit coverage.
- Behaviour parity: Regression suite confirms no change to on-disk layout or CLI output compared with the baseline.
- Documentation: Developer docs reference the new module as the canonical source for path utilities.

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../../templates/README.md#task-template-taskmd) in the templates README.
