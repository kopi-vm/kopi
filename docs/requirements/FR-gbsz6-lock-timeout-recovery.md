# Lock timeout and recovery mechanism

## Metadata
- ID: FR-gbsz6
- Type: Functional Requirement
- Category: Platform
- Priority: P0 (Critical)
- Owner: Development Team
- Reviewers: Architecture Team
- Status: Accepted
- Date Created: 2025-09-02
- Date Modified: 2025-09-03

## Links
- Implemented by Tasks: N/A – Not yet implemented
- Related Requirements: FR-02uqo, FR-ui8x2, FR-v7ql4
- Related ADRs: [ADR-8mnaz](../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Tests: N/A – Not yet tested
- Issue: N/A – No tracking issue created yet
- PR: N/A – Not yet implemented

## Requirement Statement

The system SHALL provide configurable timeout mechanisms for lock acquisition to prevent indefinite waiting and deadlock scenarios.

## Rationale

Without timeout mechanisms:
- Processes could wait indefinitely for locks that may never be released
- Users would have no recourse when operations hang
- System resources could be tied up indefinitely
- Debugging lock-related issues would be difficult

## User Story (if applicable)

As a kopi user, I want operations to fail gracefully with clear messages when they can't acquire locks, so that I'm not left waiting indefinitely for hung operations.

## Acceptance Criteria

- [ ] Process waits up to the configured timeout duration for lock acquisition
- [ ] Clear error message displayed when timeout is exceeded
- [ ] Timeout configuration follows priority: CLI args > Environment vars > Config file > Default
- [ ] "Infinite" timeout option allows indefinite waiting
- [ ] Process can be interrupted with Ctrl-C during wait
- [ ] Default timeout provides reasonable user experience

## Technical Details (if applicable)

### Functional Requirement Details
- Default timeout: 30 seconds for installations, 10 seconds for cache operations
- Environment variable: `KOPI_LOCK_TIMEOUT` (seconds)
- CLI flag: `--lock-timeout <seconds>` or `--lock-timeout infinite`
- Config file setting: `lock_timeout = <seconds>`
- Special value "infinite" or "0" means wait indefinitely

### Configuration Priority
1. CLI arguments (highest priority)
2. Environment variables
3. Configuration file
4. Built-in defaults (lowest priority)

## Verification Method

### Test Strategy
- Test Type: Integration
- Test Location: `tests/timeout_tests.rs` (planned)
- Test Names: `test_fr_gbsz6_timeout_exceeded`, `test_fr_gbsz6_timeout_priority`

### Verification Commands
```bash
# Specific commands to verify this requirement
cargo test test_fr_gbsz6
KOPI_LOCK_TIMEOUT=1 cargo test test_fr_gbsz6_env
```

### Success Metrics
- Metric 1: Lock acquisition fails within timeout + 100ms tolerance
- Metric 2: Configuration priority correctly applied in 100% of cases

## Dependencies

- Depends on: N/A – No dependencies
- Blocks: FR-02uqo, FR-ui8x2, FR-v7ql4 (all need timeout support)

## Platform Considerations

### Unix
- Uses `try_lock_exclusive()` with polling and sleep
- Signal handling for Ctrl-C interruption

### Windows
- Uses Windows lock API with timeout support
- Console control handler for Ctrl-C

### Cross-Platform
- Consistent timeout behavior across platforms
- Uniform error messages

## Risks & Mitigation

| Risk | Impact | Likelihood | Mitigation | Validation |
|------|--------|------------|------------|------------|
| Timeout too short for slow systems | Medium | Medium | Conservative defaults, user configurable | Test on slow hardware |
| Polling overhead | Low | Medium | Exponential backoff in retry loop | Measure CPU usage |
| Clock skew affects timeout | Low | Low | Use monotonic clock | Test with clock changes |

## Implementation Notes

- Use `std::time::Instant` for monotonic time measurement
- Implement exponential backoff: 10ms, 20ms, 40ms... up to 1 second
- Log timeout attempts for debugging
- Consider showing progress indicator during wait
- Handle EINTR on Unix systems

## External References
N/A – No external references

## Change History

- 2025-09-02: Initial version
- 2025-09-03: Updated to use 5-character ID format