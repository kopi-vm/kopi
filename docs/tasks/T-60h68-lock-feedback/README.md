# T-60h68 Lock Contention Feedback Task

## Metadata

- Type: Task
- Status: Complete
  <!-- Draft: Under discussion | In Progress: Actively working | Complete: Code complete | Cancelled: Work intentionally halted -->

## Links

- Related Analyses:
  - [AN-m9efc-concurrent-process-locking](../../analysis/AN-m9efc-concurrent-process-locking.md)
- Related Requirements:
  - [FR-c04js-lock-contention-feedback](../../requirements/FR-c04js-lock-contention-feedback.md)
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Associated Design Document:
  - [T-60h68-lock-feedback-design](./design.md)
- Associated Plan Document:
  - [T-60h68-lock-feedback-plan](./plan.md)

## Summary

Implement consistent user feedback for lock waits, including real-time progress updates, timeout guidance, and actionable controls across interactive and non-interactive outputs.

## Scope

- In scope:
  - Consume observer hooks from T-lqyk8 for wait-state updates.
  - Render elapsed and remaining time with TTY and non-TTY variants.
  - Document override options for advanced users.
- Out of scope:
  - Core timeout mechanics and lock acquisition logic.
  - Localization beyond English messaging.

## Success Metrics

- Feedback latency: Initial wait message renders within 100 ms of entering a lock wait in automated terminal tests.
- Update cadence: Progress indicators refresh ≥1 Hz on TTYs and every ≤5 seconds in log mode without polluting CI logs.
- Actionable guidance: Manual tests confirm cancellation and override instructions are clear across supported scenarios.

## Manual Validation (2025-10-27)

- Non-TTY sample output (captured via `TestProgressCapture` harness): `Waiting for lock on installation (timeout: 30s, source CLI flag) — Ctrl-C to cancel; override with --lock-timeout.`
- Quiet mode validation: executing lock acquisition with `SilentProgress` produced no user-facing lines while logging continued at DEBUG level.

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../../templates/README.md#task-template-taskmd) in the templates README.
