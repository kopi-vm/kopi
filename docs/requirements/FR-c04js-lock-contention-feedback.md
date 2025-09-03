# User feedback for lock contention

## Metadata

- ID: FR-c04js
- Type: Functional Requirement
- Category: Usability
- Priority: P1 (High)
- Owner: Development Team
- Reviewers: Architecture Team, UX Team
- Status: Accepted
- Date Created: 2025-09-02
- Date Modified: 2025-09-03

## Links

- Implemented by Tasks: N/A – Not yet implemented
- Related Requirements: FR-gbsz6
- Related ADRs: [ADR-8mnaz](../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Tests: N/A – Not yet tested
- Issue: N/A – No tracking issue created yet
- PR: N/A – Not yet implemented

## Requirement Statement

The system SHALL provide clear, actionable feedback to users when operations are waiting for locks, including wait progress and available actions.

## Rationale

Without clear feedback:

- Users may think the application has frozen
- Users won't understand why operations are delayed
- Users can't make informed decisions about waiting or canceling
- Debugging concurrent operation issues becomes difficult

## User Story (if applicable)

As a kopi user, I want to understand why operations are waiting and what I can do about it, so that I can make informed decisions about whether to wait or take action.

## Acceptance Criteria

- [ ] Message displayed when lock wait begins, indicating what is being waited for
- [ ] Configured timeout duration is shown to the user
- [ ] Progress indicator appears after 1 second of waiting
- [ ] Elapsed time and remaining time (if finite) are displayed
- [ ] Available actions clearly shown (e.g., "Ctrl-C to cancel")
- [ ] Timeout override options are mentioned in the message

## Technical Details (if applicable)

### Functional Requirement Details

- Initial message: "Waiting for lock on {resource}... (timeout: {duration}s)"
- Progress format: "Waiting... [{elapsed}s / {timeout}s] Press Ctrl-C to cancel"
- Infinite wait: "Waiting... [{elapsed}s] Press Ctrl-C to cancel"
- Success message: "Lock acquired, proceeding with {operation}"
- Failure message: "Could not acquire lock after {timeout}s. Try again with --lock-timeout"

### UI Components

- Use stderr for all lock-related messages
- Support both TTY and non-TTY environments
- In TTY: Use carriage return for updating progress
- In non-TTY: Print periodic updates (every 5 seconds)

## Verification Method

### Test Strategy

- Test Type: Integration
- Test Location: `tests/ui_feedback_tests.rs` (planned)
- Test Names: `test_fr_c04js_wait_message`, `test_fr_c04js_progress_display`

### Verification Commands

```bash
# Specific commands to verify this requirement
cargo test test_fr_c04js
# Manual test with visual inspection
cargo run -- install temurin@21 & cargo run -- install temurin@21
```

### Success Metrics

- Metric 1: User message appears within 100ms of lock wait beginning
- Metric 2: Progress updates at least once per second during wait

## Dependencies

- Depends on: FR-gbsz6 (timeout mechanism)
- Blocks: N/A – Blocks nothing

## Platform Considerations

### Unix

- Terminal detection via isatty()
- ANSI escape codes for progress updates

### Windows

- Console API for terminal detection
- Windows console codes for progress updates

### Cross-Platform

- Consistent message format across platforms
- Graceful degradation in non-TTY environments

## Risks & Mitigation

| Risk                           | Impact | Likelihood | Mitigation                           | Validation                 |
| ------------------------------ | ------ | ---------- | ------------------------------------ | -------------------------- |
| Progress updates cause flicker | Low    | Medium     | Buffer updates, minimize redraws     | Visual testing             |
| Messages lost in CI/CD logs    | Medium | High       | Detect CI environment, adjust output | Test in CI                 |
| Terminal width too narrow      | Low    | Low        | Truncate messages appropriately      | Test with narrow terminals |

## Implementation Notes

- Use `indicatif` crate for progress bars if appropriate
- Detect CI environment via CI environment variable
- Consider using different message verbosity levels
- Log all lock events to debug log regardless of UI display

## External References

N/A – No external references

## Change History

- 2025-09-02: Initial version
- 2025-09-03: Updated to use 5-character ID format
