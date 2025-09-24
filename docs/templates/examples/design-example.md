# JDK Metadata Cache Optimization Design

## Metadata

- Type: Design
- Status: Approved
  <!-- Draft: Work in progress | In Review: Awaiting technical review | Approved: Ready for implementation -->

## Links

<!-- Internal project artifacts only. Replace or remove bullets as appropriate. -->

- Related Requirements:
  - [FR-twzx0-cache-metadata-ttl](../../requirements/FR-twzx0-cache-metadata-ttl.md)
  - [FR-7y2x8-offline-mode](../../requirements/FR-7y2x8-offline-mode.md)
  - [FR-0cv9r-cache-management](../../requirements/FR-0cv9r-cache-management.md)
  - [NFR-j3cf1-cache-performance](../../requirements/NFR-j3cf1-cache-performance.md)
  - [NFR-z0jyi-cache-size](../../requirements/NFR-z0jyi-cache-size.md)
- Related ADRs:
  - [ADR-bw6wd-cache-storage-format](../../adr/ADR-bw6wd-cache-storage-format.md)
  - [ADR-ygma7-http-client-selection](../../adr/ADR-ygma7-http-client-selection.md)

## Overview

This design introduces a persistent cache for foojay.io metadata with TTL-based freshness checks to eliminate redundant network calls and ensure sub-100ms lookups for repeated queries.

## Success Metrics

- [ ] Achieve ≥80% cache hit rate during normal CLI use.
- [ ] Maintain cache lookups under 100ms at the 95th percentile.
- [ ] Ensure zero regressions in JDK discovery workflows.

## Background and Current State

- Context: Kopi retrieves remote metadata during version discovery and installation.
- Current behavior: Each command performs a full API fetch, taking 3-5 seconds on average.
- Pain points: Slow searches, inability to function offline, API rate limit pressure.
- Constraints: No additional background services; must function on Windows, macOS, and Linux.
- Related ADRs: ADR-ygma7-http-client-selection (HTTP client baseline), ADR-6vgm3-progress-indicators (rendering progress).

## Requirements Summary (from requirements.md)

Referenced Functional Requirements

- FR-twzx0-cache-metadata-ttl
- FR-7y2x8-offline-mode
- FR-0cv9r-cache-management

Referenced Non-Functional Requirements

- NFR-j3cf1-cache-performance
- NFR-z0jyi-cache-size
- NFR-07c4m-concurrent-access

## Proposed Design

### High-Level Architecture

```text
┌────────────┐      ┌────────────────┐      ┌──────────────┐
│ CLI Command│ ───▶ │ Cache Orchestrator │ ─▶ │ foojay.io API│
└────────────┘      └────────────────┘      └──────────────┘
        │                     │
        │                     ▼
        │             ┌──────────────┐
        └────────────▶│ Local Storage│
                      └──────────────┘
```

### Components

- `CacheOrchestrator`: Coordinates cache lookups, TTL checks, and fallback to the network.
- `CacheStore`: SQLite-backed repository that persists metadata blobs and metadata about freshness.
- `OfflineResolver`: Determines whether to rely on cached data when the network is unavailable.
- `CacheCli`: Provides `kopi cache` subcommands for inspection, refresh, and clearing.

### Data Flow

- Command requests metadata via `CacheOrchestrator`.
- Orchestrator queries `CacheStore`; if entry is fresh, returns cached payload.
- Stale or missing entries trigger a remote fetch, after which responses are stored with updated timestamps and checksums.
- `OfflineResolver` short-circuits network calls when offline mode is enforced.

### Storage Layout and Paths (if applicable)

