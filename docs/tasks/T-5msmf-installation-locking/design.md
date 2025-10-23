# T-5msmf Installation Lock Integration Design

## Metadata

- Type: Design
- Status: Draft
  <!-- Draft: Work in progress | Approved: Ready for implementation | Rejected: Not moving forward with this design -->

## Links

- Associated Plan Document:
  - [T-5msmf-installation-locking-plan](./plan.md)

## Overview

Connect the approved locking foundation to the installation pipeline so every mutation of a JDK installation directory runs under a scoped lock. The design introduces a small coordination layer inside `InstallCommand` that canonicalises the target coordinate, acquires a lock via `LockController`, and ensures release even on error paths. Lock wait status reuses the existing `StatusReporter` surface so users see clear feedback when contention or fallback behaviour occurs.

## Success Metrics

- [ ] Concurrent `kopi install temurin@21` runs never overlap in filesystem writes during integration tests that spawn parallel processes.
- [ ] Lock acquisition plus release adds ≤100 ms to uncontended installs on local filesystems compared to the current baseline (measured with `cargo bench install_pipeline` micro-benchmark).
- [ ] Fallback acquisitions leave zero `.lock` or `.marker` artefacts after abnormal termination scenarios exercised by `tests/locking_lifecycle.rs`.

## Background and Current State

- Context: `kopi install` orchestrates metadata lookup, download, extraction, and metadata writes under `src/commands/install.rs`, with filesystem staging handled by `src/storage/repository.rs`.
- Current behavior: Installations run without cross-process coordination. `LockController` (implemented in T-ec5ew) and timeout controls (T-lqyk8) are unused by the install command, so two copies of the CLI can mutate the same coordinate concurrently.
- Pain points: Forced reinstall (`--force`) can interleave directory removal and extraction. Crash recovery relies on best-effort cleanup but does not prevent double writes. Users receive no feedback when another install is already busy.
- Constraints: Must honour ADR-8mnaz by preferring standard library advisory locks and falling back to atomic staging on network filesystems. Avoid introducing new crates or unsafe code. Respect `--lock-timeout` CLI override and existing progress indicator semantics.
- Related ADRs: [`docs/adr/ADR-8mnaz-concurrent-process-locking-strategy.md`](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)

## Proposed Design

### High-Level Architecture

```text
CLI install command
        │
        ▼
InstallCommand::execute
        │
        ├─ Metadata lookup & package resolution (read-only)
        │
        ├─ InstallationLockCoordinator
        │       │
        │       ├─ PackageCoordinate::try_from_package
        │       ├─ LockController::acquire_with_status_sink
        │       └─ InstallationLockGuard (RAII release)
        │
        └─ Locked pipeline (cache refresh, download, extraction, metadata, shims)
```

### Components

- **InstallationLockCoordinator** (new helper in `src/commands/install.rs`):
  - Builds a `PackageCoordinate` from the resolved `Package`.
  - Instantiates `LockController::with_default_inspector` using `KopiConfig::kopi_home()` and `config.locking`.
  - Binds lock wait feedback to a `StatusReporter` configured with the current `--no-progress` flag.
  - Returns an `InstallationLockGuard` RAII object that owns the `LockAcquisition`.
- **InstallationLockGuard** (new struct):
  - Holds the `LockController` reference and `LockAcquisition`.
  - On `Drop`, ensures `LockController::release` is called; logs warnings if release fails.
  - Exposes `backend()` so telemetry/logging can record whether advisory or fallback was used.
- **InstallCommand adjustments**:
  - Moves installation directory checks, force-cleanup, download, extraction, metadata writes, and shim creation inside the guard’s scope.
  - Adds an explicit progress step (“Acquiring installation lock”) before filesystem mutation begins.
  - Records lock backend in `info!` logs for observability.
- **StatusReporter integration**:
  - Reuses `StatusReporterObserver` so contention emits `step/success/error` messages without breaking progress indicators. Messages execute via `progress.suspend` to avoid corrupting the active progress bar.

### Data Flow

