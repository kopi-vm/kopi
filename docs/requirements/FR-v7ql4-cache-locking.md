# Process-level locking for cache operations

## Metadata

- Type: Functional Requirement
- Status: Accepted
  <!-- Proposed: Under discussion | Accepted: Approved for implementation | Implemented: Code complete | Verified: Tests passing | Deprecated: No longer applicable -->

## Links

- Implemented by Tasks: N/A – Not yet implemented
- Related Requirements: FR-gbsz6, NFR-vcxp8
- Related ADRs: ADR-8mnaz
- Tests: N/A – Not yet tested
- Issue: N/A – No tracking issue created yet
- PR: N/A – Not yet implemented

## Requirement Statement

The system SHALL serialize cache mutation operations using exclusive writer locks while allowing concurrent lock-free reads, ensuring JDK metadata cache consistency during refreshes.

## Rationale

Cache files contain critical metadata fetched from foojay.io. Without writer locking, concurrent refreshes risk corrupting files, readers could observe incomplete data, and duplicate downloads would waste bandwidth.

## User Story (if applicable)

As a kopi user, I want metadata cache updates to occur safely without blocking reads, so that I always receive consistent data even when background refreshes are running.

## Acceptance Criteria

- [ ] Only one writer at a time can acquire the cache lock; additional writers wait or fail according to timeout settings.
- [ ] Readers access cache files without locks and never observe partially written data.
- [ ] Cache refresh writes to a temporary file and atomically replaces the target on success.
- [ ] Cache corruption is prevented under concurrent reader and writer workloads across 100 stress iterations.
- [ ] The cache lock uses a well-known path (`cache.lock`) to coordinate writers across processes.

## Technical Details (if applicable)

### Functional Requirement Details

- Lock file path: `$KOPI_HOME/locks/cache.lock` using exclusive advisory lock semantics.
- Refresh workflow: fetch metadata → write to temp file → `fsync` → atomic rename to destination.
- Readers open the cache file directly without locking but verify file integrity checks (e.g., checksum or length) post-read.
- Writer retries use exponential backoff aligned with FR-gbsz6 timeout configuration.

### Non-Functional Requirement Details

N/A – Not applicable.

## Verification Method

### Test Strategy

- Test Type: Integration
- Test Location: `tests/cache_locking_tests.rs` (planned)
- Test Names: `test_fr_v7ql4_concurrent_writers`, `test_fr_v7ql4_reader_writer_concurrent`

### Verification Commands

```bash
# Specific commands to verify this requirement
cargo test test_fr_v7ql4_concurrent_writers
cargo test test_fr_v7ql4_reader_writer_concurrent
```

### Success Metrics

- Metric 1: Zero cache corruption incidents during 100 concurrent writer stress tests.
- Metric 2: Reader latency impact remains under 5% when a writer is active.
- Metric 3: Cache refresh completion time remains within baseline ±10% under contention.

## Dependencies

- Depends on: FR-gbsz6 (timeout and contention handling)
- Blocks: N/A – Blocks nothing

## Platform Considerations

### Unix

- Use POSIX rename for atomic replacement and `fsync` to ensure durability before rename.

### Windows

- Use `MoveFileEx` with `MOVEFILE_REPLACE_EXISTING` for atomic replacement and `FlushFileBuffers` for durability.

### Cross-Platform

- Temporary file naming must avoid collisions (`cache.lock.tmpXXXX`).
- Normalize error handling to surface consistent messages regardless of platform.

## Risks & Mitigation

| Risk                                       | Impact | Likelihood | Mitigation                     | Validation                    |
| ------------------------------------------ | ------ | ---------- | ------------------------------ | ----------------------------- |
| Non-atomic rename on some filesystems      | High   | Low        | Detect and warn; fallback copy | Test on varied filesystems    |
| Temporary file left after crash            | Low    | Medium     | Cleanup orphaned temp files    | Startup hygiene routine       |
| Reader sees empty cache during first write | Medium | Low        | Ship default cache snapshot    | Verify initial cache presence |

## Implementation Notes

- Use `.tmp` suffix for temporary files and ensure they live on the same filesystem as the final cache to maintain atomicity.
- Consider memory-mapped reads for performance while maintaining simple writer implementation.
- Track cache refresh metrics to inform timeout and backoff tuning.
- Emit debug logs for lock acquisition, contention, and refresh completion.

## External References

N/A – No external references

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd).
