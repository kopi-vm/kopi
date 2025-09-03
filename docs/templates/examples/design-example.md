# JDK Metadata Cache Optimization Design

## Metadata

- Type: Design
- Owner: Backend Team Lead
- Reviewers: Platform Architect, Senior Engineers
- Status: Approved
  <!-- Draft: Work in progress | In Review: Awaiting technical review | Approved: Ready for implementation -->
- Date Created: 2024-01-20

## Links

<!-- Internal project artifacts only. For external resources, see External References section -->

- Requirements: FR-twzx0-cache-metadata-ttl, FR-7y2x8-offline-mode, FR-0cv9r-cache-management, NFR-j3cf1-cache-performance, NFR-z0jyi-cache-size
- Plan: [`docs/tasks/T-df1ny-cache-implementation/plan.md`](plan.md)
- Related ADRs: [ADR-bw6wd-cache-storage-format](../../adr/ADR-bw6wd-cache-storage-format.md), ADR-ygma7-http-client-selection, ADR-6vgm3-progress-indicators
- Issue: #234
- PR: N/A – Not yet implemented

## Overview

This design implements a local caching layer for JDK metadata to reduce API calls and improve performance. The cache uses a TTL-based expiration strategy with checksum validation for data integrity.

## Success Metrics

- [x] Cache lookups complete in <100ms (currently ~5000ms)
- [x] 80% reduction in API calls during normal usage
- [x] Zero data corruption incidents in production

## Background and Current State

- Context: Kopi fetches JDK metadata from foojay.io on every operation
- Current behavior: Direct API calls with no caching, ~5s latency
- Pain points: Slow searches, network dependency, API rate limits
- Constraints: Must maintain backward compatibility
- Related ADRs: ADR-ygma7-http-client-selection (Serialization Format), ADR-6vgm3-progress-indicators (Progress Indicators)

## Requirements Summary

Referenced Functional Requirements:

- **FR-twzx0-cache-metadata-ttl**: Cache JDK metadata locally with TTL
- **FR-7y2x8-offline-mode**: Provide offline mode using cached data
- **FR-0cv9r-cache-management**: Manual cache invalidation command

Referenced Non-Functional Requirements:

- **NFR-j3cf1-cache-performance**: Cache operations complete in <100ms
- **NFR-z0jyi-cache-size**: Cache size under 100MB
- **NFR-07c4m-concurrent-access**: Support concurrent access

## Proposed Design

### High-Level Architecture

```
┌─────────────┐     ┌──────────────┐     ┌──────────────┐
│   CLI       │────▶│ Cache Layer  │────▶│ foojay.io    │
│  Commands   │     │              │     │    API       │
└─────────────┘     └──────────────┘     └──────────────┘
                           │
                           ▼
                    ┌──────────────┐
                    │ Local Cache  │
                    │   Storage    │
                    └──────────────┘
```

### Components

- `CacheStore`: SQLite-based storage (per ADR-bw6wd-cache-storage-format and FR-twzx0-cache-metadata-ttl)
- `CacheManager`: TTL management and expiration (FR-twzx0-cache-metadata-ttl)
- `OfflineHandler`: Fallback for network failures (FR-7y2x8-offline-mode)
- `CacheCommands`: CLI commands for cache control (FR-0cv9r-cache-management)

### Data Flow

1. Command requests metadata
2. CacheManager checks local cache
3. If valid (TTL not expired), return cached data
4. If invalid/missing, fetch from API
5. Store with timestamp and checksum
6. Return fresh data

### Storage Layout and Paths

- Cache root: `~/.kopi/cache/` (all platforms)
- Database: `~/.kopi/cache/metadata.db` (SQLite, per ADR-bw6wd-cache-storage-format)
- Config: TTL configured in `~/.kopi/config.toml`
- Size limit: 100MB max (NFR-z0jyi-cache-size)

### CLI/API Design

Usage

```bash
kopi cache <subcommand> [options]
```

Implementation Notes

- Use existing clap command structure
- Add `CacheCommand` enum with subcommands
- Integrate with existing error handling

### Data Models and Types

```rust
pub struct CachedMetadata {
    pub timestamp: DateTime<Utc>,
    pub ttl_seconds: u64,
    pub checksum: String,
    pub data: MetadataContent,
}

pub struct CacheConfig {
    pub default_ttl: Duration,
    pub max_size_mb: u64,
    pub location: PathBuf,
}
```

### Error Handling

- Use `KopiError::CacheCorrupted` for integrity failures
- Use `KopiError::CacheExpired` for TTL expiration
- Exit codes: 2 (invalid cache), 20 (network required but offline)

### Security Considerations

- SHA256 checksums for cache integrity
- Atomic file writes to prevent corruption
- Validate JSON structure before parsing
- No sensitive data in cache files

### Performance Considerations

- Memory-mapped files for large cache reads
- Lazy loading of cache segments
- Background refresh for nearly-expired cache
- Progress indicators during refresh operations

### Platform Considerations

#### Unix

- Use flock for cache file locking
- Respect XDG_CACHE_HOME if set

#### Windows

