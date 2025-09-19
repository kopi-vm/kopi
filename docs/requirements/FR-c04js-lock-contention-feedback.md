# User feedback for lock contention

## Metadata

- Type: Functional Requirement
- Status: Accepted
  <!-- Proposed: Under discussion | Accepted: Approved for implementation | Implemented: Code complete | Verified: Tests passing | Deprecated: No longer applicable -->

## Links

- Implemented by Tasks: N/A – Not yet implemented
- Related Requirements: FR-gbsz6
- Related ADRs: ADR-8mnaz
- Tests: N/A – Not yet tested
- Issue: N/A – No tracking issue created yet
- PR: N/A – Not yet implemented

## Requirement Statement

The system SHALL provide clear, actionable user feedback whenever an operation waits on a lock, including status, elapsed time, remaining time (if finite), and available user actions.

## Rationale

Without clear lock wait feedback, users may assume the application has frozen, remain unaware of timeout behavior, lose context for troubleshooting, and fail to take corrective actions such as canceling or adjusting timeouts.

## User Story (if applicable)

As a kopi user, I want to understand when an operation is waiting for a lock and what options I have, so that I can decide whether to wait, cancel, or adjust the timeout.

## Acceptance Criteria

- [ ] A message appears within 100 ms of entering a lock wait, naming the blocked resource (e.g., vendor-version-os-arch or cache).
- [ ] The configured timeout duration is displayed; infinite waits clearly indicate they have no timeout.
- [ ] A progress indicator updates at least once per second with elapsed time and remaining time when finite.
- [ ] Available actions (for example, `Ctrl-C to cancel`) are shown and updated consistently across TTY and non-TTY outputs.
- [ ] Timeout override guidance references the applicable CLI flag (`--lock-timeout`) and configuration options.
- [ ] On lock acquisition or timeout, the UI prints a concluding message describing the outcome.

## Technical Details (if applicable)

### Functional Requirement Details

- Initial wait message format: `Waiting for lock on {resource} (timeout: {duration})`.
- Progress updates use carriage-return refresh in interactive TTYs and periodic appended lines in non-TTY contexts (every 5 seconds).
- Infinite waits display `timeout: infinite` and omit remaining time.
- Success message: `Lock acquired; continuing {operation}`; timeout message: `Could not acquire lock after {duration}; retry with --lock-timeout`.
- Emit all lock wait messages to stderr to avoid interfering with command output redirection.

### Non-Functional Requirement Details

N/A – Not applicable.

## Verification Method

### Test Strategy

- Test Type: Integration
- Test Location: `tests/ui_feedback_tests.rs` (planned)
- Test Names: `test_fr_c04js_wait_message`, `test_fr_c04js_progress_display`

### Verification Commands

```bash
# Specific commands to verify this requirement
cargo test test_fr_c04js_wait_message
cargo test test_fr_c04js_progress_display
cargo run -- install temurin@21 & cargo run -- install temurin@21
```

### Success Metrics

- Metric 1: Lock wait message appears within 100 ms in 95% of observed waits.
- Metric 2: Progress updates render at least once per second (TTY) or every 5 seconds (non-TTY).
- Metric 3: User satisfaction rating for lock feedback improves in usability testing (baseline to be defined in UX research).

## Dependencies

- Depends on: FR-gbsz6 (timeout configuration and signaling)
- Blocks: N/A – Blocks nothing

## Platform Considerations

### Unix

- Detect interactive terminals via `isatty()` and use ANSI escape sequences for in-place updates.

### Windows

- Use Windows console APIs for terminal detection and carriage-return updates; ensure ANSI support is enabled where available.

### Cross-Platform

- Keep message content identical across platforms and fall back to appended logs when real-time updates are unsupported.

## Risks & Mitigation

| Risk                           | Impact | Likelihood | Mitigation                                         | Validation                         |
| ------------------------------ | ------ | ---------- | -------------------------------------------------- | ---------------------------------- |
| Progress updates cause flicker | Low    | Medium     | Throttle redraws and reuse the line                | Visual regression testing          |
| Messages lost in CI logs       | Medium | High       | Detect CI via env vars; emit periodic static lines | Run tests in CI                    |
| Terminal width too narrow      | Low    | Low        | Truncate lines gracefully                          | Manual tests with narrow terminals |

## Implementation Notes

- Evaluate using the `indicatif` crate or internal helper for consistent progress rendering; ensure it works in non-TTY contexts.
- Always log lock wait state transitions at debug level for traceability regardless of UI display.
- Consider localization readiness by isolating message templates, even though English-only output is currently required.
- Provide a quiet mode that suppresses progress updates while still signaling wait state changes.

## External References

N/A – No external references

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd).
