# Cross-Platform Locking Foundation Task

## Metadata

- Type: Task
- Status: In Progress (started 2025-09-25)

## Links

- Related Analyses:
  - [AN-m9efc-concurrent-process-locking](../../analysis/AN-m9efc-concurrent-process-locking.md)
- Related Requirements:
  - [FR-02uqo-installation-locking](../../requirements/FR-02uqo-installation-locking.md)
  - [FR-ui8x2-uninstallation-locking](../../requirements/FR-ui8x2-uninstallation-locking.md)
  - [FR-v7ql4-cache-locking](../../requirements/FR-v7ql4-cache-locking.md)
  - [FR-gbsz6-lock-timeout-recovery](../../requirements/FR-gbsz6-lock-timeout-recovery.md)
  - [FR-c04js-lock-contention-feedback](../../requirements/FR-c04js-lock-contention-feedback.md)
  - [NFR-g12ex-cross-platform-compatibility](../../requirements/NFR-g12ex-cross-platform-compatibility.md)
  - [NFR-vcxp8-lock-cleanup-reliability](../../requirements/NFR-vcxp8-lock-cleanup-reliability.md)
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Associated Plan Document:
  - [docs/tasks/T-ec5ew-locking-foundation/plan.md](./plan.md)
- Associated Design Document:
  - [docs/tasks/T-ec5ew-locking-foundation/design.md](./design.md)

## Summary

Establish a reusable lock abstraction that delivers identical behavior on Linux, macOS, Windows, and WSL, including reliable cleanup and filesystem capability detection, so downstream tasks can rely on a proven locking foundation.

## Scope

- In scope: Implement core lock acquisition/release APIs; detect filesystem capabilities and fall back to atomic ops; create startup hygiene for orphaned artifacts; document debug logging expectations.
- Out of scope: Operation-specific lock wiring (installation, cache, uninstallation); UI feedback; timeout controls beyond verifying integration points.

## Success Metrics

- `Parity matrix`: Automated test suite demonstrates identical pass/fail outcomes on Linux, macOS, Windows, and WSL runners for lock lifecycle scenarios.
- `Cleanup reliability`: 100% automatic lock release across 1000 forced termination simulations on supported filesystems; orphaned artifacts cleaned within startup hygiene run.
- `Fallback safety`: Network filesystem detection triggers documented atomic fallback with warning logs in every simulated degraded scenario.

## Detailed Plan

- Design a lock abstraction module that wraps `std::fs::File` locking and exposes a cross-platform trait for acquire/release semantics plus diagnostic hooks.
- Implement filesystem capability detection (ext4, APFS, NTFS, WSL ext4, NFS, SMB) and surface fallback decisions through structured logging.
- Add startup hygiene routine that scans lock directories, removes orphaned temp files, and records metrics for later monitoring.
- Author platform-spanning integration tests and stress harnesses that validate cleanup behavior under forced termination and crash simulations.
- Update documentation references (internal developer docs) to describe the new lock foundation and fallback rules.

## Notes

- Coordinate CI matrix coverage early to guarantee all target platforms run the new lifecycle tests.
- Expose debug logging in a way that downstream tasks can reuse without redefining verbosity controls.

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../../templates/README.md#task-template-taskmd) in the templates README.
