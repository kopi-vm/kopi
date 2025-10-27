# T-q5f2p Process Activity Detection Task

## Metadata

- Type: Task
- Status: Complete
  <!-- Draft: Under discussion | In Progress: Actively working | Complete: Code complete | Cancelled: Work intentionally halted -->

## Links

- Related Analyses:
  - [AN-m9efc-concurrent-process-locking](../../analysis/AN-m9efc-concurrent-process-locking.md)
- Related Requirements:
  - [FR-rxelv-file-in-use-detection](../../requirements/FR-rxelv-file-in-use-detection.md)
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Associated Design Document:
  - [design.md](design.md)
- Associated Plan Document:
  - [plan.md](plan.md)

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

## Manual Verification

- **macOS**
  - Launch a terminal and start `java` from the target JDK, for example `~/.kopi/jdks/temurin-21.0.5+11/Contents/Home/bin/java -version` without closing the shell.
  - In a second terminal, run `kopi uninstall temurin@21.0.5+11` and confirm the validation error lists the PID, executable, and affected handle paths.
  - Retry with `kopi uninstall temurin@21.0.5+11 --force` and ensure the command succeeds while logging the same diagnostics as warnings.
  - Test from a non-admin user to observe the permission warning when scanning processes owned by other accounts.
- **Windows**
  - Open PowerShell and execute `Start-Process -PassThru "$env:USERPROFILE\.kopi\jdks\temurin-21.0.5+11\bin\java.exe" -ArgumentList '-version'` to hold an open handle.
  - Run `kopi uninstall temurin@21.0.5+11` in a separate shell and verify the CLI reports the PID, executable path, and the handle list before blocking.
  - Re-run with `--force` to confirm uninstall proceeds while emitting the warning banner captured in the logs and status reporter.
  - Attempt to enumerate a system-owned process (e.g., `services.exe`) to validate the permission-denied path is downgraded to a warning instead of aborting.

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../../templates/README.md#task-template-taskmd) in the templates README.
