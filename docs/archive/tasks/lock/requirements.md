# Process Lock Requirements

## Metadata

- Owner: Development Team
- Reviewers: [TBD]
- Status: Draft
- Last Updated: 2025-08-31
- Links: [Design](./design.md), [Implementation Plan](./plan.md)

## Problem Statement

Kopi performs various write operations on the filesystem (JDK installation, cache updates, configuration changes) that can lead to data corruption or incomplete states when multiple kopi processes run concurrently. Users may inadvertently run multiple kopi commands in parallel (e.g., from different terminals, automated scripts, or CI/CD pipelines), potentially causing:

- Corrupted JDK installations
- Inconsistent cache state
- Lost configuration updates
- Partial file writes
- Resource contention and errors

This impacts all kopi users who run multiple instances, particularly in automated environments where parallel execution is common.

## Objectives & Success Metrics

- [ ] Objective 1: Ensure data integrity by preventing concurrent write operations (0% corruption rate)
- [ ] Objective 2: Provide clear feedback when operations are blocked (<100ms detection time)
- [ ] Objective 3: Automatically clean up stale locks (within 30 seconds of process termination)
- [ ] No performance regression for single-process operations (<5ms overhead)

## Scope

### Goals

- Implement a single global filesystem-based process lock per KOPI_HOME for write operations
- Lock required for: install, uninstall, cache refresh, cache update, use, local, global, pin, shim management
- Lock not required for: list, current, search, cache list-distributions, cache search, env, version, help
- Support both Unix and Windows platforms
- Provide clear user feedback when operations are blocked
- Automatically handle stale lock cleanup via OS mechanisms
- Allow read-only operations to proceed without locking

### Non-Goals

- Distributed locking across network filesystems
- Fine-grained locking per JDK version or operation type
- Lock priority or queueing mechanisms
- Support for cluster/multi-machine coordination

### Assumptions

- Users have write access to the kopi home directory
- Filesystem supports atomic file operations
- Process crashes are rare but must be handled
- Most operations complete within seconds to minutes

### Constraints

- Must work on all supported platforms (Unix/Windows)
- Cannot use external dependencies for IPC
- Must be resilient to process crashes and abnormal termination
- Should have minimal performance impact

## User Stories / Use Cases

- As a developer, I want to run `kopi install` from multiple terminals without corruption, so that my JDK installations remain intact. (FR-001)
- As a CI/CD system, I want parallel builds to wait safely when installing JDKs, so that builds don't fail due to conflicts. (FR-002)
- As a user, I want clear feedback when an operation is waiting, so that I understand why the command isn't proceeding. (FR-003)
- As an administrator, I want stale locks to be cleaned automatically, so that crashed processes don't block the system indefinitely. (FR-004)

## Functional Requirements (FR)

- FR-001 [Must]: Prevent concurrent write operations by acquiring an exclusive lock before any filesystem modifications
- FR-002 [Must]: Block subsequent processes attempting write operations until the lock is released (wait indefinitely by default)
- FR-003 [Must]: Display clear waiting message when blocked on a lock showing holder details (e.g., "Waiting for lock held by PID 12345 (install) since 10:00:00...")
- FR-004 [Must]: Automatically release locks when the process exits (normally or abnormally) via OS file lock mechanism
- FR-005 [Must]: Allow read-only operations (list, current, search, cache list-distributions) to proceed without locking
- FR-006 [Must]: Support --no-wait flag to fail immediately with exit code 75 if lock is held
- FR-007 [Must]: Include minimal metadata in lock file: pid, command, started_at
- FR-008 [Should]: Include hostname in metadata for network filesystem scenarios
- FR-009 [Could]: Log lock acquisition and release events for debugging
- FR-010 [Must]: Use a single global lock per KOPI_HOME for all write operations (install, uninstall, cache refresh, cache update, shim operations, config changes)
- FR-011 [Must]: Support reentrant locking within the same process to prevent deadlocks when commands invoke other commands internally
- FR-012 [Should]: Detect non-TTY environments and provide appropriate output (no spinner, periodic status updates)
- FR-013 [Must]: Ensure all filesystem write operations use atomic replacement pattern (write to temp file, fsync, rename) to prevent partial reads

