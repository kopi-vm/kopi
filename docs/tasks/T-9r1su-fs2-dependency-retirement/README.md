# T-9r1su FS2 Dependency Retirement Task

## Metadata

- Type: Task
- Status: Complete
  <!-- Draft: Under discussion | In Progress: Actively working | Complete: Code complete | Cancelled: Work intentionally halted -->

## Links

- Associated Design Document:
  - [T-9r1su-fs2-dependency-retirement-design](./design.md)
- Associated Plan Document:
  - [T-9r1su-fs2-dependency-retirement-plan](./plan.md)

## Summary

Retire the `fs2` crate by migrating disk space checks to `sysinfo` and file-in-use detection to standard library locks, eliminating the dependency while preserving user-facing diagnostics and aligning with ADR-8mnaz.

## Scope

- In scope:
  - Replace `fs2` usage across disk checks and locking helpers.
  - Remove the dependency from manifests and update documentation.
  - Validate cross-platform behaviour with regression coverage.
- Out of scope:
  - Broader refactors unrelated to disk space or locking.
  - Removal of `sysinfo`.
  - Changes to external tooling.

## Success Metrics

- `fs2` removal: Manifests and code contain no references to the crate.
- Functional parity: Disk space reporting and file-in-use detection match historical behaviour on macOS, Linux, and Windows.
- Traceability: Requirements FR-x63pa and FR-rxelv marked complete with supporting tests and documentation.

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../../templates/README.md#task-template-taskmd) in the templates README.