1. Parse CLI input and resolve `VersionRequest` (unchanged).
2. Compute total progress steps, but defer starting the mutable phase until a lock is acquired.
3. Resolve the target `Package` via cache/metadata. Canonicalise the coordinate with `PackageCoordinate::try_from_package`.
4. Call `InstallationLockCoordinator::acquire(scope)`:
   - Build `LockScope::installation(coordinate)`.
   - Use `LockController::acquire_with_status_sink` with a `StatusReporter` sink wrapped in `progress.suspend`.
   - Return `InstallationLockGuard`.
5. Start/update the progress indicator, inserting a dedicated “Acquiring installation lock” step (counts toward totals).
6. Within the guard's lifetime perform installation steps (`ensure_fresh_cache`, downloads, extraction, metadata writes, shim creation).
7. On normal completion, explicitly call `InstallationLockGuard::release()` to surface release errors; `Drop` provides fallback.
8. Any early return or error propagates through `Result`, with the guard’s drop preventing stale locks.

### Storage Layout and Paths (if applicable)

- Locks: `$KOPI_HOME/locks/install/<distribution>/<slug>.lock` via `LockScope::lock_path`.
- Fallback markers: `$KOPI_HOME/locks/install/<distribution>/<slug>.lock.marker` (automatically removed by guard).
- Temporary staging remains under `$KOPI_HOME/jdks/.tmp/install-<uuid>` via `JdkInstaller`.

### CLI/API Design (if applicable)

No new CLI flags. Existing `--lock-timeout` override already supported by `KopiConfig::apply_lock_timeout_overrides`; the coordinator reads the resolved timeout source for user feedback strings (e.g., “source CLI”).

### Data Models and Types

- `InstallationLockGuard<'a>`:
  - Fields: `controller: &'a LockController`, `acquisition: Option<LockAcquisition>`.
  - Methods: `backend() -> LockBackend`, `release(self) -> Result<()>`.
- `InstallationLockCoordinator<'a>` (private helper struct or function namespace) to encapsulate lock acquisition while keeping command logic focused on install flow.

### Error Handling

- Propagate locking failures (`KopiError::LockingTimeout`, `LockingCancelled`, `LockingAcquire`) without wrapping so exit codes remain accurate.
- When release fails during explicit `release()`, bubble up as `KopiError::LockingRelease`. If release fails during `Drop`, log a warning via `warn!` while preserving the original error return to the user.
- Convert contention waits into `StatusReporter.step` messages; no change to exit codes.

### Security Considerations

- Guard ensures single-writer semantics for `$KOPI_HOME/locks/install/...` and staged directories, preventing TOCTOU race conditions that could allow path traversal.
- Lock acquisition uses existing permission defaults (`0o600` advisory files); fallback path inherits RAII cleanup and marker hygiene from T-ec5ew.
- No sensitive data printed in contention messages; coordinate slug is already visible in logs.

### Performance Considerations

- Lock acquisition reuses `LockController` exponential backoff (10 ms → 1 s cap). Under no contention, advisory locks should complete in <1 ms.
- Avoid repeated filesystem inspection by reusing a single `LockController` instance per install invocation.
- The guard defers downloads until the lock is held, preventing duplicate network transfers under contention but adding at most a few milliseconds of overhead.

### Platform Considerations

#### Unix

- Advisory locks rely on `flock`; fallback engages for NFS/CIFS as detected by `DefaultFilesystemInspector`.
- Ensure `progress.suspend` handles terminals without ANSI support (already respected by `StatusReporter`).

#### Windows

- Advisory locks rely on `LockFileEx`. Fallback handles UNC paths and FAT volumes. `StatusReporter` continues to print plain text when colors are disabled.
- Directory paths use `PathBuf` and remain Unicode-aware; no additional escaping needed.

#### Filesystem

- Coordinate slugs use `sanitize_segment`, yielding deterministic lowercase file names.
- Fallback retains atomic create-with-rename semantics already encapsulated by `LockController`; no extra staging directories are added beyond existing `.tmp` tree.

## Alternatives Considered

