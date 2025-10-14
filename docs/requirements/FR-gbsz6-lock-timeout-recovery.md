# FR-gbsz6 Lock Timeout and Recovery

## Metadata

- Type: Functional Requirement
- Status: Approved
  <!-- Draft: Under discussion | Approved: Ready for implementation | Rejected: Decision made not to pursue this requirement -->

## Links

- Prerequisite Requirements:
  - [FR-02uqo-installation-locking](../requirements/FR-02uqo-installation-locking.md)
- Dependent Requirements:
  - [FR-ui8x2-uninstallation-locking](../requirements/FR-ui8x2-uninstallation-locking.md)
  - [FR-v7ql4-cache-locking](../requirements/FR-v7ql4-cache-locking.md)
  - [FR-c04js-lock-contention-feedback](../requirements/FR-c04js-lock-contention-feedback.md)
  - [NFR-z6kan-lock-timeout-performance](../requirements/NFR-z6kan-lock-timeout-performance.md)
- Related Tasks:
  - [T-lqyk8-lock-timeout-control](../tasks/T-lqyk8-lock-timeout-control/README.md)

## Requirement Statement

The system SHALL provide a configurable timeout mechanism for lock acquisition so that operations fail gracefully instead of waiting indefinitely when a lock cannot be obtained.

## Rationale

Timeout support prevents indefinite blocking, frees system resources, and gives users a predictable fail-fast path when locks are stranded or misbehaving.

## User Story (if applicable)

As a Kopi user, I want lock acquisition to respect a configurable timeout and emit clear errors when exceeded, so that I can recover from hung operations without manually terminating processes.

## Acceptance Criteria

- [ ] Lock acquisition waits up to the configured timeout duration before failing when the lock remains unavailable.
- [ ] Timeout configuration precedence follows CLI flag > environment variable > configuration file > built-in default.
- [ ] Special timeout values support `0` (no wait) and `infinite` (wait indefinitely) semantics.
- [ ] Timeout expiration emits an actionable error message that includes the attempted timeout value and override guidance.
- [ ] Users can interrupt lock waits via `Ctrl-C`, producing a graceful cancellation message distinct from timeout errors.
- [ ] Default timeout values are defined and documented per operation (`install`: 30 s, `cache`: 10 s, `uninstall`: 30 s) unless overridden.

## Technical Details (if applicable)

### Functional Requirement Details

- CLI flag: `--lock-timeout <seconds|infinite>` overrides all other sources.
- Environment variable: `KOPI_LOCK_TIMEOUT=<seconds|infinite>`.
- Configuration file key: `lock_timeout = <seconds|infinite>`.
- Timeout evaluation uses `std::time::Instant` for monotonic measurement with exponential backoff polling (10 ms → 20 ms → … → 1 s cap).
- Timeout handling integrates with FR-c04js messaging to surface remaining time and override instructions.

### Non-Functional Requirement Details

N/A – Non-functional aspects handled in [NFR-z6kan](../requirements/NFR-z6kan-lock-timeout-performance.md).

## Platform Considerations

### Unix

- Implement polling with blocking lock attempts and sleep intervals; handle `EINTR` gracefully when users interrupt operations.

### Windows

- Emulate wait timeouts using repeated lock attempts; monitor console control events for cancellations.

### Cross-Platform

- Ensure identical timeout semantics across environments and normalise error messaging for parity.

## Risks & Mitigation

| Risk                               | Impact | Likelihood | Mitigation                                     | Validation                     |
| ---------------------------------- | ------ | ---------- | ---------------------------------------------- | ------------------------------ |
| Timeout too short for slow systems | Medium | Medium     | Ship conservative defaults and allow overrides | Benchmark on slower hardware   |
| Polling overhead                   | Low    | Medium     | Use exponential backoff and sleep hints        | Measure CPU usage under load   |
| Clock skew affects timeout         | Low    | Low        | Use monotonic clocks exclusively               | Test with adjusted system time |

## Implementation Notes

- Surface timeout configuration and the final computed value in debug logs for diagnostics.
- Provide distinct exit codes for timeout vs. cancellation to support scripting use cases.
- Capture timeout occurrences in telemetry to inform future tuning.
- Document timeout behaviour in the external user docs repository (`../kopi-vm.github.io/`).

## External References

N/A – No external references.

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd) in the templates README.