## Non-Functional Requirements (NFR)

- NFR-001 [Performance]: Lock acquisition p95 < 10ms for uncontended case (median < 5ms)
- NFR-002 [Performance]: Lock metadata read < 1ms for diagnostics
- NFR-003 [Reliability]: OS file locks ensure automatic cleanup on process termination
- NFR-004 [Reliability]: Handle SIGTERM/SIGINT gracefully to release lock before exit
- NFR-005 [Compatibility]: Support both Unix (flock/fcntl) and Windows (LockFileEx) mechanisms via fs2
- NFR-006 [UX]: English-only messages with clear holder information during wait
- NFR-007 [Security]: Lock file permissions restrict access to user only (0600 on Unix), parent directory permissions 0700
- NFR-008 [Observability]: Display lock holder details when waiting or failing with --no-wait
- NFR-009 [UX]: In non-TTY environments, degrade gracefully to periodic concise status logging without spinners

## CLI/UX Requirements

### Command Behavior

- Write operations automatically acquire lock before proceeding
- Default: wait indefinitely with spinner if lock is held
- Read operations proceed without locking
- Handle Ctrl+C gracefully to release lock

### Options

- `--no-wait`: Fail immediately if lock is held (exit code 75)
- `--quiet`: Suppress spinner and reduce output noise (useful for CI/automation)

### Examples

```bash
# Normal operation - waits if another process holds lock
kopi install temurin@21

# Fail fast if locked
kopi install temurin@21 --no-wait

# Read operations don't require lock
kopi list
kopi current
```

### Help & Messages

- Waiting: "Waiting for lock held by PID 12345 (install) since 10:00:00..."
- No-wait failure: "Lock is held by PID 12345 (install) since 10:00:00. Use default behavior to wait."
- Exit code 75: Temporary failure (lock busy with --no-wait)

## Platform Matrix

### Unix

- Lock mechanism: `flock()` or `fcntl()` via fs2 crate
- Lock file: `~/.kopi/.lock` with permissions 0600
- Signal handling: SIGTERM/SIGINT handlers for graceful cleanup
- Parent directory: `~/.kopi/` with permissions 0700

### Windows

- Lock mechanism: `LockFileEx()` via fs2 crate
- Lock file: `%USERPROFILE%\.kopi\.lock`
- Rely on filesystem ACLs for security
- OS automatically releases lock on process termination

### Lock File Format

- Persistent file (never deleted, only truncated)
- Minimal JSON metadata: `{"pid": 12345, "started_at": "2025-08-31T10:00:00Z", "command": "install", "hostname": "my-host"}`
- Write metadata after acquiring OS lock
- Truncate to 0 bytes on release
- Metadata is for diagnostics only, not for ownership decisions

## Dependencies

- Internal modules:
  - `src/config/` – Access kopi home directory path
  - `src/error/` – Error handling and user feedback
  - `src/commands/` – Integration with all write commands
- External crates:
  - `fs2` – Cross-platform file locking (advisory locks) - same as Volta
  - Note: Keep dependencies minimal - no need for process checking since we rely on OS locks

## Risks & Mitigations

1. Risk: Process crashes without releasing lock
   - Mitigation: OS file locks automatically release on process termination
   - Validation: Kill test process with -9, verify lock is released
   - Fallback: Not needed - OS handles cleanup

2. Risk: Network filesystem doesn't support advisory locking
   - Mitigation: fs2 abstracts platform differences; test on target filesystems
   - Validation: Test on NFS/SMB mounted directories
   - Fallback: Document limitation if found, recommend local filesystem

3. Risk: User interrupts with Ctrl+C during operation
   - Mitigation: Install SIGINT/SIGTERM handlers to release lock gracefully
   - Validation: Test Ctrl+C during various operations
   - Fallback: OS releases lock on termination anyway

4. Risk: Lock holder details become stale after crash
   - Mitigation: Truncate file on unlock; metadata is diagnostic only
   - Validation: Verify empty file after normal release
   - Fallback: Users understand metadata may be stale if process crashed

5. Risk: Reentrant locking causes deadlock when commands invoke other commands
   - Mitigation: Implement process-scoped lock guard that tracks if current process already holds lock
   - Validation: Test nested command invocation (e.g., install calling cache refresh)
   - Fallback: Use thread-local storage or process-global state to track lock ownership

