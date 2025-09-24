# Lock timeout and recovery mechanism

## Metadata

- Type: Functional Requirement
- Status: Accepted
  <!-- Proposed: Under discussion | Accepted: Approved for implementation | Implemented: Code complete | Verified: Tests passing | Deprecated: No longer applicable -->

## Links

- Related Analyses:
  - [AN-m9efc-concurrent-process-locking](../analysis/AN-m9efc-concurrent-process-locking.md)
- Prerequisite Requirements:
  - N/A – No prerequisites
- Dependent Requirements:
  - [FR-02uqo-installation-locking](../requirements/FR-02uqo-installation-locking.md)
  - [FR-ui8x2-uninstallation-locking](../requirements/FR-ui8x2-uninstallation-locking.md)
  - [FR-v7ql4-cache-locking](../requirements/FR-v7ql4-cache-locking.md)
  - [NFR-z6kan-lock-timeout-performance](../requirements/NFR-z6kan-lock-timeout-performance.md)
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Related Tasks:
  - N/A – Not yet implemented

## Requirement Statement

The system SHALL provide a configurable timeout mechanism for lock acquisition so that operations fail gracefully instead of waiting indefinitely when a lock cannot be obtained.

## Rationale

Timeout support prevents indefinite blocking when locks are never released, keeps system resources from being tied up, gives users a predictable fail-fast path, and simplifies debugging of deadlock scenarios.

## User Story (if applicable)

As a kopi user, I want lock acquisition to respect a configurable timeout and convey clear errors when exceeded, so that I can recover from hung operations without manually terminating processes.

## Acceptance Criteria

- [ ] Lock acquisition waits up to the configured timeout duration before failing when the lock remains unavailable.
- [ ] Timeout configuration precedence follows CLI flag > environment variable > config file > built-in default.
- [ ] Special timeout values support `0` (no wait) and `infinite` (wait indefinitely) semantics.
- [ ] Timeout expiration emits a clear error message that includes the attempted timeout value and instructions for override.
- [ ] Users can interrupt lock waits via `Ctrl-C`, producing a graceful cancellation message.
- [ ] Default timeout values are defined for each operation type and documented (`install`: 30 s, `cache`: 10 s, `uninstall`: 30 s) unless overridden.

## Technical Details (if applicable)

### Functional Requirement Details

- CLI flag: `--lock-timeout <seconds|infinite>`.
- Environment variable: `KOPI_LOCK_TIMEOUT=<seconds|infinite>`.
- Configuration file key: `lock_timeout = <seconds|infinite>`.
- Timeout evaluation uses `std::time::Instant` for monotonic measurements with exponential backoff polling (10 ms → 20 ms → ... → 1 s cap).
- Timeout handling integrates with UI feedback from FR-c04js to surface remaining time and overrides.

### Non-Functional Requirement Details

N/A – Not applicable.

## Platform Considerations

### Unix

- Implement polling using blocking lock attempts with sleep intervals; handle `EINTR` gracefully when `Ctrl-C` occurs.

### Windows

- Use Windows lock APIs with wait-timeout emulation; monitor console control events for cancellations.

### Cross-Platform

- Ensure identical timeout semantics across platforms and normalize error messaging for parity.

## Risks & Mitigation

| Risk                               | Impact | Likelihood | Mitigation                                     | Validation                     |
| ---------------------------------- | ------ | ---------- | ---------------------------------------------- | ------------------------------ |
| Timeout too short for slow systems | Medium | Medium     | Ship conservative defaults and allow overrides | Measure on slow hardware       |
| Polling overhead                   | Low    | Medium     | Use exponential backoff and sleep hints        | Benchmark CPU usage            |
| Clock skew affects timeout         | Low    | Low        | Use monotonic clocks only                      | Test with adjusted system time |

## Implementation Notes

- Surface timeout parameters in debug logs for troubleshooting.
- Provide specific exit codes for timeout vs. cancellation to support scripting.
- Integrate with telemetry (if available) to gather timeout frequency data.
- Document timeout behavior in user docs maintained in `../kopi-vm.github.io/`.

## External References

N/A – No external references

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd).
