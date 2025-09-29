# Concurrent Process Locking Strategy

## Metadata

- Type: ADR
- Status: Accepted
  <!-- Proposed: Under discussion | Accepted: Approved and to be implemented | Rejected: Considered but not approved | Deprecated: No longer recommended | Superseded: Replaced by another ADR -->

## Links

- Related Analyses:
  - [AN-m9efc-concurrent-process-locking](../analysis/AN-m9efc-concurrent-process-locking.md)
- Related Requirements:
  - [FR-02uqo-installation-locking](../requirements/FR-02uqo-installation-locking.md)
  - [FR-ui8x2-uninstallation-locking](../requirements/FR-ui8x2-uninstallation-locking.md)
  - [FR-v7ql4-cache-locking](../requirements/FR-v7ql4-cache-locking.md)
  - [FR-gbsz6-lock-timeout-recovery](../requirements/FR-gbsz6-lock-timeout-recovery.md)
  - [FR-c04js-lock-contention-feedback](../requirements/FR-c04js-lock-contention-feedback.md)
  - [NFR-z6kan-lock-timeout-performance](../requirements/NFR-z6kan-lock-timeout-performance.md)
  - [NFR-vcxp8-lock-cleanup-reliability](../requirements/NFR-vcxp8-lock-cleanup-reliability.md)
  - [NFR-g12ex-cross-platform-compatibility](../requirements/NFR-g12ex-cross-platform-compatibility.md)
- Related ADRs:
  - N/A – First ADR covering cross-process locking for Kopi
- Related Tasks:
  - [T-9r1su-fs2-dependency-retirement](../tasks/T-9r1su-fs2-dependency-retirement/README.md)

## Context

- **Problem**: Multiple Kopi processes may run concurrently without coordination, risking race conditions during JDK install/uninstall, cache corruption, and configuration conflicts.
- **Constraints and assumptions**: Works on Unix and Windows without elevated permissions, avoids persistent state after crashes, and tolerates execution on network filesystems (NFS/SMB) even when advisory locking is unreliable.
- **Forces in tension**: Simplicity versus exhaustive edge case coverage, perceived perfect safety versus practical safety, and current needs versus speculative future requirements.
- **Pain points**: Legacy reliance on third-party crates such as `fs2`/`fs4`, stale lock files created by PID-based approaches, and additional maintenance for hybrid lock strategies.
- **Prior art**: Rust 1.89.0 stabilised native `std::fs::File` advisory locking; cargo and volta already rely on kernel-managed locks combined with atomic staging-and-rename operations to guarantee integrity.

## Success Metrics (optional)

- Metric 1: Zero confirmed data corruption incidents attributed to concurrent Kopi processes by 2026-03-01.
- Metric 2: Lock acquisition latency below 1 second (p95) on local filesystems during automated telemetry sampling.
- Metric 3: Crash recovery leaves no stale lock artefacts in `~/.kopi/locks` across supported platforms.
- Metric 4: Fewer than five support tickets about NFS contention within six months of release.

## Decision

We will use the Rust standard library file locking API (`std::fs::File`) for cross-process coordination on local filesystems and fall back to atomic staging-and-rename flows on network filesystems where advisory locks are unreliable.

### Decision Drivers (optional)

- Maintain supply chain hygiene by eliminating the unmaintained `fs2` dependency and its forks.
- Provide cross-platform behaviour with minimal divergence between Unix, Windows, and WSL.
- Preserve JDK directory integrity through kernel-managed cleanup and atomic filesystem operations.

### Considered Options (optional)

- Option A: Native `std::fs::File` locks with atomic operations (selected).
- Option B: Hybrid strategy using `fs4` for network filesystems and std locks elsewhere.
- Option C: PID-based lock files with manual stale detection.

### Option Analysis (optional)

- Option A — Pros: Zero third-party dependency, automatic crash recovery, consistent API | Cons: Requires filesystem detection to handle NFS gracefully.
- Option B — Pros: Familiar crate usage, optional higher-level helpers | Cons: Reintroduces dependency risk, inconsistent behaviour when crate coverage lags platforms.
- Option C — Pros: Works even when advisory locks unavailable | Cons: Requires complex stale PID handling, risks data loss if cleanup fails.

## Rationale

