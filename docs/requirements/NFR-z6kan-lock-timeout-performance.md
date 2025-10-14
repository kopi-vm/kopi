# NFR-z6kan Lock Timeout Performance

## Metadata

- Type: Non-Functional Requirement
- Status: Approved
  <!-- Draft: Under discussion | Approved: Ready for implementation | Rejected: Decision made not to pursue this requirement -->

## Links

- Prerequisite Requirements:
  - [FR-gbsz6-lock-timeout-recovery](../requirements/FR-gbsz6-lock-timeout-recovery.md)
- Dependent Requirements:
  - N/A – No dependent requirements recorded
- Related Tasks:
  - [T-lqyk8-lock-timeout-control](../tasks/T-lqyk8-lock-timeout-control/README.md)

## Requirement Statement

Kopi SHALL ship with a default lock acquisition timeout of 600 seconds (10 minutes) while allowing configuration from 0 (no wait) to infinite, maintaining low CPU overhead and precise timeout accuracy.

## Rationale

Empirical measurements show that slow networks and large JDK downloads can take minutes; a generous default avoids premature failures while configurable bounds prevent indefinite hangs and support automation needs.

## User Story (if applicable)

The system shall provide sensible timeout defaults and precision control so that users on slow systems can complete operations without hangs while automation can choose aggressive limits.

## Acceptance Criteria

- [ ] Default lock timeout is 600 seconds when no explicit configuration is supplied.
- [ ] Supported timeout range includes `0` (immediate failure) through `infinite` (no timeout) with validation on user-provided values.
- [ ] Lock acquisition polling interval remains ≤ 100 ms once steady-state backoff is reached.
- [ ] Timeout accuracy remains within ±1 second of the configured value across 99% of measured cases.
- [ ] Timeout enforcement adds <0.1% CPU overhead on a single core during waits measured over 5-minute intervals.
- [ ] Progress displays (per FR-c04js) update elapsed/remaining time at least once per second when the timeout is finite.

## Technical Details (if applicable)

### Functional Requirement Details

N/A – Behavioural focus only.

### Non-Functional Requirement Details

- Performance: Use exponential backoff (10 ms → 20 ms → 40 ms → … → 100 ms cap) to balance responsiveness and CPU usage.
- Reliability: Base timing on `std::time::Instant` to avoid wall-clock adjustments.
- Compatibility: Ensure identical timing behaviour on Unix and Windows high-resolution timers.
- Usability: Provide a warning when user-specified timeout exceeds 1 hour to prompt validation of intent.

## Platform Considerations

### Unix

- Utilise `clock_gettime(CLOCK_MONOTONIC)` and `nanosleep` for precise timing.

### Windows

- Leverage `QueryPerformanceCounter` for timing and `Sleep`/`WaitForSingleObject` for waits.

### Cross-Platform

- Normalise timer resolution differences and guard against drift from system clock adjustments.

## Risks & Mitigation

| Risk                               | Impact | Likelihood | Mitigation                                         | Validation                   |
| ---------------------------------- | ------ | ---------- | -------------------------------------------------- | ---------------------------- |
| Default timeout too long for CI/CD | Medium | Medium     | Detect CI environments; recommend shorter defaults | Test within CI pipelines     |
| Timer resolution affects accuracy  | Low    | Medium     | Use high-resolution timers and calibrate           | Benchmark on varied hardware |
| CPU overhead from polling          | Medium | Low        | Apply exponential backoff and sleep hints          | Measure CPU usage            |

## Implementation Notes

- Log effective timeout values and precedence at debug level for troubleshooting.
- Consider adaptive tuning based on operation progress metrics (e.g., download completion percentage).
- Document timeout behaviour in the external user docs repository for automation guidance.

## External References

N/A – No external references.

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd) in the templates README.