- Use Windows file locking APIs
- Store in %LOCALAPPDATA%\kopi\cache

#### Filesystem

- Handle case-insensitive filesystems
- Support paths up to 260 chars on Windows

## ADR References

<!-- Map key design decisions to ADRs -->

| Design Decision              | ADR                                                                           | Status   | Requirement                 |
| ---------------------------- | ----------------------------------------------------------------------------- | -------- | --------------------------- |
| SQLite for cache storage     | [ADR-bw6wd-cache-storage-format](../../adr/ADR-bw6wd-cache-storage-format.md) | Accepted | FR-twzx0-cache-metadata-ttl |
| TTL-based expiration         | ADR-6vgm3-progress-indicators                                                 | Accepted | FR-twzx0-cache-metadata-ttl |
| Error handling with fallback | ADR-efx08-error-handling                                                      | Accepted | FR-7y2x8-offline-mode       |

## Alternatives Considered

1. JSON File Cache
   - Pros: Human-readable, simple implementation
   - Cons: Poor concurrent access, slow for large datasets
   - Decision: Rejected due to NFR-j3cf1-cache-performance requirements

2. Memory-only Cache
   - Pros: Fastest possible access
   - Cons: Lost on restart, doesn't support FR-7y2x8-offline-mode
   - Decision: Rejected, doesn't meet requirements

3. Custom Binary Format
   - Pros: Optimal performance and size
   - Cons: Complex implementation, hard to debug
   - Decision: Rejected, SQLite provides better tradeoffs

Decision Rationale (ADR-bw6wd-cache-storage-format)

- SQLite selected for ACID compliance and concurrent access (NFR-07c4m-concurrent-access)
- Built-in with rusqlite, no external dependencies
- Meets all performance requirements (NFR-j3cf1-cache-performance)

## Migration and Compatibility

- Backward compatibility: Old versions ignore cache files
- Rollout plan: Feature flag `--enable-cache` for gradual adoption
- Telemetry: Track cache hit rates and performance metrics
- Deprecation: None required

## Testing Strategy

### Unit Tests

- `src/cache/mod.rs`: TTL calculation (FR-twzx0-cache-metadata-ttl), expiration logic
- Mock SQLite operations for reliability
- Test concurrent access patterns (NFR-07c4m-concurrent-access)

### Integration Tests

- `tests/cache_integration.rs`: End-to-end cache scenarios
- Test concurrent access and corruption recovery

### External API Parsing

```rust
#[test]
fn test_foojay_response_parsing_fr_twzx0() {
    let json = r#"{"result": [{"id": "abc", "version": "21.0.1"}]}"#;
    let parsed: ApiResponse = serde_json::from_str(json).unwrap();
    assert_eq!(parsed.result[0].version, "21.0.1");
}
```

### Performance & Benchmarks

- `benches/cache_bench.rs`: Measure lookup times
- Target: <100ms for cache hits

## Implementation Plan

- Phase 1: Core cache infrastructure (FR-twzx0-cache-metadata-ttl)
- Phase 2: Offline mode support (FR-7y2x8-offline-mode)
- Phase 3: Cache management commands (FR-0cv9r-cache-management)
- See [`docs/tasks/T-df1ny-cache-implementation/plan.md`](plan.md) for details

## Requirements Mapping

| Requirement                 | Design Section               | Test(s) / Benchmark(s)    |
| --------------------------- | ---------------------------- | ------------------------- |
| FR-twzx0-cache-metadata-ttl | Storage Layout, Data Flow    | tests/cache_ttl.rs        |
| FR-7y2x8-offline-mode       | OfflineHandler component     | tests/offline_mode.rs     |
| FR-0cv9r-cache-management   | CLI/API Design               | tests/cache_commands.rs   |
| NFR-j3cf1-cache-performance | Performance section          | benches/cache_bench.rs    |
| NFR-z0jyi-cache-size        | Storage Layout (100MB limit) | tests/cache_size_limit.rs |
| NFR-07c4m-concurrent-access | Concurrent Access            | tests/concurrent_cache.rs |

## Documentation Impact

- Update `docs/reference.md` with new cache commands
- Add cache troubleshooting section to user docs
- Document cache file format for debugging

## External References (optional)

<!-- External standards, specifications, articles, or documentation only -->

- [Caffeine Cache Design](https://github.com/ben-manes/caffeine/wiki/Design) - High-performance cache design patterns

## Open Questions

- Should we implement cache warming on startup? → Team → Design review

## Appendix

### Diagrams

```
Cache State Machine:
    ┌──────┐
    │ Empty │
    └───┬───┘
        │ fetch
    ┌───▼───┐
    │ Valid  │◄──── refresh
    └───┬───┘
        │ expire
    ┌───▼───┐
    │Expired│
    └───────┘
```

### Examples

```bash
# Force refresh
kopi cache refresh --no-cache

# Check cache status
kopi cache info
# Output: Cache age: 2 hours, Size: 1.2 MB, Entries: 145
```

### Glossary

- TTL: Time To Live, duration before cache expiration
- Checksum: Hash value for data integrity verification

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../README.md#design-template-designmd) in the templates README.
