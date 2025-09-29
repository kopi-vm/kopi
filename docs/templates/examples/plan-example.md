# JDK Metadata Cache Optimization Implementation Plan

## Metadata

- Type: Implementation Plan
- Status: Phase 2 In Progress
  <!-- Not Started: Planning complete, awaiting start | Phase X In Progress: Actively working | Blocked: External dependency | Under Review: Implementation complete | Completed: All phases done and verified -->

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
  - [ADR-6vgm3-progress-indicators](../../adr/ADR-6vgm3-progress-indicators.md)

## Overview

Deliver the caching subsystem required to meet performance, offline, and management capabilities while reducing foojay.io traffic by more than 80%.

## Success Metrics

- [x] Cache operations complete in <100ms (95th percentile).
- [x] 80% reduction in API calls observed via telemetry.
- [ ] Zero cache corruption incidents across a 30-day soak test.
- [x] All existing tests pass; no regressions in search functionality.

## Scope

- Goal: Provide resilient, TTL-based metadata caching with CLI controls.
- Non-Goals: Multi-user cache sharing, distributed cache layers.
- Assumptions: foojay.io API contract remains stable throughout execution.
- Constraints: Must land before end of Q1 2025 with no new external services.

## Plan Summary

- Phases: Core Storage → CLI + Offline Integration → Performance Hardening.
- Timeline (optional): Target completion in 3 sprints (6 weeks).

---

## Phase 1: Core Cache Infrastructure (FR-twzx0-cache-metadata-ttl)

### Goal

Implement persistent storage, TTL tracking, and checksum validation for cached metadata.

### Inputs

- Documentation:
  - `/docs/adr/ADR-bw6wd-cache-storage-format.md` – SQLite storage decision.
  - `/docs/adr/ADR-efx08-error-handling.md` – Error taxonomy and exit codes.
- Source Code to Modify:
  - `/src/cache/mod.rs` – New cache module entry point.
  - `/src/error.rs` – Introduce cache-specific error variants.
- Dependencies:
  - Internal: `src/models/` – Metadata representations.
  - External crates: `sha2` – Checksum calculation; `chrono` – Timestamp handling.

### Tasks

- [x] **Cache Module Structure**
  - [x] Scaffold `CacheOrchestrator` and submodules.
  - [x] Define traits for storage adapters.
- [x] **Storage Implementation**
  - [x] Create SQLite schema and migrations.
  - [x] Implement atomic writes with temp files.
- [x] **Validation Logic**
  - [x] TTL comparison helpers.
  - [x] SHA-256 checksum verification.

### Deliverables

- SQLite-backed cache capable of storing and loading metadata blobs with TTL metadata.

### Verification

```bash
cargo check
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet cache
```

### Acceptance Criteria (Phase Gate)

- Cache persists metadata to disk and retrieves it within TTL windows.
- Corruption detection triggers `KopiError::CacheCorrupted` with actionable context.

### Rollback/Fallback

- Feature flag `cache.enabled=false` disables the system entirely.
- Manual cleanup script removes cache directory if corruption occurs.

---

## Phase 2: CLI Integration and Offline Mode (FR-7y2x8-offline-mode, FR-0cv9r-cache-management)

### Phase 2 Goal

Expose cache management commands and enable offline-first execution paths.

### Phase 2 Inputs

- Dependencies:
  - Phase 1 artifacts must be available in `main`.
  - CLI command registry located under `src/commands/`.
- Source Code to Modify:
  - `/src/commands/cache.rs` – New `kopi cache` subcommands.
  - `/src/commands/search.rs` – Integrate cache lookups.

### Phase 2 Tasks

- [ ] **Cache CLI**
  - [x] Implement `cache info` output with hit/miss statistics.
  - [ ] Implement `cache clear` to remove persisted entries.
- [ ] **Offline Execution**
  - [x] Respect `--offline` flag across search/install flows.
  - [ ] Add fallback messaging when fresh data is unavailable.

### Phase 2 Deliverables

- End-to-end CLI interactions (`info`, `refresh`, `clear`) and offline command support.

### Phase 2 Verification

```bash
cargo check
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet commands::cache
cargo test --test cache_integration
```

### Phase 2 Acceptance Criteria

- CLI commands manipulate cache state as expected.
- Offline flag prevents network calls and surfaces clear messaging.

### Phase 2 Rollback/Fallback

- Hide cache subcommands behind feature flag if defects appear.
- Provide troubleshooting guide to manually delete cache directory.

---

## Phase 3: Performance Hardening and Telemetry

### Phase 3 Goal

