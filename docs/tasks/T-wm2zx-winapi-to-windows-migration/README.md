# T-wm2zx Winapi to Windows Crate Migration Task

## Metadata

- Type: Task
- Status: Draft
  <!-- Draft: Under discussion | In Progress: Actively working | Complete: Code complete | Cancelled: Work intentionally halted -->

## Links

- Related Analyses:
  - N/A – Pending discovery
- Related Requirements:
  - [FR-rxelv-file-in-use-detection](../../requirements/FR-rxelv-file-in-use-detection.md)
- Related ADRs:
  - N/A – None identified
- Associated Design Document:
  - N/A – Design not started
- Associated Plan Document:
  - N/A – Plan not started

## Summary

Track the migration from the legacy `winapi` crate to the modern `windows` crate so Kopi aligns with maintained Windows bindings while reducing unsafe and duplicate FFI surface.

## Scope

- In scope:
  - Catalogue current `winapi` usages across the codebase and identify replacement APIs in the `windows` crate.
  - Define a phased rollout strategy that keeps existing Windows functionality stable during the transition.
  - Document required build configuration updates and potential feature flag implications.
- Out of scope:
  - Immediate implementation of the migration (will follow after task approval).
  - Changes to non-Windows platform bindings.

## Success Metrics

- Migration roadmap documented and approved with clear phase boundaries.
- Risk register and compatibility notes published for downstream maintainers before implementation starts.

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../../templates/README.md#task-template-taskmd) in the templates README.
