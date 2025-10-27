# T-60h68 Lock Contention Feedback Design

## Metadata

- Type: Design
- Status: Approved
  <!-- Draft: Work in progress | Approved: Ready for implementation | Rejected: Not moving forward with this design -->

## Links

- Associated Plan Document:
  - [T-60h68-lock-feedback-plan](./plan.md)

## Overview

Deliver a reusable lock wait feedback layer that consumes the instrumentation hooks introduced by T-lqyk8 and surfaces clear, actionable progress indicators across interactive and non-interactive environments. The design satisfies FR-c04js by standardising message formats, keeping latency below 100 ms, and providing guidance for cancellation and timeout overrides while reusing the existing `src/indicator` subsystem (factory, simple/indicatif reporters, and quiet handling) instead of introducing ad-hoc writers.

## Success Metrics

- [ ] Initial wait notification prints within 100 ms of `on_wait_start`.
- [ ] Progress updates occur ≥1 Hz on TTYs and ≤5 s cadence on non-TTY outputs during sustained contention.
- [ ] Cancellation (`Ctrl-C`) and timeout outcomes emit distinct guidance referencing `--lock-timeout` overrides.
- [ ] Unit tests cover TTY and non-TTY renderers, ensuring message parity and elapsed time accuracy within ±1 s.
- [ ] Observer integration verifies no progress output is written when quiet mode is enabled.

## Background and Current State

- Kopi currently relies on ad-hoc `println!` statements when contention occurs, leading to inconsistent wording and duplicated logic across commands.
- T-ec5ew established `LockController` abstractions, while T-lqyk8 (approved) adds `LockWaitObserver` callbacks for wait lifecycle events.
- FR-c04js requires uniform messaging that includes resource names, elapsed and remaining time, available actions, and concluding status messages.
- Existing locking code lacks UX branching for TTY vs. non-TTY scenarios and does not emit guidance on overriding timeout values.
- All user-facing documentation must remain in English and align with ADR-8mnaz guidance for advisory locking transparency.

## Proposed Design

### High-Level Architecture

```
LockController ──▶ LockWaitObserver (from T-lqyk8)
                      │
                      ▼
            LockFeedbackBridge
                      │
              ProgressFactory
                      │
        ┌─────────────┴─────────────┐
        ▼                           ▼
IndicatifProgress           Simple/SilentProgress
 (TTY renderer)             (non-TTY, CI, quiet)
        │                           │
      stderr                    stderr/logging
```

### Components

- **LockFeedbackBridge**
  - Implements `LockWaitObserver` and converts raw lifecycle callbacks into renderer-friendly events (`WaitStarted`, `ProgressTick`, `Acquired`, `Timeout`, `Cancelled`).
  - Resolves resource descriptors (scope, target path) using `LockScope` metadata from T-ec5ew and formats them consistently (e.g., `cache`, `temurin-21-linux-x64`).
  - Tracks `Instant::now()` at wait start to compute elapsed and remaining durations for each progress tick.
  - Delegates rendering to instances produced by `ProgressFactory` (`src/indicator/factory.rs`), allowing existing `IndicatifProgress`, `SimpleProgress`, and `SilentProgress` implementations to handle terminal detection, quiet mode, and colour support consistently.

- **ProgressEvent**
  - Enum capturing the payload for each renderer. Fields include `resource_id`, `timeout`, `elapsed`, `remaining`, `actions` (e.g., `Ctrl-C to cancel`), and state (`Acquired`, `TimedOut`, `Cancelled`).
  - Encapsulates timeout semantics by reading the `LockTimeoutValue` supplied in the observer request (finite vs. infinite).

- **TTY Renderer (IndicatifProgress)**
  - Reuses the existing `IndicatifProgress` wrapper to emit carriage-return updates (`\r`) when `stderr` is an interactive terminal and virtual terminal processing is available on Windows.
  - The bridge configures the progress bar style via `ProgressConfig::with_style(ProgressStyle::Status)` so elapsed/remaining time and action hints display inline.
  - On completion, calls `complete`/`success` to print the concluding message before clearing the spinner line.

- **Log Renderer (SimpleProgress)**
  - Default for non-TTY streams, CI detection (`KOPI_CI`), or when ANSI is disabled.
  - Uses `SimpleProgress` to append timestamped lines every ≤5 s without control characters: `... still waiting for lock on {resource}; elapsed {elapsed}s (timeout: {timeout_label})`.
  - Includes override hints on initial and terminal messages by invoking `println` and `error` helpers.

- **Silent Renderer**
  - Utilises `SilentProgress` to respect quiet mode or JSON outputs, ensuring no user-facing lines are emitted while still permitting optional diagnostic logging.

- **FeedbackActions Registry**
  - Provides context-specific action strings. Default actions include `Ctrl-C to cancel` and `Retry with --lock-timeout=<seconds>`.
  - Applies uniform guidance across lock scopes to keep the messaging concise.

- **QuietModeGuard**
  - Reads per-command flags (e.g., `--quiet`, `--json`) and requests `ProgressFactory::silent()` to obtain `SilentProgress`, while still allowing internal debug logs if required.

- **StatusReporter Alignment**
  - Refactors `StatusReporter` (under `src/indicator/status.rs`) to delegate operation/step output through the same `ProgressIndicator::println` pathway, ensuring lock wait lines and general status updates share terminal detection, colouring, and stderr routing.
  - Adds a lightweight `lock_feedback_start` helper that forwards the initial wait message to the active progress indicator when commands rely on `StatusReporter`, preventing duplicate prefixes and keeping messaging consistent with FR-c04js wording.
  - Retains silent mode semantics by short-circuiting through `SilentProgress`, so quiet commands opt out automatically without extra flags.

