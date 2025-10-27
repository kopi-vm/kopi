# T-m13bb Cache Lock Serialization Design

## Metadata

- Type: Design
- Status: Approved
  <!-- Draft: Work in progress | Approved: Ready for implementation | Rejected: Not moving forward with this design -->

## Links

- Associated Plan Document:
  - [T-m13bb-cache-locking-plan](./plan.md)

## Overview

Introduce an exclusive cache writer lock that reuses Kopi's advisory/fallback locking controller so cache refreshes and distribution updates never interleave, while keeping cache reads lock-free. The design adds a cache-specific lock guard, instruments contention via the existing status reporter bridge, and hardens metadata writes with durable temp-file swaps that honour configured lock timeouts.

## Success Metrics

- [ ] Stress runs verify only one cache refresh writes at a time; additional writers block or honour timeout budgets exposed by `LockTimeoutResolver`.
- [ ] Concurrent reader tests complete 100 iterations without reading partially written cache payloads.
- [ ] Cache writes fsync the temporary file, perform an atomic rename, and leave no `.tmp` remnants after forced termination scenarios.

## Background and Current State

- Context: Cache refreshes occur in `cache::fetch_and_cache_metadata_with_progress` and `cache::fetch_and_cache_distribution`, called by `kopi cache refresh`, automatic install refreshes, and doctor checks.
- Current behaviour: Writers operate without cross-process locking; each invocation re-creates the metadata JSON using temporary files but does not guard against concurrent writers or fsync data before rename.
- Pain points: Parallel refreshes can race, corrupt the JSON, or delete a competing writer's temp file; progress indicators cannot surface wait diagnostics, and readers only learn about corruption once deserialization fails.
- Constraints: Reuse ADR-8mnaz locking strategy, avoid new dependencies, stay cross-platform, and leave read paths lock-free to minimise latency.
- Related ADRs: [`docs/adr/ADR-8mnaz-concurrent-process-locking-strategy.md`](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md).

## Proposed Design

### High-Level Architecture

```text
┌────────────────────┐     ┌────────────────────────┐     ┌─────────────────────────┐
│ CLI / background   │     │ Cache Refresh Orchestr │     │ Metadata cache storage  │
│ invocations        │────▶│ with CacheWriterLock   │────▶│ (temp write + fsync +   │
│ (install, cache)   │     │ Guard                  │     │ atomic rename)          │
└────────────────────┘     └────────────────────────┘     └─────────────────────────┘
                                    │
                                    ▼
                           LockController (advisory → fallback)
```

### Components

- **CacheWriterLockGuard** (`src/cache/lock.rs`): RAII wrapper around `LockController::acquire_with_feedback` for `LockScope::CacheWriter`. Captures backend (advisory vs fallback) for logging, releases via `Drop`, and exposes `acquired_backend()` for diagnostics.
- **CacheWriterLock** helper (`src/cache/lock.rs`): Constructs a `LockController` using `KopiConfig.locking`, wires either a `StatusReporter` (for silent contexts) or shared progress indicator (for CLI refresh), and returns the guard.
- **Cache refresh orchestration** (`src/cache/mod.rs`): Wraps existing `fetch_and_cache_*` flows with `CacheWriterLockGuard`, surfacing wait messages through `StatusReporterObserver` so contention becomes visible in logs and progress output.
- **Durable save routine** (`src/cache/storage.rs`): Rewrites `save_cache` to open the temp file with `std::fs::File`, `write_all`, `sync_all`/`FlushFileBuffers`, then `platform::file_ops::atomic_rename`. Retains orphan cleanup but ensures fsync before rename and retries rename on transient sharing violations (Windows) using backoff aligned with lock timeouts.
- **Telemetry bridge** (`src/cache/mod.rs`): Emits structured `info!` lines (`lock_backend`, wait duration) and hooks `StatusReporter` for interactive progress. Per stakeholder guidance, no telemetry counters are added; logging alone satisfies FR-c04js dependencies.

### Data Flow

1. Caller (CLI refresh, installer auto-refresh, doctor checks) requests a cache refresh.
2. `CacheWriterLock` resolves lock timeouts via existing `LockController` configuration and acquires `LockScope::CacheWriter`, reporting waits through `StatusReporterObserver`.
3. With the guard held, the refresh fetches metadata from foojay, converts and aggregates packages, and calls `save_cache`.
4. `save_cache` writes the JSON into a sibling temp file, `sync_all`s the handle, then atomically renames to the final path using a fixed retry window bounded by the resolved lock timeout (no user-facing configuration).
5. Guard drops, releasing the lock and logging the backend and total wait duration only when an interactive progress indicator is active.
6. Readers continue to call `load_cache` without locks; if deserialization fails they trigger a single refresh attempt guarded by the writer lock.

### Storage Layout and Paths (if applicable)

- Lock file path remains `$KOPI_HOME/locks/cache.lock` managed by `LockScope::CacheWriter`.
- Temporary cache files follow `<cache>.tmpXXXX` in the same directory to keep rename atomic across platforms.

### CLI/API Design (if applicable)

- Reuse existing progress UI. `kopi cache refresh` gains explicit status lines such as "Waiting for cache writer lock" via the shared `ProgressIndicator` bridge; silent or non-interactive flows suppress wait messaging.
- No new flags; lock timeout remains controlled by the global resolver introduced in T-lqyk8.

### Data Models and Types