6. Risk: Windows antivirus/EDR software delays or blocks lock file operations
   - Mitigation: Document known issues; recommend exclusions for ~/.kopi directory
   - Validation: Test with common AV software (Windows Defender, etc.)
   - Fallback: Retry with exponential backoff for transient AV interference

7. Risk: Permission errors when creating lock directory or file
   - Mitigation: Create parent directory with 0700 permissions; check and repair if needed
   - Validation: Test with various umask settings and pre-existing directories
   - Fallback: Clear error message with permission fix instructions

8. Risk: Non-TTY environments (CI/automation) get confusing spinner output
   - Mitigation: Detect TTY and switch to periodic single-line status updates
   - Validation: Test in CI environments and with output redirection
   - Fallback: Support --quiet flag to suppress all non-error output

9. Risk: Partial file reads during non-atomic writes
   - Mitigation: All writes use temp file + fsync + atomic rename pattern
   - Validation: Stress test with concurrent readers and writers
   - Fallback: Readers retry on malformed data with exponential backoff

## Acceptance Criteria

- [ ] Satisfies FR-001: No concurrent writes (verified by parallel execution test)
- [ ] Satisfies FR-002: Indefinite waiting works (verified by lock contention test)
- [ ] Satisfies FR-003: Clear waiting message with holder details (verified by UI test)
- [ ] Satisfies FR-004: Lock released on exit via OS mechanism (verified by kill -9 test)
- [ ] Satisfies FR-005: Read operations don't lock (verified by parallel read test)
- [ ] Satisfies FR-006: --no-wait fails with exit code 75 (verified by flag test)
- [ ] Satisfies FR-007: Minimal metadata written correctly (verified by file inspection)
- [ ] Satisfies FR-010: Global lock per KOPI_HOME (verified by environment variable test)
- [ ] Satisfies FR-011: Reentrant locking works (verified by nested command test)
- [ ] Satisfies FR-012: Non-TTY detection works (verified by CI environment test)
- [ ] Satisfies FR-013: Atomic writes prevent partial reads (verified by concurrent stress test)
- [ ] Meets NFR-001: Lock overhead p95 < 10ms (verified by benchmark)
- [ ] Meets NFR-003: OS cleanup works (verified by process termination test)
- [ ] Meets NFR-004: Signal handlers work (verified by Ctrl+C test)
- [ ] Meets NFR-005: Cross-platform support (verified on Unix and Windows)
- [ ] Meets NFR-007: Correct permissions 0600/0700 (verified on Unix)
- [ ] Meets NFR-009: Non-TTY output appropriate (verified by output capture test)
- [ ] Error messages: English, actionable, exit code 75 for --no-wait (NFR-006)

## Verification Plan

- Unit tests: `cargo test --lib --quiet lock` (test lock acquisition, release, metadata, reentrant locking)
- Integration tests: `cargo test --test lock_integration` scenarios:
  - Parallel install attempts (FR-001, FR-002)
  - OS automatic cleanup on kill -9 (FR-004)
  - Read operation concurrency (FR-005)
  - --no-wait behavior with exit code 75 (FR-006)
  - Signal handler cleanup on Ctrl+C (NFR-004)
  - Global lock per KOPI_HOME with different environments (FR-010)
  - Reentrant locking with nested commands (FR-011)
  - Non-TTY environment detection and output (FR-012, NFR-009)
  - Atomic write operations under concurrent access (FR-013)
  - Unix permissions verification 0600/0700 (NFR-007)
- Performance: `cargo bench lock_overhead` for NFR-001 (p95 < 10ms)
- Platform: CI matrix verification on Linux, macOS, Windows
- Stress test: 100 parallel processes attempting writes with no corruption

## Traceability

