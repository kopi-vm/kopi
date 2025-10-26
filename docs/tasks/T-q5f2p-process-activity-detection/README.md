# T-q5f2p Process Activity Detection Task

## Metadata

- Type: Task
- Status: Draft
  <!-- Draft: Under discussion | In Progress: Actively working | Complete: Code complete | Cancelled: Work intentionally halted -->

## Links

- Related Analyses:
  - [AN-m9efc-concurrent-process-locking](../../analysis/AN-m9efc-concurrent-process-locking.md)
- Related Requirements:
  - [FR-rxelv-file-in-use-detection](../../requirements/FR-rxelv-file-in-use-detection.md)
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Associated Design Document:
  - N/A – Design not started
- Associated Plan Document:
  - N/A – Plan not started

## Summary

Implement cross-platform detection of running processes that are using a target JDK installation so uninstall logic can warn or block when binaries are still in use. This unlocks the remaining scope postponed from T-s2g7h by extending `src/platform/process.rs` with OS-specific backends.

## Scope

- In scope:
  - Add platform-specific implementations within `src/platform/process.rs` (Unix, macOS, Windows) that enumerate processes holding open handles inside a given JDK directory.
  - Expose a safe, cross-platform API (e.g., `processes_using_path`) returning structured metadata `ProcessInfo { pid, exe_path, handle_path }` for downstream consumers.
  - Provide unit/integration tests using captured OS-specific fixtures or mocks to validate detection accuracy and error handling.
- Out of scope:
  - Changes to locking, timeout, or force-override UX (handled by their respective tasks).
  - Telemetry or analytics for active process reporting.
  - Broader CLI messaging changes beyond wiring up the new API once available.

## Success Metrics

- Enumerating processes on supported platforms reliably surfaces PID and executable information when any file under the target JDK tree is open.
- The new API integrates with uninstall safety checks without introducing `unsafe` into higher layers.
- Automated tests pass across targeted platforms, using real fixture data collected via `lsof`, `procfs`, or Windows handle inspection.

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../../templates/README.md#task-template-taskmd) in the templates README.
