# T-m13bb Cache Lock Serialization Plan

## Metadata

- Type: Implementation Plan
- Status: Phase 1 In Progress
  <!-- Draft: Planning complete, awaiting start | Phase X In Progress: Actively working | Cancelled: Work intentionally halted before completion | Complete: All phases done and verified -->

## Links

- Associated Design Document:
  - [T-m13bb-cache-locking-design](./design.md)

## Overview

Introduce an exclusive cache writer lock around all metadata refresh paths, surface contention feedback, and harden cache persistence so concurrent Kopi processes never observe partially written cache files.

## Success Metrics

- [ ] Demonstrate single-writer enforcement under stress by running two concurrent refresh commands; second invocation blocks or times out per configuration.
- [ ] Execute 100 reader iterations during a refresh without JSON parse failures or truncated reads.
- [ ] Verify temporary cache files are fsynced, atomically renamed, and removed after simulated crashes.
- [ ] All existing tests pass; regressions avoided in cache loading and install flows.

## Scope

- Goal: Serialise cache mutations with `LockScope::CacheWriter`, maintain lock-free reads, and guarantee durable writes that respect timeout policy.
- Non-Goals: Installation/uninstallation locking (covered by other tasks); redesigning cache schema or metadata fetch logic.
- Assumptions: ADR-8mnaz locking infrastructure is stable; timeout resolver from T-lqyk8 is already wired; telemetry pipeline accepts `info!` logs for contention.
- Constraints: Cross-platform parity, no `unsafe`, English user messaging, avoid vague naming (`manager`/`util`).

## ADR & Legacy Alignment

- [ ] Confirm ADR-8mnaz guidance is followed for advisory vs. fallback behaviour.
- [ ] Audit existing cache write paths to remove legacy temp-file handling in favour of the new durable implementation.

## Plan Summary

- Phase 1 – Cache Lock Orchestration
- Phase 2 – Durable Cache Persistence
- Phase 3 – Testing & Integration Hardening

### Phase Status Tracking

Mark checkboxes (`[x]`) immediately after completing each task or subtask. If an item is intentionally skipped or deferred, annotate it instead of leaving it unchecked.

---

## Phase 1: Cache Lock Orchestration

### Goal

- Provide a cache-specific lock guard and wire it into all metadata refresh entry points with contention feedback hooks.

### Inputs

- Documentation:
  - `/docs/tasks/T-m13bb-cache-locking/design.md` – Architectural decisions for locking and telemetry.
- Source Code to Modify:
  - `/src/cache/mod.rs` – Refresh orchestration functions.
  - `/src/cache/tests.rs` – Existing cache behaviour tests.
  - `/src/commands/cache.rs` & `/src/commands/install.rs` – Callers that trigger refreshes.
- Dependencies:
  - Internal: `src/locking/` – LockController, wait observers, timeout resolver.

### Tasks

- [ ] **Guard scaffolding**
  - [ ] Add `src/cache/lock.rs` with `CacheWriterLockGuard` and helper constructors.
  - [ ] Integrate with `LockController::acquire_with_feedback` and `StatusReporter` fallback for silent contexts.
- [ ] **Call site integration**
  - [ ] Wrap `fetch_and_cache_metadata_with_progress` and `fetch_and_cache_distribution` in the guard lifecycle.
  - [ ] Update CLI/automatic refresh entry points to log backend and wait duration using existing logging facilities.
- [ ] **Interactive feedback**
  - [ ] Ensure contention messages flow through `StatusReporterObserver` and `info!` logs for interactive progress only; silent/non-interactive paths remain quiet.

### Deliverables

- New cache lock helper module with unit coverage.
- Updated refresh functions using the guard and emitting contention feedback.

### Verification

```bash
cargo check --package kopi
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet cache
```

### Acceptance Criteria (Phase Gate)

- Guard drops release the lock deterministically; logging shows backend selection during manual refresh.
- Concurrent refresh attempts block with observable wait messaging.

### Rollback/Fallback

- Revert to previous cache module revisions and remove the guard helper; existing behaviour remains functional though unsafe.

