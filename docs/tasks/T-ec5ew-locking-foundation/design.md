# Locking Foundation Design

## Metadata

- Type: Design
- Status: Draft

## Links

- Related Requirements:
  - [FR-02uqo-installation-locking](../../requirements/FR-02uqo-installation-locking.md)
  - [FR-ui8x2-uninstallation-locking](../../requirements/FR-ui8x2-uninstallation-locking.md)
  - [FR-v7ql4-cache-locking](../../requirements/FR-v7ql4-cache-locking.md)
  - [FR-gbsz6-lock-timeout-recovery](../../requirements/FR-gbsz6-lock-timeout-recovery.md)
  - [FR-c04js-lock-contention-feedback](../../requirements/FR-c04js-lock-contention-feedback.md)
  - [NFR-g12ex-cross-platform-compatibility](../../requirements/NFR-g12ex-cross-platform-compatibility.md)
  - [NFR-vcxp8-lock-cleanup-reliability](../../requirements/NFR-vcxp8-lock-cleanup-reliability.md)
  - [NFR-z6kan-lock-timeout-performance](../../requirements/NFR-z6kan-lock-timeout-performance.md)
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)

## Overview

We will introduce a dedicated locking subsystem that wraps Rust's `std::fs::File` advisory locking on supported filesystems while providing deterministic fallbacks and cleanup across Linux, macOS, Windows, and WSL. The design focuses on a reusable API that downstream installation, uninstallation, and cache workflows can depend on without first re-deriving platform logic or filesystem detection.

## Success Metrics

- [ ] Cross-platform lock acquisition and release parity verified on Linux, macOS, Windows, and WSL in CI.
- [ ] Startup hygiene removes all orphaned lock artifacts in reliability stress tests (1000 crash simulations).
- [ ] Network filesystem downgrade paths emit structured warnings and safely fall back to atomic rename sequences.

## Background and Current State

- Context: Kopi currently lacks a unified process-level locking API, forcing each feature to re-implement ad-hoc protections or skip safety entirely.
- Current behavior: No cross-process coordination exists; limited file-in-use checks rely on the `fs2` crate and do not cover all platforms.
- Pain points: Race conditions during concurrent installs or cache refreshes, no stale lock cleanup policy, and brittle platform detection.
- Constraints: Must rely on the standard library (`std::fs::File` locks) outlined in ADR-8mnaz, avoid unsafe code, and deliver identical semantics on Windows and Unix variants.
- Related ADRs: `ADR-8mnaz-concurrent-process-locking-strategy.md` defines the desired lock approach and filesystem fallback philosophy.

## Proposed Design

### High-Level Architecture

```text
+----------------+     +---------------------+     +----------------------+
| Command caller | --> | LockController      | --> | FilesystemInspector  |
+----------------+     |  (API + caching)    |     |  (per-OS probes)     |
                       +----------+----------+     +----------+-----------+
                                  |                           |
                                  v                           v
                       +---------------------+     +----------------------+
                       | AdvisoryBackend     |     | AtomicFallback       |
                       | (std::fs::File RAII)|     | (rename + warnings)  |
                       +---------------------+     +----------------------+
                                  |
                                  v
                       +---------------------+
                       | LockHygieneRunner   |
                       | (startup cleanup)   |
                       +---------------------+
```

### Components

- `LockController`: Entry point that resolves lock scopes, queries filesystem capability details on demand, and orchestrates acquisition and release.
- `FilesystemInspector`: Thin, platform-specific component that classifies the target mount (ext4, APFS, NTFS, CIFS, etc.) using `statfs` on Unix and `GetVolumeInformationW` on Windows.
- `AdvisoryBackend`: RAII wrapper around `std::fs::File` locking primitives, returning a `LockHandle` that unlocks on drop.
- `AtomicFallback`: Provides atomic staging and rename sequences plus structured warning logs when advisory locks are unavailable or fail repeatedly.
- `LockHygieneRunner`: Scans the lock directory during startup, removes stale temp files, and logs metrics for later aggregation.
- `LockScope`: Enum describing lock intents (`Installation { coordinate: PackageCoordinate }`, `CacheWriter`, `GlobalConfig`), ensuring downstream tasks request the correct granularity.

### Data Flow

1. Caller requests a lock via `LockController::acquire(scope, options)`.
2. `LockController` resolves the on-disk path and asks the `FilesystemInspector` for capability data when needed.
3. If advisory locks are supported, `AdvisoryBackend` creates/open the lock file, calls `lock_exclusive` or `lock_shared`, and returns a `LockHandle` capturing timing metrics.
4. If advisory locks are unsupported or fail after configurable retries, `LockController` switches to `AtomicFallback`, emitting a single INFO-level warning and returning a `FallbackHandle` that coordinates rename-based exclusivity.
5. On drop or explicit `release`, the handle performs unlock/cleanup and records duration in debug logs.
6. At process startup, `LockHygieneRunner::sweep()` removes orphaned lock files left by non-advisory fallbacks and records the number of cleaned artifacts for telemetry.

### Storage Layout and Paths

- Lock directory root: `~/.kopi/locks/`
- Installation lock files: `~/.kopi/locks/install/<distribution>/<package_slug>.lock` where `package_slug` encodes JDK vs JRE, JavaFX inclusion, architecture, and other differentiators copied from foojay metadata
- Cache writer lock file: `~/.kopi/locks/cache.lock` (single global writer lock)

### CLI/API Design

No new CLI flags are introduced in this task. This task introduces configuration keys `locking.mode` and `locking.timeout`, exposing them through the controller so downstream work can thread CLI overrides without redesigning the foundation. Default values (`auto`, `600s`) ship with this implementation.

### Data Models and Types

