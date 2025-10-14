# T-ec5ew Cross-Platform Locking Foundation Design

## Metadata

- Type: Design
- Status: Approved
  <!-- Draft: Work in progress | Approved: Ready for implementation | Rejected: Not moving forward with this design -->

## Links

- Associated Plan Document:
  - [T-ec5ew-locking-foundation-plan](./plan.md)

## Overview

Introduce a reusable locking subsystem that wraps Rust’s `std::fs::File` advisory locks, classifies filesystem capabilities, and provides deterministic fallbacks and hygiene across Linux, macOS, Windows, and WSL. Downstream tasks (installation, uninstallation, cache) consume this API instead of reimplementing platform logic.

## Success Metrics

- [x] Advisory locks succeed on Linux, macOS, Windows, and WSL with identical semantics in CI.
- [x] Startup hygiene removes all synthetic fallback artefacts across 1,000 crash simulations.
- [x] Network filesystem downgrade emits structured INFO warnings and safely falls back to atomic rename sequences.

## Background and Current State

- Context: Kopi lacks a unified process-level locking API, leading to ad hoc protections or no protection at all.
- Current behaviour: No cross-process coordination exists; limited file-in-use checks rely on the `fs2` crate.
- Pain points: Race conditions during concurrent installs/cache refreshes, no stale lock cleanup, brittle platform detection.
- Constraints: Must rely on standard library locks per ADR-8mnaz, avoid `unsafe`, deliver identical semantics on major OSes.
- Related ADRs: `docs/adr/ADR-8mnaz-concurrent-process-locking-strategy.md`.

## Proposed Design

### High-Level Architecture

```text
Caller --> LockController --> FilesystemInspector
                      |             |
                      v             v
               AdvisoryBackend   AtomicFallback
                      |
                      v
               LockHygieneRunner
```

### Components

- `LockController`: Public API supporting exclusive/shared locks, timeout budgets, and downgrade decisions.
- `FilesystemInspector`: Classifies mounts via `statfs` (Unix) or `GetVolumeInformationW`/`GetDriveTypeW` (Windows); caches results per path.
- `AdvisoryBackend`: RAII wrapper over `std::fs::File` lock primitives with structured logging.
- `AtomicFallback`: Staging + rename sequence with marker files and INFO warnings when advisory locks are unreliable.
- `LockHygieneRunner`: Startup routine that removes stale fallback artefacts and logs metrics.
- `LockScope`: Enum capturing installation coordinates, cache writer scope, or global configuration locks.

### Data Flow

1. Caller requests `LockController::acquire(scope, options)`.
2. Controller resolves lock path via `LockScope` helpers and queries `FilesystemInspector` when needed.
3. If advisory mode supported, `AdvisoryBackend` obtains the lock and returns a `LockHandle` that unlocks on drop.
4. On failure or unsupported filesystem, controller switches to `AtomicFallback`, returning a `FallbackHandle` with the same RAII contract and emitting INFO logs.
5. `LockHygieneRunner` executes once per process start, removing stale fallback artefacts.

### Storage Layout and Paths (if applicable)

- Install locks: `$KOPI_HOME/locks/install/<vendor>/<slug>.lock` where slug derives from `PackageCoordinate` (distribution, version, arch, variant).
- Cache lock: `$KOPI_HOME/locks/cache.lock`.
- Fallback marker files: `$KOPI_HOME/locks/<scope>.fallback` (ignored by advisory mode).

### CLI/API Design (if applicable)

No new CLI flags; configuration gains `locking.mode` (`auto|advisory|fallback`) and `locking.timeout` (duration). Downstream tasks reuse existing error messaging.

### Data Models and Types

- `LockController`, `LockHandle`, `FallbackHandle`, `LockScope`, `LockOptions { timeout, mode }`, `FilesystemKind` enum.
- `LockStatus` / `LockResult` wrappers returning `KopiError::Locking*` variants.

### Error Handling

- Extend `KopiError` with `LockingTimeout`, `LockingUnavailable`, `LockingDowngraded`, each with actionable English messages.
- Attach `ErrorContext` entries detailing scope, filesystem kind, timeout budget, and fallback reason.

