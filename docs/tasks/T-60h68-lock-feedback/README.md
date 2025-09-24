# Lock Contention Feedback Task

## Metadata

- Type: Task
- Status: Proposed

## Links

- Related Analyses:
  - [AN-m9efc-concurrent-process-locking](../../analysis/AN-m9efc-concurrent-process-locking.md)
- Related Requirements:
  - [FR-c04js-lock-contention-feedback](../../requirements/FR-c04js-lock-contention-feedback.md)
  - [FR-gbsz6-lock-timeout-recovery](../../requirements/FR-gbsz6-lock-timeout-recovery.md)
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Associated Plan Document:
  - N/A – Not started
- Associated Design Document:
  - N/A – Not started

## Summary

Implement consistent user feedback for lock waits, including real-time progress updates, timeout guidance, and actionable controls across interactive and non-interactive outputs.

## Scope

- In scope: Build UI layer consuming observer hooks from T-lqyk8, render elapsed/remaining time, handle TTY vs non-TTY presentation, and document overrides.
- Out of scope: Core timeout mechanics, lock acquisition logic, and localization beyond English messaging.

## Success Metrics

- `Feedback latency`: Initial wait message appears within 100 ms of entering a lock wait in automated terminal tests.
- `Update cadence`: Progress indicators refresh at least once per second on TTYs and every 5 seconds in log mode while maintaining clean output in CI pipelines.
- `Actionable guidance`: User surveys or manual tests confirm instructions for cancellation and timeout overrides are clear across supported scenarios.

## Detailed Plan

- Define a feedback adapter that subscribes to timeout observer events and renders platform-appropriate messages (TTY carriage return vs appended log lines).
- Implement formatting utilities that expose resource names, elapsed/remaining time, and guidance on `--lock-timeout` overrides.
- Add tests covering interactive terminals, redirected output, CI detection, and quiet mode to ensure consistent messaging.
- Integrate feedback into installation, uninstallation, and cache workflows once their locking tasks reach readiness checkpoints.
- Update developer documentation with guidelines for invoking feedback hooks and customizing verbosity levels.

## Notes

- Coordinate with CLI argument parsing to ensure feedback respects quiet or JSON output modes without breaking automation.
- Capture debug logs for wait state transitions to aid troubleshooting without duplicating user-facing text.

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../../templates/README.md#task-template-taskmd) in the templates README.