| Requirement | Design Section   | Test(s) / Benchmarks                              | Status  |
| ----------- | ---------------- | ------------------------------------------------- | ------- |
| FR-001      | Lock Acquisition | tests/lock_integration::test_exclusive_write      | Pending |
| FR-002      | Wait Behavior    | tests/lock_integration::test_wait_indefinitely    | Pending |
| FR-003      | User Feedback    | tests/lock_integration::test_wait_message_details | Pending |
| FR-004      | OS Cleanup       | tests/lock_integration::test_os_cleanup_on_crash  | Pending |
| FR-005      | Read Operations  | tests/lock_integration::test_concurrent_reads     | Pending |
| FR-006      | No-Wait Flag     | tests/lock_integration::test_no_wait_exit_75      | Pending |
| FR-007      | Metadata         | tests/lock_integration::test_lock_metadata        | Pending |
| FR-010      | Global Lock      | tests/lock_integration::test_global_lock_per_home | Pending |
| FR-011      | Reentrant Lock   | tests/lock_integration::test_reentrant_locking    | Pending |
| FR-012      | Non-TTY          | tests/lock_integration::test_non_tty_output       | Pending |
| FR-013      | Atomic Writes    | tests/lock_integration::test_atomic_writes        | Pending |
| NFR-001     | Performance      | benches/lock_bench::bench_acquisition_p95         | Pending |
| NFR-003     | OS Lock Release  | tests/lock_integration::test_kill_cleanup         | Pending |
| NFR-004     | Signal Handling  | tests/lock_integration::test_signal_handlers      | Pending |
| NFR-005     | Platform Support | tests/lock_integration::test_platform_lock        | Pending |
| NFR-007     | Permissions      | tests/lock_integration::test_unix_permissions     | Pending |
| NFR-009     | Non-TTY UX       | tests/lock_integration::test_non_tty_ux           | Pending |

## Open Questions

- ~~Should we support recursive locking for nested kopi commands?~~ → **Resolved**: Yes, implement reentrant locking (FR-011)
- ~~Should we add hostname to metadata for better network filesystem support?~~ → **Resolved**: Yes, include hostname (FR-008)
- ~~Should lock path be `~/.kopi/.lock` or `~/.kopi/locks/global.lock` for future extensibility?~~ → **Resolved**: Use `~/.kopi/.lock` (YAGNI - keep it simple)

## Related Research

### Other Version Manager Tools

Investigation of similar tools reveals different approaches to process locking:

**Volta:**

- Uses file lock on `volta.lock` file with reference counting
- Implements RAII pattern with automatic cleanup
- Shows spinner when lock is contended
- Uses `fs2` crate for cross-platform file locking
- No timeout mechanism - relies on process cleanup

**mise:**

- Focuses on `mise.lock` for version pinning (like package-lock.json)
- Basic mutex for thread safety within process
- No explicit inter-process locking found in public implementation
- Has experienced concurrent installation issues similar to rustup

**Key Insights:**

- Neither tool implements lock timeouts
- Both rely on OS-level file locking mechanisms
- Automatic cleanup on process termination is standard
- Simple is better - complex timeout logic introduces more problems

### Codex Architecture Review

After discussing with Codex AI, the following design decisions were validated:

- **No timeout mechanism**: Rely on user Ctrl+C for interruption
- **Minimal metadata**: Just pid, command, started_at (and optionally hostname)
- **Single global lock**: Simpler than per-operation locks, avoids deadlocks
- **OS lock reliance**: Don't try to detect/steal stale locks, trust OS cleanup
- **Exit code 75**: Standard temporary failure code for --no-wait
- **Persistent lock file**: Never delete, only truncate (avoids race conditions)

## Change Log

- 2025-08-31: Initial draft created
- 2025-08-31: Removed lock-timeout feature - processes should complete quickly or crash cleanly
- 2025-08-31: Added research on volta and mise lock implementations
- 2025-08-31: Simplified design based on Codex discussion - minimal metadata, OS lock reliance, no timeouts
- 2025-08-31: Incorporated Codex review feedback:
  - Added FR-010 to FR-013 (global lock scope, reentrant locking, non-TTY support, atomic writes)
  - Relaxed NFR-001 performance requirement to p95 < 10ms
  - Added NFR-009 for non-TTY environment handling
  - Clarified scope with specific commands requiring locks
  - Added risks for reentrant locking, Windows AV, permissions, non-TTY, partial reads
  - Resolved open questions: use `~/.kopi/.lock` (YAGNI principle), support reentrant locking, include hostname
  - Added --quiet option for CI environments

---
