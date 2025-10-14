# T-ec5ew Cross-Platform Locking Foundation Task

## Metadata

- Type: Task
- Status: Complete
  <!-- Draft: Under discussion | In Progress: Actively working | Complete: Code complete | Cancelled: Work intentionally halted -->

## Links

- Associated Plan Document:
  - [T-ec5ew-locking-foundation-plan](./plan.md)
- Associated Design Document:
  - [T-ec5ew-locking-foundation-design](./design.md)

## Summary

Establish a reusable lock abstraction that delivers identical behaviour on Linux, macOS, Windows, and WSL, including reliable cleanup and filesystem capability detection, so downstream tasks can rely on a proven locking foundation.

## Scope

- In scope: Implement core lock acquisition/release APIs, detect filesystem capabilities and fall back to atomic operations, create startup hygiene for orphaned artefacts, and document debug logging expectations.
- Out of scope: Operation-specific lock wiring (installation, cache, uninstallation), UI feedback, timeout controls beyond verifying integration points.

## Success Metrics

- Parity matrix: automated tests demonstrate identical pass/fail outcomes on Linux, macOS, Windows, and WSL for lock lifecycle scenarios.
- Cleanup reliability: 100% automatic lock release across 1000 forced termination simulations on supported filesystems;
  hygiene routine clears artefacts on startup.
- Fallback safety: network filesystem detection triggers documented atomic fallback with warning logs in simulated degraded scenarios.

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../../templates/README.md#task-template-taskmd) in the templates README.
