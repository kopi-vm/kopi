# FR-0003: Process-level locking for cache operations

## Metadata
- Type: Functional Requirement
- Owner: Development Team
- Reviewers: Architecture Team
- Status: Approved
- Priority: P0
- Date Created: 2025-09-02
- Date Modified: 2025-09-02

## Links
- Analysis: [`docs/analysis/AN-0001-concurrent-process-locking.md`](../analysis/AN-0001-concurrent-process-locking.md)
- Related ADRs: [`ADR-0001-concurrent-process-locking-strategy.md`](../adr/ADR-0001-concurrent-process-locking-strategy.md)
- Related Requirements: FR-0004 (timeout)
- Issue: N/A – No tracking issue created yet
- Task: N/A – Implementation not started

## Requirement Statement

The system SHALL provide exclusive writer locking for cache update operations while allowing concurrent lock-free reads to ensure metadata consistency.

## Rationale

The cache contains critical JDK metadata fetched from external APIs. Without proper locking:
- Concurrent cache refreshes could corrupt the metadata file
- Partial writes could leave the cache in an inconsistent state
- Multiple API calls for the same data waste bandwidth and resources
- Readers could see incomplete or corrupted data during updates

## Acceptance Criteria

1. **Exclusive Writer Lock**
   - GIVEN multiple kopi processes
   - WHEN two or more processes attempt to refresh the cache
   - THEN only one process SHALL acquire the cache writer lock
   - AND other writers SHALL wait or fail based on timeout configuration

2. **Lock-free Reads**
   - GIVEN a cache file exists
   - WHEN processes read the cache
   - THEN reads SHALL proceed without acquiring locks
   - AND multiple readers SHALL access the cache concurrently

3. **Atomic Updates**
   - GIVEN a cache update in progress
   - WHEN the update completes
   - THEN the cache file SHALL be atomically replaced
   - AND readers SHALL never see partial updates

4. **Consistency Guarantee**
   - GIVEN concurrent cache operations
   - WHEN updates and reads occur simultaneously
   - THEN readers SHALL see either the old complete cache or the new complete cache
   - AND never a partially written state

## Implementation Notes

- Writer lock file: `~/.kopi/locks/cache.lock`
- Lock type: Exclusive lock for writers only
- Update strategy: Write to temporary file, then atomic rename
- Readers access cache file directly without locking
- Use fsync before rename to ensure durability

## Verification Steps

1. **Concurrent Writer Test**
   - Start two cache refresh operations simultaneously
   - Verify only one proceeds while the other waits

2. **Reader-Writer Test**
   - Start cache refresh operation
   - Simultaneously read cache from multiple processes
   - Verify reads complete successfully without blocking

3. **Atomic Update Test**
   - Monitor cache file during update
   - Verify file is replaced atomically (no partial states visible)

4. **Crash Recovery Test**
   - Kill cache refresh process mid-operation
   - Verify cache remains in consistent state (old or new, not partial)

## Dependencies

- Native std::fs::File locking API (Rust 1.89.0+)
- Atomic file rename operations
- Filesystem fsync support

## Out of Scope

- Distributed cache synchronization
- Cache versioning or rollback
- Read locks or reader tracking