# JDK Metadata Cache Optimization Analysis

## Metadata

- Type: Analysis
- Status: Complete
  <!-- Draft: Initial exploration | Active: Ongoing analysis | Complete: Ready for requirements | Archived: Analysis concluded -->

## Links

<!-- Internal project artifacts only. Replace or remove bullets as appropriate. -->

- Related Analyses:
  - N/A – Standalone investigation
- Related Requirements:
  - [FR-twzx0-cache-metadata-ttl](../../requirements/FR-twzx0-cache-metadata-ttl.md)
  - [NFR-j3cf1-cache-performance](../../requirements/NFR-j3cf1-cache-performance.md)
- Related ADRs:
  - [ADR-ygma7-http-client-selection](../../adr/ADR-ygma7-http-client-selection.md)

## Executive Summary

Analysis of performance issues with JDK metadata fetching revealed that Kopi makes redundant API calls to foojay.io on every `cache refresh` command. Users experience 3-5 second delays, especially in regions with high latency. This analysis explores caching strategies to improve performance and reduce API load.

## Problem Space

### Current State

- Every `cache refresh` fetches full metadata from foojay.io API (~2-3MB).
- No local caching mechanism exists.
- Users in high-latency regions experience 3-5 second delays.
- API calls made even when metadata has not changed.
- No offline capability.

### Desired State

- Sub-second response for cached data.
- Intelligent caching with TTL-based expiration.
- Offline mode using cached data.
- 80% reduction in API calls.
- Configurable cache behavior.

### Gap Analysis

- Need persistent cache storage (50-100MB).
- Need cache invalidation strategy.
- Need TTL and ETag support.
- Need offline detection and fallback.
- Need cache configuration options.

## Stakeholder Analysis

| Stakeholder                | Interest/Need              | Impact | Priority |
| -------------------------- | -------------------------- | ------ | -------- |
| CLI Users                  | Fast version searches      | High   | P0       |
| CI/CD Systems              | Reliable offline operation | High   | P0       |
| API Provider               | Reduced server load        | Medium | P1       |
| Mobile/Low-bandwidth Users | Minimal data transfer      | High   | P1       |

## Research & Discovery

### User Feedback

- Support ticket #234: "Takes forever to search for JDK versions."
- Forum post: "Can't use Kopi on airplane—needs offline mode."
- Survey: 67% of users experience slow searches at least weekly.

### Competitive Analysis

- **nvm**: Caches version lists for 60 minutes; supports offline usage.
- **pyenv**: Updates cache only on explicit `update` command.
- **volta**: Smart caching with ETag validation.
- **sdkman**: Manual cache refresh with `flush` command.

### Technical Investigation

POC implemented with SQLite cache:

- 95% reduction in response time for cached queries.
- 82% reduction in API calls over 7-day period.
- Cache size: ~45MB for full metadata.
- SQLite performs well across all platforms.

### Data Analysis

API call patterns from logs (1000 users, 30 days):

- 78% of requests are for the same metadata within 24 hours.
- Peak usage during CI builds (hundreds of identical requests).
- Metadata actually changes ~2-3 times per week.

## Discovered Requirements

### Functional Requirements (Potential)

- [ ] **FR-DRAFT-1**: Cache metadata locally with configurable TTL → Will become FR-twzx0-cache-metadata-ttl
  - Rationale: Eliminate redundant API calls.
  - Priority: P0.
  - Acceptance Criteria: 90% cache hit rate for repeated queries within TTL.

- [ ] **FR-DRAFT-2**: Provide offline mode using cached data → Will become FR-7y2x8-offline-mode
  - Rationale: Support disconnected development environments.
  - Priority: P0.
  - Acceptance Criteria: All read operations work offline with cached data.

- [ ] **FR-DRAFT-3**: Manual cache invalidation command → Will become FR-0cv9r-cache-management
  - Rationale: Allow users to force-refresh when needed.
  - Priority: P1.
  - Acceptance Criteria: `kopi cache clear` removes all cached data.

### Non-Functional Requirements (Potential)

- [ ] **NFR-DRAFT-1**: Cache operations complete in <100ms → Will become NFR-j3cf1-cache-performance
  - Category: Performance.
  - Target: 95th percentile under 100ms for cache hits.
  - Rationale: 10x improvement over current network calls.

- [ ] **NFR-DRAFT-2**: Cache size under 100MB → Will become NFR-z0jyi-cache-size
  - Category: Resource Usage.
  - Target: Total cache size including indexes <100MB.
  - Rationale: Reasonable for developer machines.

## Design Considerations

### Technical Constraints

- Must work on Windows, macOS, and Linux.
- Cannot require additional runtime dependencies.
- Must handle concurrent access (multiple Kopi processes).
- Must remain backward compatible.

### Potential Approaches

1. **Option A**: SQLite Database
   - Pros: ACID compliant, built into Rust ecosystem, excellent query performance.
   - Cons: Binary format (not human-readable).
   - Effort: Medium.

2. **Option B**: JSON Files with Index
   - Pros: Human-readable, simple implementation.
   - Cons: Poor performance for large datasets, complex concurrent access.
   - Effort: Low.

3. **Option C**: Custom Binary Format
   - Pros: Optimal performance, minimal size.
   - Cons: Complex implementation, hard to debug.
   - Effort: High.

### Architecture Impact

- New ADR needed: Cache storage format decision.
- New ADR needed: Cache invalidation strategy.
- Impacts CLI commands, configuration, and error handling.

## Risk Assessment

| Risk                           | Probability | Impact | Mitigation Strategy                       |
| ------------------------------ | ----------- | ------ | ----------------------------------------- |
| API format changes break cache | Low         | High   | Version cache schema, validate on read    |
| Cache corruption               | Low         | Medium | Checksums, atomic writes, rebuild command |
| Disk space exhaustion          | Low         | Low    | Size limits, automatic cleanup            |

## Open Questions

- [x] Should cache be shared between users? → No, security concerns.
- [ ] Support for ETag validation? → Investigate in Phase 2.
- [ ] Cache pre-warming on install? → Consider for v2.

<!-- Complex investigations should spin out into their own ADR or analysis document -->

## Recommendations

### Immediate Actions

1. Implement SQLite-based cache (Option A).
2. Add `--offline` flag to force cache-only mode.
3. Default TTL of 3600 seconds (configurable).

### Next Steps

1. [x] Create formal requirements: FR-twzx0-cache-metadata-ttl, FR-7y2x8-offline-mode, FR-0cv9r-cache-management, NFR-j3cf1-cache-performance, NFR-z0jyi-cache-size.
2. [x] Draft ADR for cache storage format (SQLite selected).
3. [ ] Create task for cache implementation.
4. [ ] Further investigation: Monitoring for cache hit rates.

### Out of Scope

- Multi-user cache sharing.
- Distributed cache.
- Custom API endpoints.
- Cache pre-warming.

## Appendix

### Meeting Notes

2024-01-12: Team agreed on SQLite approach.
2024-01-14: Product approved P0 priority.

### References

- SQLite performance benchmarks: https://sqlite.org/speed.html
- foojay.io API documentation: https://api.foojay.io/docs

### Raw Data

Benchmark results (1000 iterations):

```
Network fetch: avg=3.2s, p95=5.1s, p99=8.3s
SQLite cache: avg=0.015s, p95=0.023s, p99=0.045s
JSON cache: avg=0.234s, p95=0.567s, p99=1.234s
```

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../README.md#analysis-template-analysismd) in the templates README.

- ADRs:
  - [ADR-ygma7-http-client-selection](../../adr/ADR-ygma7-http-client-selection.md)

<!--lint enable remark-validate-links -->