---

## Phase 2: Durable Cache Persistence

### Phase 2 Goal

- Guarantee cache writes are durable and atomic, with retries aligned to lock timeout policy.

### Phase 2 Inputs

- Dependencies:
  - Phase 1 guard in place to serialise writers.
  - `platform::file_ops` utilities for atomic rename.
- Source Code to Modify:
  - `/src/cache/storage.rs` – Save/load implementations.
  - `/src/platform/file_ops.rs` (if extensions needed for fsync/rename retries).

### Phase 2 Tasks

- [ ] **Durable temp write**
  - [ ] Replace `fs::write` with explicit `File` creation, `write_all`, and `sync_all`/`FlushFileBuffers`.
  - [ ] Ensure temp files inherit restrictive permissions and reside alongside the final cache.
- [ ] **Atomic swap retry**
  - [ ] Introduce a fixed exponential-backoff retry loop for `atomic_rename` (start 50 ms, double per attempt, cap at 1 s) and enforce an upper bound equal to the lock timeout budget.
  - [ ] Preserve orphan cleanup on startup while avoiding deletion of an active writer’s temp file.
  - [ ] Capture metrics (retry count, elapsed time) during tests to prove the retry window respects the lock timeout while succeeding under simulated `ERROR_SHARING_VIOLATION` conditions.
- [ ] **Error surface**
  - [ ] Wrap failures in `KopiError::ConfigError` with actionable English hints, referencing lock contention when applicable.

### Phase 2 Deliverables

- Updated persistence logic meeting FR-v7ql4 acceptance criteria.
- Unit tests covering fsync/rename behaviour and orphan handling.

### Phase 2 Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet cache::storage
```

### Phase 2 Acceptance Criteria

- Tests confirm fsync occurs before rename; temporary files are absent after success and cleaned safely after simulated crashes.
- Windows retry path validated via mocked sharing violation scenario.

### Phase 2 Rollback/Fallback

- Restore previous `storage.rs` implementation (recorded via git) to return to existing temp write behaviour.

---

## Phase 3: Testing & Integration

### Phase 3 Goal

- Create comprehensive automated coverage for lock contention, reader safety, and crash recovery.

### Phase 3 Tasks

- [ ] Test utilities
  - [ ] Add helpers to simulate concurrent refresh invocations using threads or command harness.
  - [ ] Capture Foojay API sample JSON within tests per external API policy.
- [ ] Scenarios
  - [ ] Happy path: single refresh obtains lock, writes cache, readers succeed.
  - [ ] Timeout path: second writer respects configured timeout and surfaces error messaging.
  - [ ] Reader safety: concurrent readers and writers over 100 iterations without parse failures.
- [ ] Concurrency & cleanup
  - [ ] Validate fallback backend path (network filesystem simulation) still serialises writes.
  - [ ] Verify orphan temp file cleanup on startup hygiene.

### Phase 3 Deliverables

- Integration test suite (`tests/cache_locking.rs`) and expanded unit coverage ensuring lock guarantees.

### Phase 3 Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet
cargo test --quiet --tests
```

### Phase 3 Acceptance Criteria

- All new and existing tests green on Unix and Windows CI targets; documented known limitations (if any) captured in README.

---

## Definition of Done

- [ ] `cargo check`
- [ ] `cargo fmt`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --lib --quiet`
- [ ] `cargo test --quiet --tests`
- [ ] Update `docs/reference.md` and upstream docs if CLI messaging changes
- [ ] Ensure no `unsafe` code, avoid vague naming, and run `bun scripts/trace-status.ts --write`
- [ ] Confirm design/plan links in `docs/traceability.md`

## Open Questions

- [ ] Confirm the logging-only approach (with interactive progress messaging only) satisfies FR-c04js requirements without introducing telemetry counters.
- [ ] Finalise and document the retry timing (initial delay, cap, maximum attempts) that guarantees success in Windows sharing-violation tests without exceeding the lock timeout.

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../../templates/README.md#plan-template-planmd) in the templates README.