- `pub struct CacheWriterLockGuard<'a> { controller: &'a LockController, acquisition: Option<LockAcquisition>, backend: LockBackend, started_at: Instant }`
- `pub struct CacheWriterLock<'a> { controller: LockController, reporter: Option<StatusReporter> }`
- Extend `MetadataCache` save path to accept a `&CacheWriterLockGuard` reference for logging (no behavioural change for reads).

### Error Handling

- Propagate existing `KopiError::LockingAcquire`, `LockingTimeout`, and `LockingCancelled` from `LockController`.
- Wrap fsync/rename failures in `KopiError::ConfigError` with actionable English messages.
- On write failure, ensure the guard scope drops so other processes can attempt recovery; callers fall back to retry logic defined in FR-gbsz6.

### Security Considerations

- Lock files remain zero-length placeholders under user-owned `$KOPI_HOME/locks`; no additional sensitive data is stored.
- Temp files inherit restrictive permissions from parent directories; we explicitly set `FileOptionsExt` on Windows to prevent world-readable handles.

### Performance Considerations

- Lock acquisition reuses exponential backoff (`PollingBackoff`) from `LockController`; acquiring a guard adds only one controller instantiation per refresh.
- `sync_all` adds \~1–2 ms per write on local disks but guarantees durability; cache refresh frequency is low so overhead is acceptable.
- Reads remain unchanged and lock-free, preserving lookup latency.

### Platform Considerations

#### Unix

- Use `File::sync_all` followed by `rename` (POSIX atomic) while holding the lock. Ensure temp files reside on same filesystem.

#### Windows

- Call `handle.flush()` via `File::sync_all` (maps to `FlushFileBuffers`) and rely on `platform::file_ops::atomic_rename`, retrying on `ERROR_SHARING_VIOLATION` within a fixed retry window bounded by the resolved lock timeout. The retry strategy uses exponential backoff (start 50 ms, double each attempt, cap at 1 s) and aborts once either the rename succeeds or the cumulative wait reaches the lock timeout budget. This avoids long hangs while giving SMB/UNC shares enough time to release handles.

#### Filesystem

- Detect network filesystems via the existing inspector; if advisory locks are downgraded to fallback, guard still serialises writers because fallback holds the staged swap until completion.

## Alternatives Considered

1. **Continue without cross-process locking**
   - Pros: No new code.
   - Cons: Leaves corruption risk unresolved, violates FR-v7ql4; no contention telemetry.

2. **PID-based lock files**
   - Pros: Works even if advisory locks fail.
   - Cons: Requires manual cleanup, duplicates fallback staging logic already decided against in ADR-8mnaz.

Decision Rationale

- Adopting the `LockController` keeps behaviour consistent with install/uninstall flows, provides timeout observability, and satisfies ADR-8mnaz without reinventing locking.

## Migration and Compatibility

- Backward compatible: cache file format unchanged; existing caches remain valid.
- Rollout: ship guard alongside code changes; no feature flag required. On failure we log actionable messages and leave existing cache untouched.
- Deprecation: none.

## Testing Strategy

### Unit Tests

- Add tests for `CacheWriterLockGuard` verifying release-on-drop and backend reporting using `TempDir` controllers.
- Extend `storage::tests::test_save_and_load_cache` to assert temp file removal and durable write via simulated interruption (e.g., rename after sync).
- Add serialization retry test verifying `save_cache` handles Windows sharing violations by retrying with backoff.

### Integration Tests

- New `tests/cache_locking.rs` exercising two concurrent refresh processes: first holds lock, second times out per configured budget.
- Stress test with multiple reader threads loading cache while a writer refreshes, asserting zero JSON parse failures over 100 iterations.
- Platform smoke under `tests/locking_platform_integration.rs` extended to cover `LockScope::CacheWriter` acquisition path.

### External API Parsing (if applicable)

- Reuse existing captured Foojay responses in cache tests; no new external API parsing required beyond current fixtures.

### Performance & Benchmarks (if applicable)

- Optional micro-benchmark comparing refresh duration before/after fsync to document overhead; not required for initial rollout.

## Documentation Impact

- Update `docs/reference.md` to mention cache refresh contention messaging.
- Coordinate with the user-docs repo to describe lock wait behaviour if CLI messaging changes.

## External References (optional)

- [Rust std::fs::File::lock documentation](https://doc.rust-lang.org/std/fs/struct.File.html) – baseline advisory locking semantics.
- [Windows `FlushFileBuffers` guidance](https://learn.microsoft.com/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers) – ensures durable writes before rename.

## Open Questions

- [ ] Confirm logging-only feedback meets FR-c04js expectations for lock contention without introducing telemetry counters. → Coordinate with task T-60h68 only if future instrumentation is required.
- [ ] Validate through targeted tests that the 50 ms → 1 s capped exponential backoff stays within the lock timeout budget yet consistently overcomes Windows sharing violations (e.g., mocked `ERROR_SHARING_VIOLATION` loops and UNC path integration tests).

## Appendix

### Diagrams

```text
Caller ─► CacheWriterLock ─► LockController ─► (Advisory | Fallback)
                                   │
                                   └─► Save cache (temp write -> sync -> rename)
```

### Examples

```bash
# Two concurrent cache refresh commands
kopi cache refresh &
kopi cache refresh --lock-timeout 5
# Second command prints wait status and times out after 5s if the first holds the lock.
```

### Glossary

- **Cache writer lock**: Exclusive lock for metadata cache mutations defined by `LockScope::CacheWriter`.
- **Fallback backend**: Atomic staging path used when advisory locks are unsupported (network filesystems).

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../../templates/README.md#design-template-designmd) in the templates README.
