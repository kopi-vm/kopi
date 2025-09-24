# Uninstallation Lock Integration Task

## Metadata

- Type: Task
- Status: Proposed

## Links

- Related Analyses:
  - [AN-m9efc-concurrent-process-locking](../../analysis/AN-m9efc-concurrent-process-locking.md)
- Related Requirements:
  - [FR-ui8x2-uninstallation-locking](../../requirements/FR-ui8x2-uninstallation-locking.md)
  - [NFR-g12ex-cross-platform-compatibility](../../requirements/NFR-g12ex-cross-platform-compatibility.md)
  - [NFR-vcxp8-lock-cleanup-reliability](../../requirements/NFR-vcxp8-lock-cleanup-reliability.md)
  - [FR-gbsz6-lock-timeout-recovery](../../requirements/FR-gbsz6-lock-timeout-recovery.md)
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Associated Plan Document:
  - N/A – Not started
- Associated Design Document:
  - N/A – Not started

## Summary

Apply exclusive locking and timeout-aware coordination to the uninstallation workflow to guarantee atomic removal of JDKs while preventing conflicts with concurrent installations or other uninstallations.

## Scope

- In scope: Reuse installation lock keys, guard uninstallation phases with exclusive locks, coordinate with timeout and feedback observers, and validate rollback when failures occur.
- Out of scope: Installation pipeline, cache refresh handling, and new UI features beyond leveraging shared feedback interfaces.

## Success Metrics

- `Atomic removal`: Tests verify that uninstallation either completes fully or rolls back without leaving partial directories or metadata.
- `Concurrent safety`: Attempts to install/uninstall the same coordinate while a lock is held respect timeout and cancellation policies with accurate user feedback.
- `Active-use protection`: The workflow detects active default or running JDK processes and aborts safely with actionable guidance.

## Detailed Plan

- Instrument uninstallation entry points with lock acquisition/release around preflight checks, shim cleanup, directory deletion, and metadata updates.
- Ensure lock acquisition respects canonicalized coordinates and recycles the same lock files used during installation.
- Integrate timeout/cancellation handling, mapping failure modes to exit codes and observer callbacks for consistent messaging.
- Expand test coverage to include conflicting install/uninstall operations, forced termination mid-delete, and recovery after partial failures.
- Document operator guidance, including forced removal flags and expected diagnostics when uninstallation is blocked.

## Notes

- Coordinate with shell integration components to ensure active shims are detected before deletion begins.
- Evaluate two-phase delete strategies and capture design decisions for potential ADR follow-up if complexity increases.

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../../templates/README.md#task-template-taskmd) in the templates README.
