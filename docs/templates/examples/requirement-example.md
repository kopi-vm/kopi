# FR-twzx0-cache-metadata-ttl: Cache JDK Metadata Locally with TTL

## Metadata

- Type: Functional Requirement
- Status: Implemented
  <!-- Proposed: Under discussion | Accepted: Approved for implementation | Implemented: Code complete | Verified: Tests passing | Deprecated: No longer applicable -->

## Links

<!-- Internal project artifacts only. Replace or remove bullets as appropriate. -->

- Related Analyses:
  - [AN-21-cache-optimization](../../analysis/AN-21-cache-optimization.md)
- Prerequisite Requirements:
  - N/A – First requirement in caching track
- Dependent Requirements:
  - [FR-7y2x8-offline-mode](../../requirements/FR-7y2x8-offline-mode.md)
- Related ADRs:
  - [ADR-bw6wd-cache-storage-format](../../adr/ADR-bw6wd-cache-storage-format.md)
- Related Tasks:
  - [T-df1ny-cache-implementation](../../tasks/T-df1ny-cache-implementation/README.md)

## Requirement Statement

The system shall cache foojay.io metadata locally with a configurable Time-To-Live (TTL), defaulting to 3600 seconds, to reduce redundant network calls and improve response times.

## Rationale

Telemetry shows 78% of API requests retrieve identical metadata within a 24-hour window. Each network call introduces 3-5 seconds of latency, while cached responses complete in <100ms. Local caching drastically improves user experience and lowers foojay.io load.

## User Story (if applicable)

As a Kopi user, I want metadata to be cached locally so that repeated searches run quickly even when I have intermittent connectivity.

## Acceptance Criteria

- [x] Metadata persists to `~/.kopi/cache/metadata.db` (Unix) or `%LOCALAPPDATA%\kopi\cache\metadata.db` (Windows).
- [x] Default TTL is 3600 seconds and configurable via `cache.ttl_seconds` in `config.toml`.
- [x] Cache hit rate exceeds 90% for repeated queries within the TTL window.
- [x] Expired entries automatically refresh on the next access.
- [x] Cache can be cleared via `kopi cache clear`.
- [x] Concurrent access across multiple Kopi processes remains safe.
- [x] Cache respects ETag headers when provided by foojay.io.

## Technical Details (if applicable)

### Functional Requirement Details

- Cache entries store payload, checksum, TTL expiration timestamp, and source ETag.
- Fresh entries are served directly; stale entries trigger a network refresh with fallback to stale data if offline.
- Configuration surface:

```toml
[cache]
enabled = true
ttl_seconds = 3600
max_size_mb = 100
```

### Non-Functional Requirement Details

- Performance: Cache hit latency ≤100ms at P95, measured via integration benchmarks.
- Reliability: Cache recovers from corruption by rebuilding after checksum failure.
- Compatibility: Works on Windows, macOS, and Linux using platform-specific paths.

## Platform Considerations

### Unix

Cache stored at `~/.kopi/cache/metadata.db` with permissions set to `0o600`.

### Windows

Cache stored at `%LOCALAPPDATA%\kopi\cache\metadata.db` with user-only access control.

### Cross-Platform

Cache keys normalize casing and path separators to avoid duplicates on case-insensitive filesystems.

## Risks & Mitigation

| Risk                  | Impact | Likelihood | Mitigation                            | Validation                          |
| --------------------- | ------ | ---------- | ------------------------------------- | ----------------------------------- |
| Cache corruption      | High   | Low        | Atomic writes with temp files         | Integration tests simulate crashes  |
| Disk space exhaustion | Medium | Low        | Enforce 100MB limit and cleanup tasks | Monitor cache size during testing   |
| Serving stale data    | Low    | Medium     | TTL enforcement and ETag validation   | Automated tests cover TTL rollover  |

## Implementation Notes

- Use `rusqlite` with WAL mode for concurrent reads.
- Emit cache hit/miss metrics at `DEBUG` level to aid troubleshooting.
- Document manual cache clear steps in troubleshooting guides.

## External References

- [SQLite Write-Ahead Logging](https://sqlite.org/wal.html) - Concurrency strategy.
- [HTTP ETag](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/ETag) - Revalidation mechanism.

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../README.md#individual-requirement-template-requirementsmd) in the templates README.

- Tasks:
  - [T-df1ny-cache-implementation](../../tasks/T-df1ny-cache-implementation/README.md)

<!--lint enable remark-validate-links -->
