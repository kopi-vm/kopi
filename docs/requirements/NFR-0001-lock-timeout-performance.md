# Lock acquisition timeout limit

## Metadata
- Type: Non-Functional Requirement
- Category: Performance
- Owner: Development Team
- Reviewers: Architecture Team
- Status: Approved
- Priority: P0
- Date Created: 2025-09-02
- Date Modified: 2025-09-02

## Links
- Analysis: [`docs/analysis/AN-m9efc-concurrent-process-locking.md`](../analysis/AN-m9efc-concurrent-process-locking.md)
- Related ADRs: [`ADR-8mnaz-concurrent-process-locking-strategy.md`](../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Related Requirements: FR-0004 (timeout mechanism)
- Issue: N/A – No tracking issue created yet
- Task: N/A – Implementation not started

## Requirement Statement

The system SHALL have a default lock acquisition timeout of 600 seconds (10 minutes) with support for user configuration ranging from 0 (no-wait) to infinite.

## Rationale

The timeout duration is based on empirical measurements:
- JDK downloads can take 30-60 seconds on slow connections
- Extraction and verification add 10-30 seconds
- Network interruptions may cause retries
- 10 minutes provides sufficient buffer for worst-case scenarios while preventing indefinite hangs

## Acceptance Criteria

1. **Default Timeout**
   - The default lock acquisition timeout SHALL be 600 seconds
   - This default SHALL apply when no user configuration is provided

2. **Configuration Range**
   - Timeout SHALL support values from 0 to infinite
   - 0 SHALL mean immediate failure if lock unavailable
   - "infinite" SHALL mean wait indefinitely

3. **Performance Impact**
   - Lock acquisition check interval SHALL be ≤ 100ms
   - Timeout checks SHALL have negligible CPU overhead (<0.1%)

4. **Measurement Accuracy**
   - Timeout enforcement SHALL be accurate within ±1 second
   - Elapsed time display SHALL update at least every second

## Measurement Methods

- **Load Testing**: Simulate 10 concurrent processes with lock contention
- **Performance Profiling**: Measure CPU usage during lock wait
- **Timeout Accuracy**: Test with various timeout values (1s, 10s, 600s)
- **Network Simulation**: Test with simulated slow network (bandwidth limiting)

## Target Metrics

| Metric | Target | Minimum Acceptable |
|--------|--------|-------------------|
| Default timeout | 600s | 300s |
| Lock check interval | 100ms | 1000ms |
| CPU usage during wait | <0.1% | <1% |
| Timeout accuracy | ±1s | ±5s |
| Configuration parse time | <1ms | <10ms |

## Implementation Notes

- Use exponential backoff for lock checks (start at 10ms, max 100ms)
- Implement using std::time::Instant for accurate timing
- Consider system load when setting check intervals

## Verification Steps

1. **Default Timeout Test**
   - Run without configuration
   - Verify 600-second timeout is used

2. **CPU Usage Test**
   - Monitor CPU during 600-second wait
   - Verify usage stays below 0.1%

3. **Accuracy Test**
   - Test timeouts at 1s, 10s, 60s, 600s
   - Verify accuracy within ±1 second

4. **Configuration Test**
   - Test all configuration methods
   - Verify correct timeout values are applied

## Dependencies

- High-resolution timer support
- Configuration system implementation

## Out of Scope

- Dynamic timeout adjustment based on operation type
- Predictive timeout based on historical data
- Network speed detection for timeout adjustment