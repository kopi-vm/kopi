# T-98zsb Uninstallation Lock Integration Design

## Metadata

- Type: Design
- Status: Draft
  <!-- Draft: Work in progress | Approved: Ready for implementation | Rejected: Not moving forward with this design -->

## Links

- Associated Plan Document:
  - [T-98zsb-uninstallation-locking-plan](./plan.md)

## Overview

Extend the Kopi locking foundation to the entire uninstallation workflow so that every removal of a JDK coordinate acquires an exclusive lock shared with installations. The design introduces a reusable scoped lock guard for installed artifacts, resolves canonical lock scopes from on-disk metadata, and wires lock acquisition plus feedback into single and batch uninstall paths. The result prevents concurrent install/uninstall collisions, preserves atomic rename-and-delete semantics, and keeps timeout handling consistent with ADR-8mnaz.

## Success Metrics

- [ ] Concurrent `kopi install` and `kopi uninstall` operations targeting the same coordinate serialize successfully in automated contention tests without filesystem corruption.
- [ ] Uninstall flows (single and batch) either complete fully or roll back without leaving residual directories while the associated lock is held.
- [ ] Lock telemetry shows matching scope identifiers for install and uninstall acquisitions with no orphaned lock files after forced termination scenarios.

## Background and Current State

- Context: The CLI routes `kopi uninstall` through `UninstallCommand`, `UninstallHandler`, and optionally `BatchUninstaller`; these modules orchestrate safety checks, atomic rename-to-temp, metadata removal, and cleanup.
- Current behavior: Uninstallation performs destructive work without cross-process coordination. `LockController`, timeout policies, and wait observers introduced for installation remain unused, so concurrent installs and uninstalls can interleave directory mutations.
- Pain points: Simultaneous uninstall/install can delete or recreate the same JDK tree, causing partial removals, orphaned metadata, or shim inconsistencies. Users receive no feedback when contention exists and cannot rely on atomicity guarantees.
- Constraints: Reuse ADR-8mnaz locking primitives (advisory with fallback) and the existing timeout resolver. Preserve English-only messaging, avoid new third-party crates, and respect platform abstractions in `platform::file_ops` and `paths::locking`.
- Related ADRs: [`docs/adr/ADR-8mnaz-concurrent-process-locking-strategy.md`](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)

## Proposed Design

### High-Level Architecture

```text
kopi uninstall CLI
        │
        ▼
UninstallCommand
        │
        ▼
UninstallHandler::uninstall_jdk / BatchUninstaller
        │
        ├─ InstalledScopeResolver
        │       └─ Reads JdkMetadataWithInstallation + fallback heuristics
        │
        ├─ ScopedPackageLockGuard
        │       └─ LockController::acquire_with_status_sink → StatusReporterObserver
        │
        └─ Removal pipeline (safety checks → atomic rename → metadata cleanup → post-check)
```

### Components

- **ScopedPackageLockGuard** (new module replacing the installation-only guard):
  - Consolidates the RAII guard currently implemented in `locking/installation.rs` into a reusable `ScopedPackageLockGuard<'a>` that holds a `LockController` reference and `LockAcquisition`.
  - Exposes `backend()` and `scope_label()` for logging/telemetry and retains `release()` for explicit error surfacing; drop still logs warnings.
  - Existing installation flow migrates to the new guard with minimal call-site changes.
- **InstalledScopeResolver** (new helper in `locking/uninstallation.rs`):
  - Accepts an `InstalledJdk`, `KopiConfig`, and `JdkRepository` access to produce a `LockScope::installation`.
  - Primary path reads `~/.kopi/jdks/<slug>.meta.json`, strictly parses `JdkMetadataWithInstallation` (no repair or recovery attempts), and reuses `PackageCoordinate::try_from_package`, appending `distribution_version` to variant tags so scope identifiers match installation.
  - Fallback path activates when metadata is missing or parsing fails: derives coordinate from `InstalledJdk` fields, parses `InstallationMetadata::platform` (`<os>_<arch>`), infers libc (if available), and always adds the sanitized directory slug so installs and uninstalls converge on the same lock even for legacy installs.
  - Emits structured warnings when falling back and returns `KopiError::LockingScopeUnavailable` only if neither path can guarantee a unique coordinate; corrupted metadata is treated as absent with no auto-repair attempt.
