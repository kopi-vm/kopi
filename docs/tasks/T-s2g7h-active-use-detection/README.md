# T-s2g7h Active-Use Detection Task

## Metadata

- Type: Task
- Status: Draft
  <!-- Draft: Under discussion | In Progress: Actively working | Complete: Code complete | Cancelled: Work intentionally halted -->

## Links

- Related Analyses:
  - [AN-m9efc-concurrent-process-locking](../../analysis/AN-m9efc-concurrent-process-locking.md)
- Related Requirements:
  - [FR-ui8x2-uninstallation-locking](../../requirements/FR-ui8x2-uninstallation-locking.md)
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Associated Design Document:
  - N/A – Design not started
- Associated Plan Document:
  - N/A – Plan not started

## Summary

Deliver active-use detection for uninstall flows by identifying globally or locally pinned JDKs and refusing removal while surfacing clear guidance, fulfilling the remaining scope deferred from T-98zsb.

## Scope

- In scope:
  - Detect when a JDK slated for uninstall is the current global or project default.
  - Surface actionable feedback and override mechanisms consistent with timeout feedback observers.
  - Integrate detection with uninstall safety checks, batch removal, and recovery tooling.
- Out of scope:
  - New auto-detection features outside of uninstall workflows.
  - Redesign of existing locking or timeout subsystems.

## Success Metrics

- Active default detection: Uninstall aborts with descriptive messaging whenever the target JDK is the active global default, unless `--force` is supplied.
- Local session guardrails: Project-scoped active JDKs trigger a safe abort with instructions to switch or force before uninstall proceeds.
- Integration coverage: Automated tests span single, batch, and recovery uninstall paths demonstrating correct detection, messaging, and interaction with contention handling.

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../../templates/README.md#task-template-taskmd) in the templates README.