### Security Considerations

- Lock files created with owner-only permissions (`0o600` / restrictive ACLs on Windows).
- Hygiene runner avoids deleting non-Kopi files by checking marker naming conventions.
- Logging redacts absolute paths beyond existing verbosity levels.

### Performance Considerations

- Use exponential backoff with 100 ms cap during contention to limit CPU overhead (<0.1% single-core).
- Cache filesystem classification per mount to avoid redundant syscalls.
- Record acquisition durations for telemetry (optional).

### Platform Considerations

#### Unix

- Leverage `statfs` to classify ext4, xfs, btrfs, NFS, CIFS; degrade to fallback for network filesystems.
- Respect symlinked JDK paths by canonicalising before inspection.

#### Windows

- Use `GetVolumeInformationW`/`GetDriveTypeW` to detect NTFS vs network drives; fallback for UNC paths and FAT variants.
- Ensure handles close promptly to release locks.

#### Filesystem

- Maintain allowlist of advisory-friendly filesystems (ext4, APFS, NTFS) and degrade for FAT, CIFS/SMB, NFS.
- When filesystem is unknown, attempt advisory once; on failure, mark mount as fallback-required for the remainder of the process.

## Alternatives Considered

1. Adopt the `fs4` crate.
   - Pros: Existing abstraction.
   - Cons: Adds dependency contrary to ADR-8mnaz; limited control over telemetry and fallbacks.
2. Use PID-based lock files.
   - Pros: Works on network shares.
   - Cons: Requires manual cleanup and robust stale detection; fails NFR-vcxp8.

Decision Rationale

Standard library locks combined with deterministic fallback best satisfy ADR-8mnaz and linked NFRs without growing dependencies.

## Migration and Compatibility

- Downstream tasks (`T-5msmf`, `T-98zsb`, `T-m13bb`) adopt `LockController` in follow-up work.
- Hygiene runner executes during CLI startup; existing commands unchanged until integration tasks land.
- Legacy helpers remain available behind feature flags until dependents migrate.

## Testing Strategy

### Unit Tests

- Filesystem inspector tests using mocked `statfs` / Win32 responses.
- Controller tests covering advisory success, contention, timeout, downgrade, and error propagation.
- Fallback tests validating staging cleanup and marker handling.

### Integration Tests

- `tests/locking_lifecycle.rs` exercises install/cache scopes, contention, timeout, fallback, and hygiene flows.
- Ignored crash simulation harness runs 1,000 forced terminations to validate cleanup reliability.

### External API Parsing (if applicable)

- N/A.

### Performance & Benchmarks (if applicable)

- Optional micro-benchmark capturing acquisition latency under contention; ensure results feed NFR-z6kan reporting.

## Documentation Impact

- Update `docs/architecture.md` and `docs/error_handling.md` with subsystem description and new error variants.
- Provide developer guidance for `LockController` usage in module docs and CODEOWNERS notes.
- Coordinate external docs once user-facing behaviour appears in downstream tasks.

## External References

- [Rust std::fs::File locking API](https://doc.rust-lang.org/std/fs/struct.File.html)
- [Cargo lock implementation](https://github.com/rust-lang/cargo/blob/master/src/cargo/util/flock.rs)

## Open Questions

- [ ] How should we store telemetry for downgrade events (stdout log vs metrics sink)? → Decide during Phase 3 rollout.
- [ ] What is the process for validating additional filesystems (e.g., ZFS) in CI? → Track in follow-up issue.

## Appendix

### Diagrams

```text
Install scope --> LockController --> AdvisoryBackend --success--> LockHandle (Drop releases)
                                      |--downgrade--> AtomicFallback --> FallbackHandle
```

### Examples

```rust
let options = LockOptions::default();
let controller = LockController::new(lock_root);
let handle = controller.acquire(LockScope::install(coord), &options)?;
// perform install
// handle drops -> unlocks automatically
```

### Glossary

- **Lock scope**: Logical grouping describing the resource being protected (install coordinate, cache writer, global config).
- **Fallback**: Atomic staging + rename strategy used when advisory locks are unreliable.

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../../templates/README.md#design-template-designmd) in the templates README.
