# ADR-8mnaz Concurrent Process Locking Strategy

## Metadata

- Type: ADR
- Status: Approved
  <!-- Draft: Under discussion | Approved: Ready to be implemented | Rejected: Considered but not approved | Deprecated: No longer recommended | Superseded: Replaced by another ADR -->

## Links

- Impacted Requirements:
  - [FR-02uqo-installation-locking](../requirements/FR-02uqo-installation-locking.md)
  - [FR-ui8x2-uninstallation-locking](../requirements/FR-ui8x2-uninstallation-locking.md)
  - [FR-v7ql4-cache-locking](../requirements/FR-v7ql4-cache-locking.md)
  - [FR-gbsz6-lock-timeout-recovery](../requirements/FR-gbsz6-lock-timeout-recovery.md)
  - [FR-c04js-lock-contention-feedback](../requirements/FR-c04js-lock-contention-feedback.md)
  - [FR-rxelv-file-in-use-detection](../requirements/FR-rxelv-file-in-use-detection.md)
  - [FR-x63pa-disk-space-telemetry](../requirements/FR-x63pa-disk-space-telemetry.md)
  - [NFR-z6kan-lock-timeout-performance](../requirements/NFR-z6kan-lock-timeout-performance.md)
  - [NFR-vcxp8-lock-cleanup-reliability](../requirements/NFR-vcxp8-lock-cleanup-reliability.md)
  - [NFR-g12ex-cross-platform-compatibility](../requirements/NFR-g12ex-cross-platform-compatibility.md)
- Supersedes ADRs:
  - N/A – First ADR covering cross-process locking for Kopi
- Related Tasks:
  - [T-5msmf-installation-locking](../tasks/T-5msmf-installation-locking/README.md)
  - [T-98zsb-uninstallation-locking](../tasks/T-98zsb-uninstallation-locking/README.md)
  - [T-s2g7h-active-use-detection](../tasks/T-s2g7h-active-use-detection/README.md)
  - [T-m13bb-cache-locking](../tasks/T-m13bb-cache-locking/README.md)
  - [T-lqyk8-lock-timeout-control](../tasks/T-lqyk8-lock-timeout-control/README.md)
  - [T-60h68-lock-feedback](../tasks/T-60h68-lock-feedback/README.md)
  - [T-ec5ew-locking-foundation](../tasks/T-ec5ew-locking-foundation/README.md)
  - [T-9r1su-fs2-dependency-retirement](../tasks/T-9r1su-fs2-dependency-retirement/README.md)

## Context

- Kopi currently allows multiple processes to mutate shared state (install, uninstall, cache refresh) without coordination, risking corruption and inconsistent metadata.
- Historic reliance on third-party crates (`fs2`) increased supply-chain risk and produced divergent behaviour across platforms.
- Rust 1.89.0 stabilised cross-platform advisory locking via `std::fs::File`, enabling a first-party solution aligned with Kopi's safety goals.
- Network filesystems (NFS/SMB) and WSL introduce environments where advisory locks may be unreliable or unsupported.
- Kopi must uphold developer experience by providing actionable feedback during contention and avoiding deadlocks.

## Success Metrics (optional)

- Metric 1: Zero confirmed data corruption incidents attributed to concurrent Kopi processes by 2026-03-01.
- Metric 2: Lock acquisition latency p95 stays below 1 second on local filesystems.
- Metric 3: Crash recovery leaves no stale lock artefacts under `~/.kopi/locks` on supported platforms.

## Decision

We will use Rust's standard library advisory file locks (`std::fs::File::lock`, `try_lock`, and `unlock`) as the primary coordination mechanism for Kopi operations. We will pair this with atomic staging-and-rename flows for scenarios where locks are unreliable (notably network filesystems) and emit telemetry plus user-facing messaging when contention occurs.

### Decision Drivers (optional)

- Reduce supply-chain risk by eliminating `fs2`/`fs4`.
- Provide deterministic crash recovery through kernel-managed locks.
- Maintain consistent cross-platform behaviour and telemetry.
- Align with Rust ecosystem best practices (cargo, rustup, Volta).

### Considered Options (optional)

- Option A: Standard library locks with atomic fallback _(selected)_
- Option B: Hybrid approach using third-party locking crates for specific platforms
- Option C: PID-based lock files with manual stale detection

### Option Analysis (optional)

