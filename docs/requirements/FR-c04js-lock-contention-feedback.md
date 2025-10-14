# FR-c04js Lock Contention User Feedback

## Metadata

- Type: Functional Requirement
- Status: Approved
  <!-- Draft: Under discussion | Approved: Ready for implementation | Rejected: Decision made not to pursue this requirement -->

## Links

- Prerequisite Requirements:
  - [FR-gbsz6-lock-timeout-recovery](../requirements/FR-gbsz6-lock-timeout-recovery.md)
- Dependent Requirements:
  - N/A – Blocks nothing
- Related Tasks:
  - [T-60h68-lock-feedback](../tasks/T-60h68-lock-feedback/README.md)

## Requirement Statement

The system SHALL present clear, actionable feedback whenever an operation waits on a lock, including the targeted resource, elapsed and remaining time (if finite), and available user actions.

## Rationale

Without timely messaging, users assume the CLI has frozen, lose context for ongoing operations, and cannot make informed decisions about canceling, retrying, or adjusting lock timeouts.

## User Story (if applicable)

As a Kopi user, I want to know when a command is blocked on a lock and what options I have, so that I can decide whether to wait, cancel, or change the timeout.

## Acceptance Criteria

- [ ] A wait message appears within 100 ms of entering a lock wait, naming the blocked resource (vendor-version-os-arch, cache, etc.).
- [ ] The configured timeout duration is displayed; infinite waits explicitly state they have no timeout.
- [ ] A progress indicator updates at least once per second with elapsed time and remaining time where applicable.
- [ ] Available actions (for example, `Ctrl-C to cancel`) are shown consistently across TTY and non-TTY output modes.
- [ ] Timeout override guidance references the relevant CLI flag (`--lock-timeout`) and configuration settings.
- [ ] On acquisition or timeout, the CLI prints a concluding message describing the outcome.

## Technical Details (if applicable)

### Functional Requirement Details

- Initial message format: `Waiting for lock on {resource} (timeout: {duration})`.
- Progress updates use carriage-return refresh for interactive TTYs and periodic appended lines (every 5 seconds) for non-TTY output.
- Infinite waits display `timeout: infinite` and omit remaining time display.
- Success message: `Lock acquired; continuing {operation}`.
- Timeout message: `Could not acquire lock after {duration}; retry with --lock-timeout`.
- Emit feedback to stderr to avoid interfering with stdout pipelines.

### Non-Functional Requirement Details

N/A – No additional non-functional constraints.

## Platform Considerations

### Unix

- Detect interactive terminals via `isatty()` and rely on ANSI escape sequences for in-place updates.

### Windows

- Use Windows console APIs or virtual terminal processing for carriage-return updates; ensure ANSI mode is enabled when available.

### Cross-Platform

- Keep message content identical across platforms; fall back to appended logs when real-time updates are unsupported.

## Risks & Mitigation

| Risk                           | Impact | Likelihood | Mitigation                                            | Validation                         |
| ------------------------------ | ------ | ---------- | ----------------------------------------------------- | ---------------------------------- |
| Progress updates cause flicker | Low    | Medium     | Throttle redraws and reuse the same line              | Visual regression testing          |
| Messages lost in CI logs       | Medium | High       | Detect CI environments and emit periodic static lines | Run tests in CI pipelines          |
| Terminal width too narrow      | Low    | Low        | Truncate lines gracefully and avoid wrapping noise    | Manual tests with narrow terminals |

## Implementation Notes

- Evaluate reusing `indicatif` or internal helpers for consistent progress rendering; confirm compatibility with non-TTY contexts.
- Always log lock wait state transitions at debug level for traceability even when UI output is suppressed.
- Provide a quiet mode that suppresses progress updates while still signaling wait state changes.

## External References

N/A – No external references.

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd) in the templates README.