- **UninstallHandler updates**:
  - Instantiate `LockController::with_default_inspector` using `KopiConfig::kopi_home()` and `config.locking` (handler keeps an owned controller or shared `Arc`).
  - Before safety checks run, compute the lock scope for the selected `InstalledJdk`, acquire the scoped guard via `controller.acquire_with_status_sink`, and suspend status output through the existing `StatusReporter` so contention feedback mirrors installation UX.
  - Guard lifetime covers safety checks, atomic rename (`prepare_atomic_removal`), final delete, rollback, shim cleanup, and metadata removal to guarantee atomicity.
  - On successful completion explicitly call `release()`; error paths rely on Drop to release while logging.
- **BatchUninstaller updates**:
  - For each candidate `InstalledJdk`, resolve the lock scope and acquire a guard before invoking repository removal.
  - Progress bars (`indicatif`) continue to render; wait messages appear via `StatusReporter` tied to the batch reporter and gated behind `progress_reporter.suspend`.
  - If acquisition fails or times out, mark that JDK as failed with actionable messaging and proceed to the next item without attempting destructive work.
- **Cleanup & recovery alignment**:
  - `UninstallCleanup::execute_cleanup` reuses per-coordinate locks only when the resolver can identify a valid scope; `force_cleanup_jdk` itself remains intentionally lock-free so emergency cleanup never relies on a separate maintenance lock.
- **Telemetry & logging**:
  - Log backend (`advisory` vs `fallback`) at INFO for uninstall similar to install.
  - Forward timeout/cancellation errors through existing `KopiError` variants so CLI exits stay consistent.
  - No additional telemetry collection is required for this task.

### Data Flow

1. `UninstallCommand` resolves target `InstalledJdk`(s) and constructs `UninstallHandler` with a lock-aware configuration.
2. `InstalledScopeResolver` reads metadata (or falls back) to create `LockScope::installation` sharing the canonical slug with installs.
3. `LockController::acquire_with_status_sink` obtains the lock, surfacing wait/timeout events through `StatusReporter`.
4. Under the guard, safety checks validate default usage, platform processes, and file-in-use detection.
5. Removal proceeds via atomic rename → repository delete → shim/metadata cleanup; rollback logic executes on failure before the guard drops.
6. Batch and cleanup workflows repeat steps 2–5 per coordinate, aggregating successes/failures for progress reporting.

### Storage Layout and Paths

- Locks continue to reside under `~/.kopi/locks/install/<distribution>/<slug>.lock`, using `PackageCoordinate::slug()` to keep scope identifiers consistent.
- Metadata lookup relies on existing `~/.kopi/jdks/<slug>.meta.json` files created during install; fallback consumes directory names such as `temurin-21.0.5+11[-fx]`.

### CLI/API Design

- No new CLI flags. Existing `--lock-timeout` override (handled globally by `LockingConfig`) applies automatically to uninstall flows.
- Wait messaging remains in English and reuses the same `StatusReporter` format introduced for installation locking.

### Data Models and Types

- `ScopedPackageLockGuard<'a>` (new) replaces the installation-specific guard while preserving API surface.
- `InstalledScopeResolver` returns `Result<(LockScope, PackageCoordinateMetadata)>`, enabling callers to log coordinate context; it internally leverages `PackageCoordinate` and `PackageKind::try_from_str`.
- Extended `JdkRepository` API exposes read-only access to `KopiConfig` or a dedicated helper `load_installed_metadata(&InstalledJdk)` returning `Option<JdkMetadataWithInstallation>`.

### Error Handling

- Lock acquisition failures surface via existing `KopiError::LockingTimeout` and `KopiError::LockingAcquire` variants, triggering guarded cleanup (rollback rename) before propagating.
- Missing metadata triggers a warning and fallback slug; total failure to derive a scope aborts uninstall with a descriptive `KopiError::LockingScopeUnavailable` instructing the user to run cleanup.
- Drop-based release logs warnings (no panics) if releasing the lock fails; cleanup paths swallow but report issues via `StatusReporter::error`.

### Security Considerations

- Scope resolution validates that metadata paths reside under `KopiConfig::jdks_dir()` to prevent directory traversal.
- Repository removal continues to enforce security checks (`remove_jdk` refuses paths outside the JDK root).
- Lock files retain owner-only permissions (`0o600` on Unix) as enforced by `LockController`.

### Performance Considerations

- Metadata read is cached per JDK and happens once per uninstall; files are small JSON payloads.
- Holding the lock wraps only the destructive phase; downloads or heavy IO are not part of uninstall, so contention windows remain small.
- Batch uninstall serializes per coordinate but still streams progress; no additional threads or blocking beyond existing timeout/backoff.

