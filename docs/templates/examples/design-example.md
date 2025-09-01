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
- Requirements: FR-0001, FR-0002, FR-0003, NFR-0001, NFR-0002
- Plan: [`docs/tasks/cache-implementation/plan.md`](plan.md)
- Related ADRs: [ADR-015](../../adr/015-cache-storage-format.md), ADR-002, ADR-006
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
- Related ADRs: ADR-002 (Serialization Format), ADR-006 (Progress Indicators)

## Requirements Summary

Referenced Functional Requirements:
- **FR-0001**: Cache JDK metadata locally with TTL
- **FR-0002**: Provide offline mode using cached data
- **FR-0003**: Manual cache invalidation command

Referenced Non-Functional Requirements:
- **NFR-0001**: Cache operations complete in <100ms
- **NFR-0002**: Cache size under 100MB
- **NFR-0003**: Support concurrent access

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
- `CacheStore`: SQLite-based storage (per ADR-015 and FR-0001)
- `CacheManager`: TTL management and expiration (FR-0001)
- `OfflineHandler`: Fallback for network failures (FR-0002)
- `CacheCommands`: CLI commands for cache control (FR-0003)

### Data Flow
1. Command requests metadata
2. CacheManager checks local cache
3. If valid (TTL not expired), return cached data
4. If invalid/missing, fetch from API
5. Store with timestamp and checksum
6. Return fresh data

### Storage Layout and Paths
- Cache root: `~/.kopi/cache/` (all platforms)
- Database: `~/.kopi/cache/metadata.db` (SQLite, per ADR-015)
- Config: TTL configured in `~/.kopi/config.toml`
- Size limit: 100MB max (NFR-0002)

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
| Design Decision | ADR | Status | Requirement |
|-----------------|-----|--------|-------------|
| SQLite for cache storage | [ADR-015](../../adr/015-cache-storage-format.md) | Accepted | FR-0001 |
| TTL-based expiration | ADR-006 | Accepted | FR-0001 |
| Error handling with fallback | ADR-004 | Accepted | FR-0002 |

## Alternatives Considered

1. JSON File Cache
   - Pros: Human-readable, simple implementation
   - Cons: Poor concurrent access, slow for large datasets
   - Decision: Rejected due to NFR-0001 performance requirements

2. Memory-only Cache
   - Pros: Fastest possible access
   - Cons: Lost on restart, doesn't support FR-0002 offline mode
   - Decision: Rejected, doesn't meet requirements

3. Custom Binary Format
   - Pros: Optimal performance and size
   - Cons: Complex implementation, hard to debug
   - Decision: Rejected, SQLite provides better tradeoffs

Decision Rationale (ADR-015)
- SQLite selected for ACID compliance and concurrent access (NFR-0003)
- Built-in with rusqlite, no external dependencies
- Meets all performance requirements (NFR-0001)

## Migration and Compatibility

- Backward compatibility: Old versions ignore cache files
- Rollout plan: Feature flag `--enable-cache` for gradual adoption
- Telemetry: Track cache hit rates and performance metrics
- Deprecation: None required

## Testing Strategy

### Unit Tests
- `src/cache/mod.rs`: TTL calculation (FR-0001), expiration logic
- Mock SQLite operations for reliability
- Test concurrent access patterns (NFR-0003)

### Integration Tests
- `tests/cache_integration.rs`: End-to-end cache scenarios
- Test concurrent access and corruption recovery

### External API Parsing
```rust
#[test]
fn test_foojay_response_parsing() {
    let json = r#"{"result": [{"id": "abc", "version": "21.0.1"}]}"#;
    let parsed: ApiResponse = serde_json::from_str(json).unwrap();
    assert_eq!(parsed.result[0].version, "21.0.1");
}
```

### Performance & Benchmarks
- `benches/cache_bench.rs`: Measure lookup times
- Target: <100ms for cache hits

## Implementation Plan

- Phase 1: Core cache infrastructure (FR-0001)
- Phase 2: Offline mode support (FR-0002)
- Phase 3: Cache management commands (FR-0003)
- See [`docs/tasks/cache-implementation/plan.md`](plan.md) for details

## Requirements Mapping

| Requirement | Design Section | Test(s) / Benchmark(s) |
|-------------|----------------|-------------------------|
| FR-0001 | Storage Layout, Data Flow | tests/cache_ttl.rs |
| FR-0002 | OfflineHandler component | tests/offline_mode.rs |
| FR-0003 | CLI/API Design | tests/cache_commands.rs |
| NFR-0001 | Performance section | benches/cache_bench.rs |
| NFR-0002 | Storage Layout (100MB limit) | tests/cache_size_limit.rs |
| NFR-0003 | Concurrent Access | tests/concurrent_cache.rs |

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