Ensure cache meets non-functional requirements and add observability.

### Phase 3 Inputs

- Dependencies:
  - Phase 2 functionality merged and available for benchmarking.
- Source Code to Modify:
  - `/benches/cache_lookup.rs` – Benchmark harness.
  - `/src/telemetry/` – Hook cache metrics.

### Phase 3 Tasks

- [ ] **Benchmarking**
  - [ ] Capture baseline and optimized lookup timings.
  - [ ] Validate disk usage under repeated refresh cycles.
- [ ] **Telemetry Hooks**
  - [ ] Emit hit/miss counters via existing logging framework.
  - [ ] Document manual verification steps.

### Phase 3 Deliverables

- Benchmark results published in task notes.
- Logging output demonstrating cache health indicators.

### Phase 3 Verification

```bash
cargo bench cache_lookup
cargo test --lib --quiet telemetry
```

### Phase 3 Acceptance Criteria

- Meets NFR-j3cf1-cache-performance and NFR-z0jyi-cache-size targets.
- Provides observable metrics for ongoing monitoring.

### Phase 3 Rollback/Fallback

- Disable telemetry hooks if they introduce regressions; keep core cache active.

---

## Platform Matrix (if applicable)

### Unix

- Verify cache directory honors `$XDG_DATA_HOME` when set.
- Ensure file permissions default to `0o600`.

### Windows

- Validate `%LOCALAPPDATA%` paths and long path handling.
- Confirm file locking prevents concurrent corruption.

### Filesystem

- Test on case-insensitive filesystems and with long paths.

---

## Dependencies

### External Crates

- `rusqlite` – SQLite access.
- `sha2` – Digest computation.

### Internal Modules

- `src/models/` – Metadata serialization.
- `src/telemetry/` – Metrics plumbing.

---

## Risks & Mitigations

1. Risk: Cache corruption during concurrent writes.
   - Mitigation: Use transactional writes and file locking.
   - Validation: Stress tests with concurrent commands.
   - Fallback: Auto-rebuild cache on corruption detection.

2. Risk: Missed TTL refresh leading to stale data.
   - Mitigation: Include TTL checks before every cache read.
   - Validation: CLI integration tests enforce expected refresh behavior.
   - Fallback: Provide `kopi cache refresh` guidance in troubleshooting docs.

---

## Documentation & Change Management

### CLI/Behavior Changes

- Update `docs/reference.md` with new cache commands and flags.
- Provide troubleshooting entry for stale cache scenarios.

### ADR Impact

- Follow up with eviction policy ADR if future iterations require LRU behavior.

---

## Implementation Guidelines

### Error Handling

- Emit precise `KopiError` variants with remediation guidance; avoid generic failures.

### Naming & Structure

- Prefer descriptive component names like `CacheOrchestrator`; avoid vague labels such as `Manager` or `Util`.

### Safety & Clarity

- No `unsafe` blocks; favor readability over premature optimization.

---

## Definition of Done

- [x] `cargo check`
- [x] `cargo fmt`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib --quiet`
- [ ] Integration/perf/bench: `cargo it`, `cargo perf`, `cargo bench`
- [ ] Requirements verified: FR-twzx0, FR-7y2x8, FR-0cv9r, NFR-j3cf1, NFR-z0jyi
- [ ] Performance targets met (<100ms hits, <100MB storage)
- [ ] `docs/reference.md` updated for user-facing changes
- [ ] ADRs written for any additional design decisions
- [x] Error messages actionable and in English
- [ ] Platform verification completed (Unix/Windows)
- [x] No `unsafe` and no vague naming

---

## Status Tracking

- Phase 1: Completed 2025-01-31
- Phase 2: In progress (70% complete)
- Phase 3: Not started

---

## External References (optional)

- [Git Index Format](https://git-scm.com/docs/index-format) - Efficient cache file format reference.

## Open Questions

- [ ] Should cache warming run at startup? → Next step: Coordinate downstream task docs/tasks/T-kopi-cache-warm/README.md before sprint planning.
- [ ] Do we need configurable TTL per namespace? → Method: SRE to evaluate during capacity planning.

<!-- Complex investigations should spin out into their own ADR or analysis document -->

---

## Visual/UI Reference (optional)

```text
$ kopi cache info
Cache Status: Valid
Location: /home/user/.kopi/cache/
Size: 1.2 MB (145 entries)
Age: 2 hours 15 minutes
TTL: 6 hours (3h 45m remaining)
Hit Rate: 87% (261/300 requests)
```

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../README.md#plan-template-planmd) in the templates README.
