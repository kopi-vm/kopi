# Process-level locking for cache operations

## Metadata

- ID: FR-v7ql4
- Type: Functional Requirement
- Category: Platform
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

The system SHALL provide exclusive writer locking for cache update operations while allowing concurrent lock-free reads to ensure metadata consistency.

## Rationale

The cache contains critical JDK metadata fetched from external APIs. Without proper locking:

- Concurrent cache refreshes could corrupt the metadata file
- Partial writes could leave the cache in an inconsistent state
- Multiple API calls for the same data waste bandwidth and resources
- Readers could see incomplete or corrupted data during updates

## User Story (if applicable)

As a kopi user, I want cache operations to be safe and consistent, so that I always get reliable metadata even when multiple processes are running.

## Acceptance Criteria

- [ ] Only one process can acquire the cache writer lock at a time
- [ ] Other writers wait or fail based on timeout configuration
- [ ] Cache reads proceed without acquiring locks
- [ ] Multiple readers can access the cache concurrently
- [ ] Cache file is atomically replaced during updates
- [ ] Readers never see partial updates or corrupted data

## Technical Details (if applicable)

### Functional Requirement Details

- Writer lock file: `~/.kopi/locks/cache.lock`
- Lock type: Exclusive lock for writers only
- Update strategy: Write to temporary file, then atomic rename
- Readers access cache file directly without locking
- Use fsync before rename to ensure durability

## Verification Method

### Test Strategy

- Test Type: Integration
- Test Location: `tests/cache_locking_tests.rs` (planned)
- Test Names: `test_fr_v7ql4_concurrent_writers`, `test_fr_v7ql4_reader_writer_concurrent`

### Verification Commands

```bash
# Specific commands to verify this requirement
cargo test test_fr_v7ql4
```

### Success Metrics

- Metric 1: Zero cache corruption incidents during concurrent operations
- Metric 2: Read operations complete without blocking during cache updates

## Dependencies

- Depends on: N/A – No dependencies
- Blocks: N/A – Blocks nothing

## Platform Considerations

### Unix

- Atomic rename via POSIX rename()
- fsync for durability guarantees

### Windows

- Atomic rename via MoveFileEx with MOVEFILE_REPLACE_EXISTING
- FlushFileBuffers for durability

### Cross-Platform

- Consistent temporary file naming pattern
- Handle platform-specific atomic rename semantics

## Risks & Mitigation

| Risk                                       | Impact | Likelihood | Mitigation                     | Validation                    |
| ------------------------------------------ | ------ | ---------- | ------------------------------ | ----------------------------- |
| Non-atomic rename on some filesystems      | High   | Low        | Verify filesystem capabilities | Test on various filesystems   |
| Temporary file left after crash            | Low    | Medium     | Cleanup on startup             | Check for orphaned temp files |
| Reader sees empty cache during first write | Medium | Low        | Ship with default cache        | Verify initial cache present  |

## Implementation Notes

- Use `.tmp` suffix for temporary cache files
- Implement exponential backoff for lock acquisition
- Consider using memory-mapped files for large caches
- Log cache refresh operations for debugging

## External References

N/A – No external references

## Change History

- 2025-09-02: Initial version
- 2025-09-03: Updated to use 5-character ID format
