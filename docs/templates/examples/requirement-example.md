# FR-twzx0-cache-metadata-ttl: Cache JDK Metadata Locally with TTL

## Metadata
- ID: FR-twzx0-cache-metadata-ttl
- Type: Functional Requirement
- Category: Performance, Caching
- Priority: P0 (Critical)
- Owner: Backend Team Lead
- Reviewers: Platform Team, SRE Team
- Status: Implemented
- Date Created: 2024-01-15
- Date Modified: 2024-02-10

## Links
<!-- Internal project artifacts only -->
- Implemented by Tasks: [`T-df1ny-cache-implementation`](../../tasks/T-df1ny-cache-implementation/), [`T-h5ys6-cache-config`](../../tasks/T-h5ys6-cache-config/)
- Related Requirements: FR-7y2x8-offline-mode (Offline Mode), NFR-j3cf1-cache-performance (Cache Performance)
- Related ADRs: [ADR-bw6wd-cache-storage-format](../../adr/ADR-bw6wd-cache-storage-format.md) (SQLite for cache storage)
- Tests: `test_cache_ttl_fr_twzx0`, `test_cache_hit_rate_fr_twzx0`, `bench_cache_performance_nfr_j3cf1`
- Issue: #234
- PR: #567

## Requirement Statement

The system shall cache JDK metadata from the foojay.io API locally with a configurable Time-To-Live (TTL), defaulting to 3600 seconds, to reduce redundant network calls and improve response times.

## Rationale

Analysis showed that 78% of API requests retrieve identical metadata within a 24-hour period. Network calls to foojay.io take 3-5 seconds on average, while cached queries can complete in under 100ms. Implementing local caching will significantly improve user experience and reduce API server load.

## User Story

As a Kopi user, I want JDK metadata to be cached locally, so that repeated searches and operations are fast and don't require network calls every time.

## Acceptance Criteria

- [x] Metadata is stored in local SQLite database at `~/.kopi/cache/metadata.db`
- [x] Cache TTL is configurable via `cache.ttl_seconds` in config.toml
- [x] Default TTL is 3600 seconds (1 hour)
- [x] Cache hit rate exceeds 90% for repeated queries within TTL period
- [x] Expired cache entries are automatically refreshed on next access
- [x] Cache operations handle concurrent access correctly
- [x] Cache can be manually cleared with `kopi cache clear` command
- [x] Cache respects ETag headers when available

## Technical Details

### Functional Requirement Details

**Cache Storage:**
- Location: `~/.kopi/cache/metadata.db`
- Format: SQLite database (per ADR-bw6wd-cache-storage-format)
- Schema includes: metadata content, timestamp, ETag, TTL

**Cache Behavior:**
- On cache miss: Fetch from API, store with timestamp
- On cache hit within TTL: Return cached data
- On cache hit beyond TTL: Fetch fresh data, update cache
- On API failure with stale cache: Use stale cache with warning

**Configuration:**
```toml
[cache]
enabled = true
ttl_seconds = 3600
max_size_mb = 100
```

## Verification Method

### Test Strategy
- Test Type: Unit, Integration, Benchmark
- Test Location: `tests/cache_tests.rs`, `src/cache/mod.rs#[cfg(test)]`
- Test Names: `test_cache_ttl_fr_0001`, `bench_cache_hit_rate_fr_0001`

### Verification Commands
```bash
# Unit tests for cache TTL logic
cargo test test_cache_ttl

# Integration test for cache behavior
cargo test --test cache_integration

# Benchmark cache performance
cargo bench bench_cache_performance

# Manual verification
kopi cache refresh
sleep 3601  # Wait for TTL to expire
kopi cache refresh  # Should fetch fresh data
```

### Success Metrics
- Cache hit rate > 90% within TTL period
- Cache miss fetches complete in < 5 seconds
- Cache hit queries complete in < 100ms
- Zero data corruption over 10,000 read/write cycles

## Dependencies

- Depends on: N/A – No dependencies
- Blocks: FR-0002 (Offline mode requires cache to function)

## Platform Considerations

### Unix
- Cache directory: `~/.kopi/cache/`
- File permissions: 0600 (user read/write only)

### Windows  
- Cache directory: `%USERPROFILE%\.kopi\cache\`
- Uses Windows file locking for concurrent access

### Cross-Platform
- SQLite handles platform differences transparently
- Path separators normalized by Rust's std::path

## Risks & Mitigation

| Risk | Impact | Likelihood | Mitigation | Validation |
|------|--------|------------|------------|------------|
| Cache corruption | High | Low | SQLite ACID properties, checksums | Integrity tests on read |
| Disk space exhaustion | Medium | Low | 100MB size limit, auto-cleanup | Monitor cache size |
| Stale data served | Low | Medium | TTL expiration, ETag validation | Timestamp checks |

## Implementation Notes

- Use `rusqlite` crate for SQLite access
- Implement connection pooling for concurrent access
- Use `tokio::sync::RwLock` for in-memory cache layer
- Consider bloom filter for quick existence checks
- Log cache hits/misses at DEBUG level for monitoring

## External References
- [SQLite Write-Ahead Logging](https://sqlite.org/wal.html) - For concurrent access
- [HTTP ETag](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/ETag) - Cache validation

## Change History

- 2024-01-15: Initial version
- 2024-01-20: Added ETag support requirement
- 2024-02-10: Marked as Implemented after PR #567