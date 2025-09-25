# Path Registry Refactor Task

## Metadata

- Type: Task
- Status: Proposed

## Links

- Related Analyses:
  - N/A – Not yet created
- Related Requirements:
  - N/A – To be confirmed during analysis
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Associated Plan Document:
  - N/A – Not started
- Associated Design Document:
  - N/A – Not started

## Summary

Refactor Kopi’s filesystem path handling so that installation, cache, shim, and locking directories are derived from a single, well-documented module (`src/paths/`) instead of being assembled piecemeal throughout the codebase.

## Scope

- In scope: Catalog all current path construction sites, move shared logic into `src/paths`, and update call sites to use the centralized helpers while maintaining existing behavior.
- Out of scope: Introducing new directories, changing default locations, or modifying user-facing configuration schema beyond using the shared helpers.

## Success Metrics

- `Centralized helpers`: All path-building code for Kopi home subdirectories goes through `src/paths` functions with unit coverage.
- `Behavior parity`: Regression suite and targeted snapshots confirm no change to on-disk layout or CLI output compared to the pre-refactor baseline.
- `Documentation`: Developer documentation references the new module as the canonical place for path-related utilities.

## Detailed Plan

- Inventory existing path creation logic (storage, shims, cache, locking, metadata) and capture gaps in an analysis note.
- Introduce path helper functions and types in `src/paths`, covering standard directories and common file names.
- Incrementally migrate modules (config, installation, cache, shim, locking) to consume the centralized helpers, adding tests where coverage is missing.
- Update developer docs and error-handling guidance to reference the shared path utilities and note any required migration steps for contributors.

## Notes

- Task raised after introducing `src/paths/mod.rs` for locking; the broader refactor is deferred to avoid scope creep in the current locking foundation work.

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../../templates/README.md#task-template-taskmd) in the templates README.
