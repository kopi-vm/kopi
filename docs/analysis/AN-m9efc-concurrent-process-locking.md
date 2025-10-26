# AN-m9efc Concurrent Process Locking Analysis

## Metadata

- Type: Analysis
- Status: Complete
  <!-- Draft: Initial exploration | Complete: Ready for requirements | Cancelled: Work intentionally halted | Archived: Analysis concluded -->

## Links

- Related Analyses:
  - N/A – Standalone analysis
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
  - [ADR-8mnaz-concurrent-process-locking-strategy](../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Related Tasks:
  - [T-5msmf-installation-locking](../tasks/T-5msmf-installation-locking/README.md)
  - [T-ec5ew-locking-foundation](../tasks/T-ec5ew-locking-foundation/README.md)
  - [T-98zsb-uninstallation-locking](../tasks/T-98zsb-uninstallation-locking/README.md)
  - [T-s2g7h-active-use-detection](../tasks/T-s2g7h-active-use-detection/README.md)
  - [T-m13bb-cache-locking](../tasks/T-m13bb-cache-locking/README.md)
  - [T-lqyk8-lock-timeout-control](../tasks/T-lqyk8-lock-timeout-control/README.md)
  - [T-60h68-lock-feedback](../tasks/T-60h68-lock-feedback/README.md)

## Executive Summary

Multiple Kopi commands can run concurrently without coordination, creating race conditions during installs, uninstalls, cache refreshes, and configuration updates. This analysis sizes the problem, compares industry approaches, and recommends adopting Rust 1.89.0 standard library locks supplemented with atomic staging-and-rename flows for network filesystems. The findings directly informed ADR-8mnaz and the subsequent requirements and tasks listed above.

## Problem Space

### Current State

- No cross-process coordination in Kopi; only intra-process mutexes exist.
- Critical sections (install/uninstall, cache refresh, shim updates) can overlap.
- Race conditions risk corrupted installations, cache inconsistency, and stale metadata.
- Users receive no feedback when concurrent operations interfere.

### Desired State

- Cross-process locking enforces mutual exclusion for critical operations.
- Operations either wait with clear user feedback or abort gracefully with timeouts.
- Network filesystems (NFS/CIFS) fall back to safe atomic flows when advisory locks are unreliable.
- Lock lifecycle management avoids stale artefacts after crashes.

### Gap Analysis

- Missing: Lock acquisition and release strategy for each operation.
- Missing: Timeout policy, user messaging, and telemetry.
- Missing: Cleanup logic for stale locks on startup.
- Missing: Documentation and requirements linking behaviour across subsystems.

## Stakeholder Analysis

| Stakeholder          | Interest/Need                              | Impact | Priority |
| -------------------- | ------------------------------------------ | ------ | -------- |
| End Users            | Reliable JDK management without corruption | High   | P0       |
| CI/CD Systems        | Parallel builds using Kopi                 | High   | P0       |
| Development Teams    | Multi-terminal workflows                   | Medium | P1       |
| Support & Operations | Diagnosable contention scenarios           | Medium | P1       |

## Research & Discovery

### User Feedback

Bug reports highlight intermittent installation failures and corrupted directories when two terminals run Kopi commands simultaneously. Users request clearer messaging when operations block or fail due to contention.

### Competitive Analysis

- **Volta (Rust)**: Uses kernel-managed locks with atomic staging; provides reference implementation for RAII guard pattern.
- **Rustup/Cargo (Rust)**: Fallback to atomic operations on NFS and emit informative “blocking waiting for file lock” messages.
- **Pyenv (Shell)**: Uses temporary shim files with `noclobber` for mutual exclusion; demonstrates portable approaches but lacks automatic cleanup.
- **nvm/sdkman**: No locking, exposing users to corruption risk; underscores Kopi's need for a stronger guarantee.

### Technical Investigation

- Rust 1.89.0 introduces stable `std::fs::File::lock/try_lock` APIs across Unix and Windows.
- Advisory locks are released automatically when processes exit, eliminating stale lock files in most scenarios.
- NFS and some SMB mounts ignore or mis-handle locks; atomic staging plus rename avoids corruption in those environments.
- Telemetry hooks can record wait times and fallback usage to validate success metrics.

### Data Analysis

Internal telemetry shows spikes in installation failures when users run Kopi concurrently during major JDK updates. Recovery often requires manual directory cleanup, indicating high impact.

## Discovered Requirements

### Functional Requirements (Potential)

- [x] **FR-DRAFT-1**: Provide exclusive locks for installation operations. → [FR-02uqo](../requirements/FR-02uqo-installation-locking.md)
- [x] **FR-DRAFT-2**: Extend locking to uninstall and cache operations. → [FR-ui8x2](../requirements/FR-ui8x2-uninstallation-locking.md), [FR-v7ql4](../requirements/FR-v7ql4-cache-locking.md)
- [x] **FR-DRAFT-3**: Surface user feedback and telemetry for lock contention. → [FR-c04js](../requirements/FR-c04js-lock-contention-feedback.md), [FR-gbsz6](../requirements/FR-gbsz6-lock-timeout-recovery.md)

### Non-Functional Requirements (Potential)

- [x] **NFR-DRAFT-1**: Guarantee lock acquisition latency targets and cleanup reliability. → [NFR-z6kan](../requirements/NFR-z6kan-lock-timeout-performance.md), [NFR-vcxp8](../requirements/NFR-vcxp8-lock-cleanup-reliability.md)
- [x] **NFR-DRAFT-2**: Maintain cross-platform compatibility, including WSL and network volumes. → [NFR-g12ex](../requirements/NFR-g12ex-cross-platform-compatibility.md)

## Design Considerations

### Technical Constraints

- Advisory locks fail silently on some network filesystems.
- Windows requires `LockFileEx`; Unix relies on `flock` under the hood but semantics differ on WSL1.
- CLI must remain responsive; indefinite blocking is unacceptable for Kopi’s UX goals.

### Potential Approaches

1. **Standard library advisory locks with atomic fallback** _(Recommended)_
   - Pros: No external dependencies; automatic cleanup; aligns with Rust ecosystem best practices.
   - Cons: Requires filesystem detection and fallback logic for NFS/SMB.
   - Effort: Medium
2. **PID-based lock files**
   - Pros: Works on network shares; simple to implement.
   - Cons: Requires manual cleanup and crash detection; risk of stale locks high.
   - Effort: Medium
3. **Hybrid third-party crates (fs2/fs4)**
   - Pros: Built-in helpers.
   - Cons: Adds supply-chain risk; duplicates standard library capabilities.
   - Effort: Low

### Architecture Impact

Resulting decisions are documented in ADR-8mnaz. Locking responsibilities must be centralised so future subsystems reuse consistent primitives and telemetry.

## Risk Assessment

| Risk                                        | Probability | Impact | Mitigation Strategy                                                    |
| ------------------------------------------- | ----------- | ------ | ---------------------------------------------------------------------- |
| Locks hang on network filesystems           | Medium      | High   | Detect network mounts and switch to atomic staging/rename flows.       |
| Users confused by contention delays         | Medium      | Medium | Provide clear CLI messaging and progress indicators during waits.      |
| Crash leaves lingering state despite locks  | Low         | Medium | Depend on kernel cleanup; add startup sweep for legacy lock artefacts. |
| Interoperability issues on WSL or UNC paths | Medium      | Medium | Include platform-specific tests and document known limitations.        |

## Open Questions

- [ ] What timeout values balance responsiveness and avoiding false positives? → Next step: Prototype default and configurable thresholds in FR-gbsz6.
- [ ] Which telemetry sink owns contention metrics? → Next step: Resolve in task T-ec5ew.
- [ ] How should Kopi communicate fallback to atomic mode on NFS? → Next step: Align with documentation team via task T-60h68.

## Recommendations

### Immediate Actions

1. Draft and approve requirements FR-02uqo, FR-ui8x2, FR-v7ql4, FR-gbsz6, and FR-c04js.
2. Create ADR covering the locking strategy (ADR-8mnaz) and ensure stakeholders sign off.

### Next Steps

1. [x] Implement foundational locking primitives governed by ADR-8mnaz.
2. [x] Extend coverage to uninstall, cache, and telemetry workflows via dedicated tasks.
3. [ ] Validate timeout and fallback behaviour across Unix, Windows, WSL, and common network filesystems.
4. [ ] Integrate contention metrics into observability stack and monitor post-launch.

### Out of Scope

- Non-locking concurrency mechanisms (actor models, distributed locks).
- Broader refactors outside installation, uninstall, cache, and shim updates.

## Appendix

### References

- Rust 1.89.0 release notes (file locking stabilisation)
- Cargo `flock.rs` implementation
- Volta locking implementation notes
- pyenv `pyenv-rehash` locking script

### Raw Data

- Inventory of Kopi commands and code paths requiring exclusive access.
- Telemetry snapshots showing contention rates during JDK release peaks.

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../templates/README.md#analysis-template-analysismd) in the templates README.
