# T-98zsb Uninstallation Lock Integration Task

## Metadata

- Type: Task
- Status: Complete
  <!-- Draft: Under discussion | In Progress: Actively working | Complete: Code complete | Cancelled: Work intentionally halted -->

## Links

- Related Analyses:
  - [AN-m9efc-concurrent-process-locking](../../analysis/AN-m9efc-concurrent-process-locking.md)
- Related Requirements:
  - [FR-ui8x2-uninstallation-locking](../../requirements/FR-ui8x2-uninstallation-locking.md)
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Associated Design Document:
  - [T-98zsb-uninstallation-locking-design](./design.md)
- Associated Plan Document:
  - [T-98zsb-uninstallation-locking-plan](./plan.md)

## Summary

Apply exclusive locking and timeout-aware coordination to the uninstallation workflow to guarantee atomic removal of JDKs while preventing conflicts with concurrent installations or other uninstallations. Phase 4 adds cross-process contention tests that prove uninstall operations block on both peer uninstallers and in-flight installs before proceeding.

## Scope

- In scope:
  - Reuse installation lock keys and wrap uninstallation phases with exclusive locks.
  - Coordinate with timeout and feedback observers for shared behaviour.
  - Validate rollback paths when failures occur.
- Out of scope:
  - Installation pipeline changes.
  - Cache refresh handling.
  - New UI features beyond shared feedback interfaces.

## Success Metrics

- [x] Atomic removal: Tests verify uninstallation either completes fully or rolls back without leaving partial directories or metadata.
- [x] Concurrent safety: Mixed install/uninstall operations respect timeout and cancellation policies with accurate feedback.
- [ ] Active-use protection: Flow detects active defaults or running JDK processes and aborts safely with actionable guidance. Current implementation relies on stub safety checks in `src/uninstall/safety.rs`; completion is deferred to [T-s2g7h-active-use-detection](../T-s2g7h-active-use-detection/README.md).

## Follow-Up

- Deferred scope: Active-use detection migrated to [T-s2g7h-active-use-detection](../T-s2g7h-active-use-detection/README.md).

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../../templates/README.md#task-template-taskmd) in the templates README.
