# T-60h68 Lock Contention Feedback Task

## Metadata

- Type: Task
- Status: Draft
  <!-- Draft: Under discussion | In Progress: Actively working | Complete: Code complete | Cancelled: Work intentionally halted -->

## Links

- Associated Plan Document:
  - N/A – Plan not started
- Associated Design Document:
  - N/A – Design not started

## Summary

Implement consistent user feedback for lock waits, including real-time progress updates, timeout guidance, and actionable controls across interactive and non-interactive outputs.

## Scope

- In scope: Consume observer hooks from T-lqyk8, render elapsed/remaining time, handle TTY vs non-TTY presentation, and document overrides.
- Out of scope: Core timeout mechanics, lock acquisition logic, localization beyond English messaging.

## Success Metrics

- Feedback latency: initial wait message renders within 100 ms of entering a lock wait in automated terminal tests.
- Update cadence: progress indicators refresh at least once per second on TTYs and every 5 seconds in log mode while preserving clean CI output.
- Actionable guidance: manual tests confirm instructions for cancellation and timeout overrides are clear across supported scenarios.

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../../templates/README.md#task-template-taskmd) in the templates README.