### Platform Considerations

#### Unix

- Advisory locks rely on `flock`; fallback triggers on NFS/SMB detection via the `FilesystemInspector` already in use.
- Directory rename/remove semantics remain unchanged; additional logging clarifies when fallback is active.

#### Windows

- Uses `LockFileEx`; fallback handles UNC and FAT volumes. Guard ensures retry logic (`platform::file_ops::prepare_for_removal`) executes under lock to avoid double deletes.

#### Filesystem

- Scope resolver strips path separators and normalises variant tags to avoid mismatched lock filenames.
- Temporary `.removing` directories inherit the lock of their source coordinate so cleanup tasks cannot race with reinstalls.

## Alternatives Considered

1. **Lock only around final directory deletion**
   - Pros: Shorter lock hold duration.
   - Cons: Safety checks, metadata removal, and rollback could still interleave with installations, undermining atomicity guarantees.
2. **Introduce a global uninstall mutex**
   - Pros: Simplifies coordination logic.
   - Cons: Over-serialises unrelated coordinates, harming throughput; diverges from ADR-8mnaz which mandates per-coordinate scoping.

**Decision Rationale**: Reusing per-coordinate locks preserves install/uninstall symmetry, minimises contention to the affected coordinate, and leverages the already approved fallback pipeline.

## Migration and Compatibility

- Existing installations gain contention protection automatically; no directory layout changes occur.
- Legacy installs missing metadata enter the fallback slug path but still receive locking, maintaining backward compatibility.
- No feature flags required; rollout occurs when the implementation lands and documentation is updated.

## Testing Strategy

### Unit Tests

- Cover `InstalledScopeResolver` metadata parsing, fallback derivation from `InstallationMetadata::platform`, and sanitisation of directory slugs.
- Verify corrupted metadata triggers the fallback path without attempting file repair or mutation.
- Exercise the guard’s explicit `release()` path and drop logging via mocked `LockController` (similar to existing installation guard tests).
- Validate cleanup locking chooses the correct scope for `.removing` directories.

### Integration Tests

- Add `tests/uninstall_locking.rs` orchestrating two parallel uninstalls and an install+uninstall pair, asserting serialization and absence of partial directories.
- Extend batch uninstall tests to verify that contention on one coordinate yields a timeout while others proceed.
- Simulate metadata corruption to ensure fallback locking still protects uninstall and emits warnings.

### External API Parsing

- Not applicable; no new Foojay parsing introduced.

### Performance & Benchmarks

- Track uninstall wall-clock time with and without lock contention in CI smoke tests; no dedicated benchmarks required because uninstall is IO-bound and short-lived.

## Documentation Impact

- Update `docs/architecture.md` to reflect that uninstall now participates in the locking subsystem.
- Amend task README once the design is approved to mark Status → In Progress.
- Coordinate with external user docs (`../kopi-vm.github.io/`) to mention lock wait messaging for uninstall commands.

## External References

- [Rust std::fs::File locking API](https://doc.rust-lang.org/std/fs/struct.File.html) – Reference for advisory lock behaviour across platforms.

## Open Questions

- [x] Should we extend locking to `UninstallCleanup::force_cleanup_jdk` for orphaned directories that lack metadata, or treat it separately via a global maintenance lock? → Decision: Do not acquire any additional lock; `force_cleanup_jdk` remains intentionally lock-free to keep emergency cleanup simple.
- [x] Do we need to expose lock backend/coordinate in telemetry for analytics parity with install? → Decision: No extra telemetry work; rely on existing logging only.
- [x] How aggressively should we retry metadata parsing before falling back (e.g., auto-heal corrupt JSON)? → Decision: No retries—treat any parse failure as a signal to use the fallback path to keep the implementation simple.

## Appendix

### Examples

```bash
# Concurrent operations targeting the same coordinate
kopi install temurin@21 &
kopi uninstall temurin@21 &
# Output includes:
#   Waiting for installation temurin-21-jdk-x64 lock (timeout 600s, source config)
#   Lock contention detected for installation temurin-21-jdk-x64…
```

### Glossary

- **ScopedPackageLockGuard**: RAII wrapper around a lock acquisition for a package coordinate.
- **InstalledScopeResolver**: Helper translating on-disk installation metadata into a `LockScope` shared with the installation pipeline.

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../../templates/README.md#design-template-designmd) in the templates README.
