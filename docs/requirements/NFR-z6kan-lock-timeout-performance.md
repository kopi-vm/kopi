# Lock acquisition timeout limit

## Metadata

- ID: NFR-z6kan
- Type: Non-Functional Requirement
- Category: Performance
- Priority: P0 (Critical)
- Owner: Development Team
- Reviewers: Architecture Team
- Status: Accepted
- Date Created: 2025-09-02
- Date Modified: 2025-09-03

## Links

- Implemented by Tasks: N/A – Not yet implemented
- Related Requirements: FR-gbsz6
- Related ADRs: [ADR-8mnaz](../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Tests: N/A – Not yet tested
- Issue: N/A – No tracking issue created yet
- PR: N/A – Not yet implemented

## Requirement Statement

The system SHALL have a default lock acquisition timeout of 600 seconds (10 minutes) with support for user configuration ranging from 0 (no-wait) to infinite.

## Rationale

The timeout duration is based on empirical measurements:

- JDK downloads can take 30-60 seconds on slow connections
- Extraction and verification add 10-30 seconds
- Network interruptions may cause retries
- 10 minutes provides sufficient buffer for worst-case scenarios while preventing indefinite hangs

## User Story (if applicable)

The system shall provide reasonable default timeouts to ensure operations complete successfully on slow systems while preventing indefinite hangs.

## Acceptance Criteria

- [ ] Default lock acquisition timeout is 600 seconds
- [ ] Default applies when no user configuration is provided
- [ ] Timeout supports values from 0 to infinite
- [ ] 0 means immediate failure if lock unavailable
- [ ] "infinite" means wait indefinitely
- [ ] Lock acquisition check interval ≤ 100ms
- [ ] Timeout checks have negligible CPU overhead (<0.1%)
- [ ] Timeout enforcement accurate within ±1 second
- [ ] Elapsed time display updates at least every second

## Technical Details (if applicable)

### Non-Functional Requirement Details

- Performance: Lock check interval max 100ms, CPU overhead < 0.1%
- Reliability: Timeout accuracy ±1 second
- Usability: Progress updates every second minimum
- Compatibility: Consistent behavior across all platforms

### Implementation Constraints

- Use monotonic clock for timeout measurement
- Exponential backoff for lock checks: 10ms → 20ms → 40ms → ... → 100ms (max)
- Separate timeouts for different operation types if needed

## Verification Method

### Test Strategy

- Test Type: Benchmark
- Test Location: `benches/lock_performance.rs` (planned)
- Test Names: `bench_nfr_z6kan_lock_overhead`, `bench_nfr_z6kan_timeout_accuracy`

### Verification Commands

```bash
# Specific commands to verify this requirement
cargo bench bench_nfr_z6kan
cargo test test_nfr_z6kan_timeout_accuracy
```

### Success Metrics

- Metric 1: CPU usage during lock wait < 0.1% of single core
- Metric 2: Timeout triggers within target time ±1 second in 99% of cases
- Metric 3: Lock acquisition overhead < 1ms for uncontended locks

## Dependencies

- Depends on: FR-gbsz6 (timeout mechanism implementation)
- Blocks: N/A – Blocks nothing

## Platform Considerations

### Unix

- Use clock_gettime(CLOCK_MONOTONIC) for timing
- Sleep with nanosleep() for precise delays

### Windows

- Use QueryPerformanceCounter for high-resolution timing
- Sleep with Sleep() or WaitForSingleObject

### Cross-Platform

- Consistent timeout behavior regardless of system clock changes
- Account for timer resolution differences

## Risks & Mitigation

| Risk                               | Impact | Likelihood | Mitigation                                 | Validation               |
| ---------------------------------- | ------ | ---------- | ------------------------------------------ | ------------------------ |
| Default timeout too long for CI/CD | Medium | Medium     | Detect CI environment, use shorter default | Test in CI pipelines     |
| Timer resolution affects accuracy  | Low    | Medium     | Use high-resolution timers                 | Test on various hardware |
| CPU overhead from polling          | Medium | Low        | Exponential backoff                        | Benchmark CPU usage      |

## Implementation Notes

- Consider separate defaults: Installation (600s), Cache (60s), Uninstall (300s)
- Log timeout configuration at debug level
- Show remaining time in progress messages
- Consider making timeout adaptive based on operation progress

## External References

N/A – No external references

## Change History

- 2025-09-02: Initial version
- 2025-09-03: Updated to use 5-character ID format
