# Lock timeout and recovery mechanism

## Metadata
- Type: Functional Requirement
- Owner: Development Team
- Reviewers: Architecture Team
- Status: Approved
- Priority: P0
- Date Created: 2025-09-02
- Date Modified: 2025-09-02

## Links
- Analysis: [`docs/analysis/AN-m9efc-concurrent-process-locking.md`](../analysis/AN-m9efc-concurrent-process-locking.md)
- Related ADRs: [`ADR-8mnaz-concurrent-process-locking-strategy.md`](../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Related Requirements: FR-0001, FR-0002, FR-0003 (all lock operations)
- Issue: N/A – No tracking issue created yet
- Task: N/A – Implementation not started

## Requirement Statement

The system SHALL provide configurable timeout mechanisms for lock acquisition to prevent indefinite waiting and deadlock scenarios.

## Rationale

Without timeout mechanisms:
- Processes could wait indefinitely for locks that may never be released
- Users would have no recourse when operations hang
- System resources could be tied up indefinitely
- Debugging lock-related issues would be difficult

## Acceptance Criteria

1. **Configurable Timeout**
   - GIVEN a lock acquisition attempt
   - WHEN the timeout duration is configured
   - THEN the process SHALL wait up to the specified duration
   - AND fail with a clear error message if timeout is exceeded

2. **Timeout Configuration Priority**
   - GIVEN multiple configuration sources
   - WHEN determining timeout value
   - THEN priority SHALL be: CLI args > Environment vars > Config file > Default
   - AND the active timeout value SHALL be used

3. **Infinite Wait Option**
   - GIVEN a user requirement for indefinite waiting
   - WHEN timeout is set to "infinite"
   - THEN the process SHALL wait indefinitely for lock acquisition
   - AND only exit on user interruption (Ctrl-C) or lock acquisition

4. **No-wait Option**
   - GIVEN a requirement for immediate failure
   - WHEN timeout is set to 0 or --no-wait flag is used
   - THEN lock acquisition SHALL fail immediately if lock is not available
   - AND return appropriate error code

5. **Automatic Recovery**
   - GIVEN a process crash while holding a lock
   - WHEN using native std::fs::File locks
   - THEN the operating system SHALL automatically release the lock
   - AND other processes SHALL be able to acquire it immediately

## Implementation Notes

- Default timeout: 600 seconds (10 minutes)
- Configuration methods:
  - CLI: `--wait=<seconds|infinite>`, `--no-wait`
  - Environment: `KOPI_LOCKING__TIMEOUT=<seconds|infinite>`
  - Config file: `[locking]` section with `timeout` field
- Use try_lock() in a loop with sleep intervals for timeout implementation
- Native locks automatically cleaned up by OS on process exit

## Verification Steps

1. **Default Timeout Test**
   - Acquire lock in one process
   - Attempt acquisition in second process
   - Verify timeout after 600 seconds

2. **Custom Timeout Test**
   - Set custom timeout via CLI/env/config
   - Verify timeout honors configuration

3. **Infinite Wait Test**
   - Set timeout to "infinite"
   - Verify process waits indefinitely
   - Verify Ctrl-C interruption works

4. **No-wait Test**
   - Use --no-wait flag
   - Verify immediate failure when lock unavailable

5. **Priority Test**
   - Set different timeout values in CLI, env, and config
   - Verify CLI takes precedence

## Dependencies

- Native std::fs::File locking API (Rust 1.89.0+)
- Configuration system (CLI parser, env vars, config file)
- Time measurement utilities

## Out of Scope

- Distributed timeout coordination
- Lock lease renewal mechanisms
- Automatic retry with backoff
- Deadlock detection algorithms