# T-ec5ew Cross-Platform Locking Foundation Task

## Metadata

- Type: Task
- Status: Complete
  <!-- Draft: Under discussion | In Progress: Actively working | Complete: Code complete | Cancelled: Work intentionally halted -->

## Links

- Related Analyses:
  - [AN-m9efc-concurrent-process-locking](../../analysis/AN-m9efc-concurrent-process-locking.md)
- Related Requirements:
  - [FR-02uqo-installation-locking](../../requirements/FR-02uqo-installation-locking.md)
  - [NFR-g12ex-cross-platform-compatibility](../../requirements/NFR-g12ex-cross-platform-compatibility.md)
  - [NFR-vcxp8-lock-cleanup-reliability](../../requirements/NFR-vcxp8-lock-cleanup-reliability.md)
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Associated Design Document:
  - [T-ec5ew-locking-foundation-design](./design.md)
- Associated Plan Document:
  - [T-ec5ew-locking-foundation-plan](./plan.md)

## Summary

Establish a reusable lock abstraction that delivers identical behaviour on Linux, macOS, Windows, and WSL, including reliable cleanup and filesystem capability detection, so downstream tasks can rely on a proven locking foundation.

## Scope

- In scope:
  - Implement core lock acquisition and release APIs.
  - Detect filesystem capabilities, falling back to atomic operations when required.
  - Add startup hygiene to clear orphaned artefacts.
  - Document debug logging expectations for maintainers.
- Out of scope:
  - Operation-specific lock wiring for installation, cache, or uninstallation.
  - UI feedback and timeout control features beyond verifying integration points.

## Success Metrics

- Parity matrix: Automated tests demonstrate identical pass/fail outcomes on Linux, macOS, Windows, and WSL for lock lifecycle scenarios.
- Cleanup reliability: 100% automatic lock release across 1000 forced termination simulations on supported filesystems, with hygiene routines clearing artefacts on startup.
- Fallback safety: Network filesystem detection triggers documented atomic fallbacks with warning logs in simulated degraded scenarios.

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../../templates/README.md#task-template-taskmd) in the templates README.