- Cache root: `~/.kopi/cache/` (Unix) / `%LOCALAPPDATA%\kopi\cache\` (Windows).
- Database: `metadata.db` (SQLite) containing tables for manifests and TTL metadata.
- Temporary files: `metadata.tmp` within the cache directory for atomic writes.

### CLI/API Design (if applicable)

Usage

```bash
kopi cache <subcommand> [options]
```

Options

- `info`: Display cache statistics and TTL status.
- `refresh`: Force re-fetch of metadata, bypassing cache.
- `clear`: Remove all cache entries.
- `--offline`: Restrict commands to cached content only.

Examples

```bash
kopi cache info
kopi cache refresh --offline=false
```

Implementation Notes

- Extend the existing Clap configuration with a `cache` command tree.
- Reuse progress indicator settings from ADR-6vgm3 to surface cache refresh status.

### Data Models and Types

- `CachedMetadata { id: String, payload: Vec<u8>, ttl_expires_at: DateTime<Utc>, checksum: String }`
- `CacheConfig { default_ttl: Duration, max_size_mb: u64, location: PathBuf }`

### Error Handling

- Use `KopiError::CacheExpired`, `KopiError::CacheCorrupted`, and `KopiError::OfflineUnavailable` with actionable messages.
- Attach `ErrorContext` entries specifying cache path, TTL, and command parameters.
- Exit codes: `2` for invalid cache data, `20` for enforced offline failures, `28` for disk write issues.

### Security Considerations

- Strip credentials from request keys before persisting to cache.
- Enforce file permissions `0o600` (Unix) and `FILE_GENERIC_READ|FILE_GENERIC_WRITE` (Windows).
- Validate checksums before serving cached payloads.

### Performance Considerations

- Batch writes using transactions to minimize disk I/O.
- Track hit/miss counters and log summary at `INFO` when `--verbose` is enabled.
- Provide metrics hooks for future perf benchmarking.

### Platform Considerations

#### Unix

- Respect `$XDG_DATA_HOME` overrides for cache location.
- Support concurrent access via POSIX advisory locks.

#### Windows

- Handle long path support (`\\?\` prefix) for cache directory.
- Utilize Windows file locking primitives to avoid corruption.

#### Filesystem

- Normalize case when storing cache keys to avoid duplicate entries on case-insensitive filesystems.
- Ensure atomic replace via `rename` semantics on supported platforms.

## ADR References

| Design Decision             | ADR                               | Status   |
| --------------------------- | --------------------------------- | -------- |
| Cache storage format        | ADR-bw6wd-cache-storage-format    | Accepted |
| HTTP client selection       | ADR-ygma7-http-client-selection   | Accepted |
| Progress rendering strategy | ADR-6vgm3-progress-indicators     | Accepted |

## Alternatives Considered

1. In-memory cache only
   - Pros: Simplest implementation, fastest lookups.
   - Cons: No offline support; data lost between runs; fails FR-7y2x8.
2. JSON file-based cache
   - Pros: Human-readable, easy debugging.
   - Cons: Slow for large payloads, concurrency concerns; fails NFR-j3cf1.

Decision Rationale

SQLite provides transactional safety, cross-platform support, and performant queries while keeping implementation effort moderate.

## Migration and Compatibility

- Backward compatibility: Older Kopi clients ignore the cache directory safely.
- Rollout plan: Feature flag `cache.enabled` defaults to true but can be toggled for staged adoption.
- Telemetry: Record hit/miss metrics and average lookup time in debug logs for manual analysis.
- Deprecation plan: None; legacy behavior remains available by disabling cache.

## Testing Strategy

### Unit Tests

- `src/cache/storage.rs`: TTL arithmetic and checksum validation.
- `src/cache/orchestrator.rs`: Offline fallback and hit/miss counting.

### Integration Tests

- `tests/cache_cli.rs`: Validate CLI commands (`info`, `refresh`, `clear`).
- `tests/cache_offline.rs`: Simulate offline usage and stale cache recovery.

### External API Parsing (if applicable)

- Include captured foojay.io JSON in `tests/cache_parsing.rs` to validate schema compatibility.

### Performance & Benchmarks (if applicable)

- `benches/cache_lookup.rs`: Measure lookup times and log 95th percentile results.

## Implementation Plan

- Phase 1: Introduce `CacheStore` with SQLite tables and TTL enforcement.
- Phase 2: Add `CacheOrchestrator` integration into existing CLI commands.
- Phase 3: Deliver CLI tooling and offline mode refinements.

## Requirements Mapping

| Requirement                 | Design Section                     | Test(s) / Benchmark(s)     |
| --------------------------- | ---------------------------------- | -------------------------- |
| FR-twzx0-cache-metadata-ttl | Data Flow, Storage Layout          | tests/cache_cli.rs         |
| FR-7y2x8-offline-mode       | OfflineResolver, Testing Strategy  | tests/cache_offline.rs     |
| FR-0cv9r-cache-management   | CLI/API Design                     | tests/cache_cli.rs         |
| NFR-j3cf1-cache-performance | Performance Considerations         | benches/cache_lookup.rs    |
| NFR-z0jyi-cache-size        | Storage Layout and Paths           | tests/cache_limits.rs      |
| NFR-07c4m-concurrent-access | Platform Considerations (Unix/Win) | tests/cache_concurrency.rs |

## Documentation Impact

- Update `docs/reference.md` with `kopi cache` usage examples.
- Coordinate with `../kopi-vm.github.io/` to publish user-facing cache guidance.
- Reference ADR updates for cache eviction decisions when finalized.

## External References (optional)

- [Caffeine Cache Design](https://github.com/ben-manes/caffeine/wiki/Design) - High-performance cache design patterns.

## Open Questions

- Do we need configurable per-namespace TTLs? → Platform Team → Q2 roadmap review.

## Appendix

### Diagrams

```text
Cache State Machine:
    ┌──────┐
    │Empty │
    └─┬────┘
      │ fetch
    ┌─▼────┐   refresh   ┌─────────┐
    │Valid │────────────▶│Refreshing│
    └─┬────┘             └────┬────┘
      │ expire                 │ complete
    ┌─▼────┐                   ▼
    │Stale │◀──────────────────┘
    └──────┘
```

### Examples

```bash
# Force refresh regardless of TTL
kopi cache refresh --offline=false

# Inspect cache statistics
kopi cache info --verbose
```

### Glossary

- TTL: Time to Live; duration before cached data is considered stale.
- Checksum: SHA-256 digest validating payload integrity.

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../README.md#design-template-designmd) in the templates README.