- **Option A** — Pros: Zero external dependency, automatic cleanup, consistent semantics | Cons: Requires filesystem detection and fallback logic.
- **Option B** — Pros: Familiar APIs, potential helpers for network filesystems | Cons: Reintroduces dependency risk and inconsistent behaviours when crates lag behind.
- **Option C** — Pros: Works on filesystems without advisory lock support | Cons: Requires complex stale PID management, risks data loss if cleanup fails.

## Rationale

Rust 1.89.0 delivers a stable, cross-platform locking API that meets Kopi's requirements without relying on external crates. Kernel-managed advisory locks automatically release when processes exit, solving historical stale-lock issues. Combining locks with atomic staging-and-rename operations preserves data integrity even when locks cannot be obtained. Third-party crates or PID-based schemes add maintenance complexity without delivering superior guarantees.

## Consequences

### Positive

- Removes dependency on unmaintained locking crates.
- Enables deterministic crash recovery via kernel-managed cleanup.
- Aligns Kopi with established tooling patterns (cargo, volta).

### Negative

- Introduces filesystem detection logic to downgrade gracefully on network shares.
- Requires additional user messaging and telemetry to handle contention cases.

### Neutral (if useful)

- Future enhancements (heartbeat, lease monitoring) remain optional follow-ups.

## Implementation Notes (optional)

- Implement locking via `std::fs::File::lock_exclusive`, `try_lock_exclusive`, and `unlock`, with RAII guards to ensure deterministic release.
- Detect network filesystems using `statfs`/`statvfs` on Unix, UNC heuristics on Windows, and WSL-specific checks; fall back to staging-and-rename flows when reliability is uncertain.
- Place lock files under `$KOPI_HOME/locks/` with owner-only permissions; treat files as zero-length placeholders.
- Emit telemetry capturing acquisition latency, contention outcomes, and fallback reasons.
- Integrate messaging into CLI flows to inform users when operations wait or downgrade.

## Examples (optional)

```rust
use std::fs::File;
use std::path::Path;
use std::io::Result;

fn with_lock<F>(lock_path: &Path, operation: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    let file = File::create(lock_path)?;
    file.lock_exclusive()?;
    let result = operation();
    file.unlock()?;
    result
}
```

## Platform Considerations (required if applicable)

- **Unix/Linux**: Uses `flock(2)` advisory locks under the hood; detect NFS/CIFS via `statfs` and downgrade to atomic staging when necessary. Permissions should restrict lock files to the Kopi user.
- **Windows**: Relies on `LockFileEx`; UNC and mapped network drives trigger fallback flows with user-facing warnings. Ensure handles close promptly to release locks.
- **WSL**: WSL1 inherits host filesystem semantics where locks may be unreliable; treat as network filesystem and fall back when detection indicates host-mounted paths.
- **Network filesystems**: Always provide atomic staging-and-rename fallback and record downgrade telemetry.

## Security & Privacy (required if applicable)

- Lock files contain no sensitive data. Maintain owner-only permissions to avoid leaking path information to other users.
- Logging must avoid exposing full filesystem paths beyond existing diagnostics norms.

## Monitoring & Logging (required if applicable)

- Emit structured telemetry events for `lock_acquired`, `lock_contended`, and `fallback_to_atomic` including timing metrics.
- Log contention warnings at INFO level with concise, actionable guidance (e.g., "Another Kopi command is running; waiting for up to <timeout>").
- Record downgrade events for auditability and follow-up analysis.

## Open Questions

- [ ] What default timeout balances responsiveness with avoiding spurious failures? → Next step: Prototype in FR-gbsz6 and capture telemetry.
- [ ] How should we surface fallback-to-atomic events in the CLI UX? → Next step: Coordinate with task T-60h68.
- [ ] Which observability backend owns long-term storage of contention metrics? → Next step: Resolve in task T-ec5ew.

## External References (optional)

- [Rust std::fs::File documentation](https://doc.rust-lang.org/std/fs/struct.File.html) – Native locking API (stable since 1.89.0)
- [Rust 1.89.0 release notes](https://blog.rust-lang.org/2025/08/07/Rust-1.89.0/index.html) – File locking stabilisation announcement
- [Cargo flock implementation](https://github.com/rust-lang/cargo/blob/master/src/cargo/util/flock.rs)
- [Volta locking design](https://volta-cli.github.io/volta/main/volta_core/sync/index.html)

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../templates/README.md#adr-templates-adrmd-and-adr-litemd) in the templates README.
