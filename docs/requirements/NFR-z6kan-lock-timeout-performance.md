# Lock acquisition timeout limit

## Metadata

- Type: Non-Functional Requirement
- Status: Accepted
  <!-- Proposed: Under discussion | Accepted: Approved for implementation | Implemented: Code complete | Verified: Tests passing | Deprecated: No longer applicable -->

## Links

- Implemented by Tasks: N/A – Not yet implemented
- Related Requirements: FR-gbsz6
- Related ADRs: ADR-8mnaz
- Tests: N/A – Not yet tested
- Issue: N/A – No tracking issue created yet
- PR: N/A – Not yet implemented

## Requirement Statement

The system SHALL ship with a default lock acquisition timeout of 600 seconds (10 minutes) while allowing configuration from 0 (no wait) to infinite, maintaining low CPU overhead and precise timeout accuracy.

## Rationale

Empirical measurements show that slow networks and large JDK downloads can take minutes; a generous default avoids premature failures while configurable bounds prevent indefinite hangs and support automation needs.

## User Story (if applicable)

The system shall provide sensible timeout defaults and precision control so that users on slow systems can complete operations without hangs while automation can choose aggressive limits.

## Acceptance Criteria

- [ ] Default lock timeout is 600 seconds when no explicit configuration is supplied.
- [ ] Supported timeout range includes `0` (immediate failure) through `infinite` (no timeout) with validation on user-provided values.
- [ ] Lock acquisition polling interval remains ≤ 100 ms once steady state backoff is reached.
- [ ] Timeout accuracy remains within ±1 second of the configured value across 99% of measured cases.
- [ ] Timeout enforcement adds <0.1% CPU overhead on a single core during waits measured over 5-minute intervals.
- [ ] Progress displays (per FR-c04js) update elapsed/remaining time at least once per second when the timeout is finite.

## Technical Details (if applicable)

### Functional Requirement Details

N/A – Not applicable.

### Non-Functional Requirement Details

- Performance: Use exponential backoff (10 ms → 20 ms → 40 ms → … → 100 ms cap) to balance responsiveness and CPU usage.
- Reliability: Base timing on `std::time::Instant` to avoid wall-clock adjustments.
- Compatibility: Ensure identical timing behavior on Unix and Windows high-resolution timers.
- Usability: Provide warning when user-specified timeout exceeds 1 hour to prompt validation of intent.

#### Implementation Constraints

- Provide separate defaults per operation type if required (`install`: 600 s, `cache`: 60 s, `uninstall`: 300 s) while retaining global fallback.
- Expose timeout configuration via CLI flag, environment variable, and config file consistent with FR-gbsz6.

## Verification Method

### Test Strategy

- Test Type: Benchmark
- Test Location: `benches/lock_performance.rs` (planned)
- Test Names: `bench_nfr_z6kan_lock_overhead`, `bench_nfr_z6kan_timeout_accuracy`

### Verification Commands

```bash
# Specific commands to verify this requirement
cargo bench bench_nfr_z6kan_lock_overhead
cargo bench bench_nfr_z6kan_timeout_accuracy
cargo test test_nfr_z6kan_timeout_accuracy
```

### Success Metrics

- Metric 1: CPU usage during lock waits remains <0.1% of a single core averaged over 5 minutes.
- Metric 2: Timeout triggers occur within ±1 second of configured values in 99% of benchmark samples.
- Metric 3: Progress output updates at least once per second in TTY environments and every 5 seconds in non-TTY contexts.

## Dependencies

- Depends on: FR-gbsz6 (timeout mechanism implementation)
- Blocks: N/A – Blocks nothing

## Platform Considerations

### Unix

- Utilize `clock_gettime(CLOCK_MONOTONIC)` and `nanosleep` for precise timing.

### Windows

- Leverage `QueryPerformanceCounter` for timing and `Sleep`/`WaitForSingleObject` for waits.

### Cross-Platform

- Normalize timer resolution differences and guard against drift from system clock adjustments.

## Risks & Mitigation

| Risk                               | Impact | Likelihood | Mitigation                                         | Validation                   |
| ---------------------------------- | ------ | ---------- | -------------------------------------------------- | ---------------------------- |
| Default timeout too long for CI/CD | Medium | Medium     | Detect CI environments; recommend shorter defaults | Test within CI pipelines     |
| Timer resolution affects accuracy  | Low    | Medium     | Use high-resolution timers and calibrate           | Benchmark on varied hardware |
| CPU overhead from polling          | Medium | Low        | Apply exponential backoff and sleep hints          | Measure CPU usage            |

## Implementation Notes

- Log effective timeout values and source precedence at debug level for troubleshooting.
- Consider adaptive tuning based on operation progress metrics (e.g., download completion percentage).
- Provide documentation in external user docs about timeout implications for automation and CI.

## External References

N/A – No external references

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd).
