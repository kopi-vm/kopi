# T-98zsb Uninstallation Lock Integration Task

## Metadata

- Type: Task
- Status: Draft
  <!-- Draft: Under discussion | In Progress: Actively working | Complete: Code complete | Cancelled: Work intentionally halted -->

## Links

- Associated Design Document:
  - N/A – Design not started
- Associated Plan Document:
  - N/A – Plan not started

## Summary

Apply exclusive locking and timeout-aware coordination to the uninstallation workflow to guarantee atomic removal of JDKs while preventing conflicts with concurrent installations or other uninstallations.

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

- Atomic removal: Tests verify uninstallation either completes fully or rolls back without leaving partial directories or metadata.
- Concurrent safety: Mixed install/uninstall operations respect timeout and cancellation policies with accurate feedback.
- Active-use protection: Flow detects active defaults or running JDK processes and aborts safely with actionable guidance.

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../../templates/README.md#task-template-taskmd) in the templates README.