- `LockController`: Struct containing `locks_root: PathBuf`, `inspector: Arc<dyn FilesystemInspector>`,
- `LockScope`: Enum with variants `Install { coordinate: PackageCoordinate }`, `CacheWriter`, and `GlobalConfig` (future-proofed).
- `LockHandle`: Struct owning the open `File` plus metadata (start time, `PackageCoordinate` when applicable, scope, backend). Implements `Drop` to ensure unlock and metrics emission.
- `FallbackHandle`: Struct capturing fallback state (temp path, destination) to clean up staging artifacts on drop.
- `FilesystemKind`: Enum describing filesystem categories, enriched with `supports_advisory: bool` and `requires_warning: bool` flags.
- `PackageCoordinate`: Struct capturing distribution, version, package type (JDK/JRE), architecture, JavaFX flag, and variant metadata used to generate lock slugs.

### Error Handling

- All public APIs return `Result<LockHandle>` or `Result<FallbackHandle>` using `KopiError::Locking` variants.
- Errors include actionable messages such as `"Failed to acquire cache lock within 600s; last error: ..."`.
- When downgrading to fallback, we emit INFO-level logs and return a handle with `mode: LockMode::Fallback` so callers can surface contextual user messages if desired.

### Security Considerations

- Lock files are created with owner-only permissions (`0o600` or Windows equivalent) to avoid leaking diagnostics.
- No PID or user data is stored in lock files; telemetry metrics stay local.
- Hygiene routine respects symlink boundaries by resolving the locks root before scanning.

### Performance Considerations

- Filesystem detection results are compared against previously mapped mounts within the current process to avoid redundant syscalls whenever practical.
- `LockController` supports `try_acquire` for low-latency paths, deferring to blocking acquire only when needed (enables FR-gbsz6 later).
- Fallback rename sequences reuse existing staging directories to minimize disk churn.

### Platform Considerations

#### Unix

- Use `statfs` via `nix` crate (already a dependency) to inspect `f_type` and map to known network filesystems.
- Handle `EINTR` by retrying lock attempts within the configured timeout budget.

#### Windows

- Use `GetVolumeInformationW` to retrieve filesystem name and `GetDriveTypeW` to detect network drives.
- Ensure handles are opened with `FILE_SHARE_READ | FILE_SHARE_WRITE` before locking to avoid unintended denial of access.

#### Filesystem

- Maintain allowlist of advisory-friendly filesystems (ext4, xfs, btrfs, APFS, NTFS) and denylist (FAT variants, CIFS, SMB, NFS).
- When filesystem is unknown, attempt advisory lock once; on failure, mark mount as fallback-required for the remainder of the process.

## Alternatives Considered

1. Use the `fs4` crate for cross-platform locking instead of the standard library.
   - Pros: Mature abstraction, historical precedent in other projects.
   - Cons: Adds external dependency contrary to ADR-8mnaz, less control over fallback hooks.
2. Rely solely on PID-based lock files with manual cleanup.
   - Pros: Works on network filesystems without special casing.
   - Cons: Requires robust stale detection, complicates crash recovery, and violates cleanup reliability requirements.

Decision Rationale

The chosen design aligns with ADR-8mnaz by preferring native advisory locks while adding deterministic fallbacks and hygiene that meet cleanup and parity requirements without introducing new dependencies.

## Migration and Compatibility

- Existing features will adopt the new API in subsequent tasks (`T-5msmf`, `T-98zsb`, `T-m13bb`).
- No breaking CLI changes; internal modules will gradually migrate off ad-hoc file checks.
- Hygiene runner executes during application bootstrap and logs summary metrics at DEBUG level.
- Execution phases and milestones are tracked in [`docs/tasks/T-ec5ew-locking-foundation/plan.md`](./plan.md).

## Testing Strategy

### Unit Tests

- `filesystem::tests::detect_fs_kind` ensures inspector classifications for mocked `statfs`/Win32 results.
- `locking::tests::acquire_release_drops` verifies RAII unlock semantics and error propagation.
- `fallback::tests::rename_flow` validates staging cleanup on drop.

### Integration Tests

- Add `tests/locking_lifecycle.rs` exercising install/cache scopes with blocking acquire, contention, and timeout transitions.
- Provide platform-gated tests (feature `locking_win`) covering Windows-specific inspector branches.

### Performance & Benchmarks

- Record lock acquisition latency under contention using `cargo bench --bench locking_contention` (optional but recommended prior to GA).

## Documentation Impact

- Update `docs/architecture.md` to include the locking subsystem and directories.
- Draft developer runbook entry in `docs/error_handling.md` for new `KopiError::Locking` variants.
- Coordinate with the documentation site (`../kopi-vm.github.io/`) once user-facing flags surface in downstream tasks.

## External References

- [Rust std::fs::File locking API](https://doc.rust-lang.org/std/fs/struct.File.html) – Advisory lock primitives used by the subsystem.
- [Cargo lock implementation](https://github.com/rust-lang/cargo/blob/master/src/cargo/util/flock.rs) – Reference for network filesystem handling and fallbacks.

## Open Questions

- None at this time; revisit after initial prototype if CI uncovers unsupported filesystems.

## Appendix

### Example Sequence (Install Lock)

```text
install command
  └─> LockController::acquire(Install { temurin, 21 })
         ├─> FilesystemInspector::classify("~/.kopi/locks/install")
         ├─> AdvisoryBackend::lock("install/temurin/temurin-21-jdk-x64.lock")
         ├─> LockHandle stored in Install workflow
         └─> Drop releases lock and logs duration
```

### Hygiene Sweep Algorithm

```text
for each entry in ~/.kopi/locks:
  if entry has .fallback marker and age > configured threshold:
    remove temp + marker files
  record removals in metrics summary
```