### Data Flow

1. `LockController::acquire_with` (from T-lqyk8) receives a `LockWaitObserver` reference; commands register `LockFeedbackBridge` when interactive output is desired.
2. On `on_wait_start`, the bridge determines renderer choice based on `isatty(stderr)` and Windows virtual terminal support, then emits the initial message via the selected renderer.
3. Each `on_retry`/`on_progress_tick` callback updates elapsed time; the bridge computes remaining time (`timeout - elapsed`) when the timeout is finite, or labels it `infinite`.
4. Renderers throttle output (TTY ≥1 Hz, Log ≤5 s) while always relaying the first and final messages.
5. When `on_acquired`, `on_timeout`, or `on_cancelled` fires, the bridge prints a concluding message and flushes `stderr`.
6. Diagnostic logging records which renderer was selected so operators can troubleshoot environment-specific behaviour without additional telemetry pipelines.

### Message Formatting

- **Initial Line:** `Waiting for lock on {resource} (timeout: {timeout_label}) — {actions}`
- **Progress Update (TTY):** `Waiting for lock on {resource} — elapsed {elapsed}s of {timeout_label} [Ctrl-C to cancel]`
- **Progress Update (Log):** `[elapsed={elapsed}s] still waiting for lock on {resource}; timeout {timeout_label}`
- **Acquired:** `Lock acquired after {elapsed}s; continuing {operation}.`
- **Timeout:** `Could not acquire lock on {resource} after {timeout_label}; retry with --lock-timeout or adjust configuration.`
- **Cancelled:** `Lock wait on {resource} cancelled after {elapsed}s; rerun when the resource is free.`
- All messages target `stderr` to avoid disturbing `stdout` pipelines, fulfilling FR-c04js guidance.

### Configuration and Extensibility

- Renderer selection relies on `ProgressFactory::detect` to incorporate terminal checks, colour decisions, and environment overrides:
  - `KOPI_NO_TTY_PROGRESS=1` forces `SimpleProgress` even on interactive terminals.
  - `KOPI_FORCE_TTY_PROGRESS=1` enables `IndicatifProgress` when detection is inconclusive (e.g., Windows ConPTY).
- Future integrations (e.g., GUI front-ends) can register custom `ProgressIndicator` implementations with the factory without changing `LockFeedbackBridge`.

### Error Handling

- Bridge gracefully handles missing timeout data by treating it as infinite and logging a debug warning.
- If renderer writes fail (e.g., broken pipe), the bridge stops emitting further output and records a debug log, preventing panics during CLI piping.
- Cancellation events originating from the `CancellationToken` differentiate user-triggered aborts from timeouts and map to `KopiError::LockingCancelled`.

### Performance Considerations

- Elapsed time calculations reuse a shared `Instant` to avoid repeated system calls.
- Renderer throttling ensures CPU utilisation remains negligible during long waits.
- TTY renderer performs O(1) formatting per tick using preallocated buffers to minimise allocations during contention.

### Platform Considerations

- **Unix:** Uses `nix` crate’s `isatty` helper via existing CLI utilities. Carries on standard ANSI escape sequences.
- **Windows:** Enables virtual terminal mode via the existing console setup in Kopi; falls back to log renderer when the console does not support VT sequences.
- **CI/Headless:** Detects `CI`, `KOPI_CI`, or absence of TTY to default to appended lines without control characters.

## Testing Strategy

- **Unit Tests**
  - Simulate observer events and assert rendered strings for TTY and non-TTY variants, using a mock clock to control elapsed time.
  - Verify suppression logic under quiet mode and when `KOPI_NO_TTY_PROGRESS` is set.

- **Integration Tests**
  - Spawn a child process running a long-lived lock holder and confirm that waiting commands produce the expected output sequences.
  - Capture Windows-specific behaviour via targeted tests leveraging the `windows` crate console flag API.

- **Manual Verification**
  - Run on macOS, Linux, and Windows terminals to confirm carriage-return updates, cancellation messaging, and timeout guidance.
  - Validate CI logs (GitHub Actions) to ensure updates appear at the correct cadence without overwhelming logs.

## Risks and Mitigations

| Risk                                                      | Mitigation                                                                 |
| --------------------------------------------------------- | -------------------------------------------------------------------------- |
| Carriage-return updates cause flicker on slower terminals | Cap update frequency at 4 Hz and allow environment opt-out.                |
| Infinite timeout confuses users about progress            | Explicitly render `timeout: infinite` and emphasise cancellation controls. |
| Renderer emits messages during quiet/json modes           | Central `QuietModeGuard` checks command context before forwarding events.  |

## Alternatives Considered

1. **Command-Specific Messaging**
   - Pros: Tailored text per command.
   - Cons: Duplicates logic, increases maintenance cost, risks divergence from FR-c04js; rejected.

2. **Direct Third-Party Progress Layer**
   - Pros: Off-the-shelf spinners/bars with minimal internal code.
   - Cons: Would bypass the established `src/indicator` abstractions, forcing divergent styling and limiting control over non-TTY cadence; rejected in favour of extending the existing indicator stack.

3. **Silencing Non-TTY Output Entirely**
   - Pros: Eliminates log noise.
   - Cons: Violates FR-c04js requirement for actionable feedback in pipelines; rejected.

## Open Questions

- None identified at this stage.