1. **Acquire lock only for finalisation phase**
   - Pros: Shorter lock hold time; downloads occur in parallel.
   - Cons: Still allows concurrent extraction/removal inside temp directories, leaving corruption risk; does not meet requirement for full installation serialization.
2. **Introduce a global install queue service**
   - Pros: Centralises contention logic for future operations.
   - Cons: Over-engineered for CLI; adds IPC complexity and daemon lifecycle not justified by requirements.

**Decision Rationale**: A dedicated guard within `InstallCommand` is the minimal change that enforces mutual exclusion across the entire mutation window while leveraging the existing locking foundation.

## Migration and Compatibility

- Installation UX: Users may see new messages when waiting for locks; ensure translations stay in English per policy.
- Future tasks (`T-98zsb`, `T-m13bb`) can reuse the coordinator pattern by extracting shared helpers if needed; the guard is intentionally general-purpose.
- No data format changes; installed JDK directories remain intact. Existing partial installs rely on the same staging cleanup.

## Testing Strategy

### Unit Tests

- Add tests for `InstallationLockGuard` ensuring explicit release propagates errors and drop logs warnings.
- Unit-test helper that transforms `Package` into `LockScope` to verify slug canonicalisation (JavaFX, libc variants).
- Extend `StatusReporterObserver` recording sink tests to cover suspend integration.

### Integration Tests

- Add a new ignored test `tests/install_locking.rs` spawning two `kopi install` subprocesses targeting the same coordinate, asserting one waits and both complete successfully.
- Reuse `tests/locking_lifecycle.rs` fixtures to simulate fallback acquisitions by forcing inspector to mark the filesystem as networked.
- Verify forced reinstalls (`--force`) under contention by orchestrating simultaneous runs.

### External API Parsing (if applicable)

- Not applicable; no new Foojay parsing introduced.

### Performance & Benchmarks (if applicable)

- Update or add `benches/install_pipeline.rs` to compare before/after lock integration using `criterion`.
- Capture metrics for uncontended vs contended scenarios to validate success metric #2.

## Documentation Impact

- Update `docs/architecture.md` to note that install, uninstall, and cache flows now consume `LockController`.
- Add troubleshooting entry in user docs (external repository) describing lock wait messages and timeout resolution.
- Document new lock step in `docs/error_handling.md` for `KopiError::LockingTimeout`.

## External References (optional)

- [Cargo lock coordination](https://github.com/rust-lang/cargo/blob/master/src/cargo/util/flock.rs) – comparison point for CLI signalling and RAII guards.
- [Rust 1.89.0 release notes](https://blog.rust-lang.org/2025/08/07/Rust-1.89.0/index.html) – advisory lock stabilisation.

## Open Questions

- [ ] Should install locking logic be extracted into a shared module immediately to unblock uninstall/cache tasks, or is duplication acceptable until those tasks land? → Next step: Validate during plan review.
- [ ] How aggressively should we log fallback acquisitions? (INFO vs DEBUG) → Next step: Decide with observability owners when implementing.
- [ ] Do we need a user-facing hint when `--lock-timeout infinite` is in effect to avoid confusion about indefinite waits? → Method: Prototype messaging in Phase 2 and solicit UX feedback.

## Appendix

### Diagrams

```text
Lock wait flow:

StatusReporterObserver ─┐
                        ├─> progress.suspend(|| StatusReporter::step/… )
InstallCommand -------->│
                        └─> LockController::acquire -> LockHandle/FallbackHandle
```

### Examples

```bash
# Concurrent install of the same coordinate
kopi install temurin@21 &
kopi install temurin@21 &
# Second invocation prints:
#   Waiting for installation temurin-21-jdk-x64 lock (timeout 600s, source config)
#   Lock contention detected for installation temurin-21-jdk-x64…
```

### Glossary

- **Installation lock**: Exclusive guard over the canonicalised vendor/version/os/arch coordinate.
- **Fallback acquisition**: Atomic file-based lock used when advisory locks are unreliable.
- **StatusReporter**: Lightweight textual status output used for non-progress feedback.

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../../templates/README.md#design-template-designmd) in the templates README.
