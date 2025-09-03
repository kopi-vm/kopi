# User feedback for lock contention

## Metadata
- Type: Functional Requirement
- Owner: Development Team
- Reviewers: Architecture Team, UX Team
- Status: Approved
- Priority: P1
- Date Created: 2025-09-02
- Date Modified: 2025-09-02

## Links
- Analysis: [`docs/analysis/AN-m9efc-concurrent-process-locking.md`](../analysis/AN-m9efc-concurrent-process-locking.md)
- Related ADRs: [`ADR-8mnaz-concurrent-process-locking-strategy.md`](../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Related Requirements: FR-0004 (timeout mechanism)
- Issue: N/A – No tracking issue created yet
- Task: N/A – Implementation not started

## Requirement Statement

The system SHALL provide clear, actionable feedback to users when operations are waiting for locks, including wait progress and available actions.

## Rationale

Without clear feedback:
- Users may think the application has frozen
- Users won't understand why operations are delayed
- Users can't make informed decisions about waiting or canceling
- Debugging concurrent operation issues becomes difficult

## Acceptance Criteria

1. **Waiting Notification**
   - GIVEN a process waiting for a lock
   - WHEN the wait begins
   - THEN a message SHALL indicate what is being waited for
   - AND the configured timeout duration SHALL be displayed

2. **Progress Indication**
   - GIVEN an ongoing lock wait
   - WHEN waiting for more than 1 second
   - THEN a progress indicator SHALL show elapsed time
   - AND remaining time (if finite timeout) SHALL be displayed

3. **Actionable Instructions**
   - GIVEN a lock wait in progress
   - WHEN displaying wait message
   - THEN available actions SHALL be shown (e.g., "Ctrl-C to cancel")
   - AND timeout override options SHALL be mentioned

4. **Timeout Notification**
   - GIVEN a lock acquisition timeout
   - WHEN the timeout is exceeded
   - THEN an error message SHALL explain the timeout
   - AND suggest remediation options (e.g., retry with longer timeout)

5. **Success Confirmation**
   - GIVEN a lock successfully acquired after waiting
   - WHEN the operation proceeds
   - THEN a brief confirmation SHALL indicate lock acquired
   - AND the operation SHALL continue with normal output

## Implementation Notes

- Message examples:
  - Waiting: "Another process is installing temurin@21. Waiting up to 600s (Ctrl-C to cancel)"
  - Progress: "Waiting for lock... [45s/600s] ⠋"
  - Timeout: "Timed out after 600s. Try --wait=1200 or KOPI_LOCKING__TIMEOUT=infinite"
  - Success: "Lock acquired, proceeding with installation..."
- Use spinner or progress bar for visual feedback
- Log detailed lock information at DEBUG level
- Consider terminal capabilities for progress display

## Verification Steps

1. **Wait Message Test**
   - Create lock contention scenario
   - Verify clear waiting message appears immediately

2. **Progress Display Test**
   - Create extended wait scenario
   - Verify progress indicator updates regularly

3. **Timeout Message Test**
   - Force timeout scenario
   - Verify helpful error message with suggestions

4. **Non-TTY Test**
   - Run in non-interactive environment (CI/pipe)
   - Verify appropriate text-only output

5. **Cancellation Test**
   - Use Ctrl-C during wait
   - Verify clean cancellation and appropriate message

## Dependencies

- Terminal detection for appropriate output format
- Progress indicator library or implementation
- Signal handling for Ctrl-C

## Out of Scope

- GUI progress dialogs
- Sound or system notifications
- Lock queue position information
- Historical lock wait statistics