Rust 1.89.0 delivers a stable, cross-platform locking API that matches Kopi's needs without external crates. Using kernel-managed advisory locks allows automatic cleanup after crashes and reduces the maintenance burden. Atomic staging-and-rename ensures correctness even when locks are bypassed (such as on NFS). Hybrid or PID-based approaches add complexity without clear benefits for Kopi's current user base, where local filesystem usage dominates and telemetry shows minimal NFS reliance.

## Consequences

### Positive

- Eliminates dependency on unmaintained locking crates.
- Provides deterministic crash recovery with kernel-managed locks.
- Simplifies operations by reusing established cargo-style patterns.

### Negative

- Requires filesystem detection logic and user messaging on network shares.
- Leaves NFS scenarios to rely solely on atomic operations, which may have longer recovery paths under contention.

### Neutral (if useful)

- Future enhancements (heartbeat, lease monitoring) remain optional follow-ups if telemetry justifies them.

## Implementation Notes (optional)

- Implement locking via `std::fs::File::lock`/`lock_shared`/`unlock` with feature gating on Rust 1.89.0+.
- Detect network filesystems using `/proc/mounts` (Linux), `statfs/statvfs` (Unix variants), and UNC path checks on Windows; downgrade to atomic flows with INFO-level warnings.
- Stage JDK operations under `~/.kopi/jdks/.staging/<id>-<token>` and perform integrity verification before atomic rename to the final directory.
- Use temporary files plus `fsync` and atomic rename for configuration and cache writes.
- Emit consistent telemetry for lock acquisition, contention, and fallback events for observability.

## Examples (optional)

```rust
use std::fs::File;
use std::path::Path;
use std::io::Result;

fn with_lock<F>(lock_path: &Path, f: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    let file = File::create(lock_path)?;
    file.lock()?; // Exclusive lock
    let result = f();
    file.unlock()?;
    result
}
```

## Platform Considerations (required if applicable)

- **Unix/Linux**: Uses `flock(2)` advisory locks; network filesystems are detected via mount inspection and fallback to atomic operations when necessary.
- **Windows**: Relies on `LockFileEx`; UNC and mapped network drives trigger fallback logic and user-facing warnings.
- **WSL**: WSL1 shares host filesystem semantics where `fcntl` locks may fail; WSL2 aligns with Linux behaviour.
- **Network filesystems**: When detection indicates NFS/CIFS, Kopi records the downgrade, skips advisory locking, and continues with staging + rename to avoid blocking behaviour.

## Security & Privacy (required if applicable)

- Lock files are empty placeholders with owner-only permissions, exposing no sensitive data.
- No additional telemetry beyond lock contention durations is collected; logging avoids leaking filesystem paths unnecessarily.

## Monitoring & Logging (required if applicable)

- Log lock acquisition and release at DEBUG level, including wait durations.
- Log contention warnings at INFO level and include fallback reason codes.
- Record fallback-to-atomic events for later analysis and potential ADR follow-up.

## Open Questions

- When should Phase 2 heartbeat/lease features be introduced? → Product Team → Evaluate after six months of production telemetry (2026-03-01).
- How should long-running NFS operations be surfaced in UX copy? → UX Review → Provide wording before Kopi v1.0 release.
- Which telemetry sink owns detailed contention metrics? → Observability Working Group → Align during Q4 2025 planning.

## External References (optional)

- [Rust std::fs::File documentation](https://doc.rust-lang.org/std/fs/struct.File.html) - Native file locking API (stable since 1.89.0)
- [Rust 1.89.0 Release Notes](https://blog.rust-lang.org/2025/08/07/Rust-1.89.0/index.html) - File locking stabilisation announcement
- [flock(2) man page](https://man7.org/linux/man-pages/man2/flock.2.html) - Unix advisory locking semantics
- [Volta sync implementation](https://volta-cli.github.io/volta/main/volta_core/sync/index.html) - Prior art for staging and locking behaviour
- [Cargo flock.rs](https://github.com/rust-lang/cargo/blob/master/src/cargo/util/flock.rs) - Handling of NFS detection and fallbacks

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../templates/README.md#adr-templates-adrmd-and-adr-litemd) in the templates README.

- Tasks:
  - [T-9r1su-fs2-dependency-retirement](../tasks/T-9r1su-fs2-dependency-retirement/README.md)

<!--lint enable remark-validate-links -->
