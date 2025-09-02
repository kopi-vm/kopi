# JDK Metadata Cache Optimization Implementation Plan

## Metadata
- Type: Implementation Plan
- Owner: Backend Team Lead
- Reviewers: Senior Engineers, QA Lead
- Status: Phase 2 In Progress
  <!-- Not Started: Planning complete, awaiting start | Phase X In Progress: Actively working | Blocked: External dependency | Under Review: Implementation complete | Completed: All phases done and verified -->
- Date Created: 2024-01-25

## Links
<!-- Internal project artifacts only. For external resources, see External References section -->
- Requirements: FR-0001, FR-0002, FR-0003, NFR-0001, NFR-0002
- Design: [`docs/tasks/cache-implementation/design.md`](design.md)
- Related ADRs: [ADR-015](../../adr/015-cache-storage-format.md), ADR-004, ADR-006
- Issue: #234
- PR: #256 (Phase 1), #267 (Phase 2 - WIP)

## Overview

Implementation of requirements FR-0001 (cache with TTL), FR-0002 (offline mode), and FR-0003 (cache management) to improve performance and reduce API calls to foojay.io. This plan breaks down the work into three phases aligned with the requirements.

## Success Metrics
- [x] Cache operations complete in <100ms
- [x] 80% reduction in API calls verified
- [ ] Zero cache corruption issues in 30-day production run
- [x] All existing tests pass; no regressions in search functionality

## Scope
- Goal: Implement complete caching solution with TTL management
- Non-Goals: Cache sharing between users, custom API endpoints
- Assumptions: foojay.io API remains stable during implementation
- Constraints: Must ship by end of Q1 2024

## Plan Summary
- Phases: Core Infrastructure → CLI Integration → Performance Optimization
- Timeline: 3 sprints (6 weeks total)

---

## Phase 1: Core Cache Infrastructure (FR-0001)

### Goal
Implement FR-0001: Cache JDK metadata locally with TTL

### Inputs
- Requirements: FR-0001, NFR-0001, NFR-0002
- Documentation:
  - `/docs/adr/archive/015-cache-storage-format.md` – SQLite storage decision
  - `/docs/adr/archive/004-error-handling.md` – Error types to implement
- Source Code to Modify:
  - `/src/lib.rs` – Add cache module
  - `/src/error.rs` – Add cache-specific errors
- Dependencies:
  - Internal: `src/models/` – Metadata structures
  - External crates: `sha2` – Checksum calculation

### Tasks
- [x] **Cache Module Structure**
  - [x] Create `src/cache/mod.rs`
  - [x] Define `CacheManager` struct
  - [x] Implement `MetadataCache` trait
- [x] **Storage Implementation**
  - [x] File I/O operations
  - [x] Atomic writes with temp files
  - [x] Platform-specific path handling
- [x] **Validation Logic**
  - [x] SHA256 checksum generation
  - [x] TTL expiration checks
  - [x] Corruption detection

### Deliverables
- Working cache module with unit tests
- Cache storage/retrieval functionality
- Error handling for corruption cases

### Verification
```bash
# Build and checks
cargo check
cargo fmt
cargo clippy --all-targets -- -D warnings
# Focused unit tests
cargo test --lib --quiet cache
```

### Acceptance Criteria (Phase Gate)
- Cache can store and retrieve metadata
- Checksums validate correctly
- TTL expiration works as designed

### Rollback/Fallback
- Feature flag to disable cache entirely
- Manual cache clear command available

---

## Phase 2: Offline Mode and CLI Integration (FR-0002, FR-0003)

### Goal
Implement FR-0002 (offline mode) and FR-0003 (cache management commands)

### Inputs
- Requirements: FR-0002, FR-0003
- Dependencies:
  - Phase 1: Core cache implementation (FR-0001)
  - `src/commands/` – Existing command structure
- Source Code to Modify:
  - `/src/commands/cache.rs` – New cache commands
  - `/src/commands/search.rs` – Integrate cache lookups

### Tasks
- [x] **Cache Commands (FR-0003)**
  - [x] `cache refresh` implementation
  - [x] `cache clear` implementation
  - [ ] `cache info` implementation
- [ ] **Offline Mode (FR-0002)**
  - [x] Modify search to use cache
  - [ ] Add `--no-cache` flag support
  - [ ] Add `--offline` flag for forced offline mode
- [ ] **Configuration**
  - [ ] Add cache settings to config.toml
  - [ ] Environment variable overrides

### Deliverables
- Complete cache command suite
- Cache-aware search functionality
- Configuration options

### Verification
```bash
cargo check
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet commands::cache
# Integration tests
cargo it cache_commands
```

### Acceptance Criteria (Phase Gate)
- FR-0002: Offline mode works with cached data
- FR-0003: All cache management commands functional
- NFR-0001: Cache operations under 100ms

### Rollback/Fallback
- Existing direct API calls remain as fallback
- Cache can be disabled via config

---

## Phase 3: Performance Optimization (NFR-0001, NFR-0002)

### Goal
Optimize for NFR-0001 (<100ms operations) and NFR-0002 (size under 100MB)

### Inputs
- Dependencies:
  - Phase 2: Complete cache implementation
  - Performance baseline measurements
- Source Code to Modify:
  - `/src/cache/` – Performance improvements

### Tasks
- [ ] **Performance Tuning**
  - [ ] Implement memory-mapped files for large caches
  - [ ] Add lazy loading for cache segments
  - [ ] Background refresh for expiring cache
- [ ] **Monitoring**
  - [ ] Cache hit/miss metrics
  - [ ] Performance counters
  - [ ] Debug logging improvements
- [ ] **Testing**
  - [ ] Load testing with large metadata sets
  - [ ] Concurrent access testing
  - [ ] Platform-specific testing

### Deliverables
- Optimized cache with <100ms lookups
- Monitoring and metrics
- Complete test coverage

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
# Performance benchmarks
cargo bench cache
# Load tests
cargo test --features perf-tests
```

### Acceptance Criteria (Phase Gate)
- Cache lookups consistently <100ms
- No performance degradation under load
- Metrics show 80%+ cache hit rate

---

## Testing Strategy

### Unit Tests
- Test each cache operation in isolation (FR-0001)
- Mock SQLite operations for reliability
- Test TTL expiration logic (FR-0001)

### Integration Tests
- End-to-end cache scenarios (FR-0001, FR-0002)
- Concurrent access patterns (NFR-0003)
- Offline mode functionality (FR-0002)
- Cache management commands (FR-0003)

### External API Parsing
- Captured foojay.io responses in tests
- Verify parsing with real data

### Performance & Benchmarks
- Benchmark cache operations
- Compare with direct API calls
- Monitor memory usage

---

## Platform Matrix

### Unix
- Test on Ubuntu 22.04, macOS 14
- Verify XDG_CACHE_HOME support

### Windows
- Test on Windows 11
- Verify %LOCALAPPDATA% usage

### Filesystem
- Test case-insensitive filesystems
- Verify long path support

---

## Dependencies

### External Crates
- `sha2 = "0.10"` – Checksum calculation
- `chrono = "0.4"` – Timestamp handling

### Internal Modules
- `src/models/` – Metadata structures
- `src/error/` – Error types

---

## Risks & Mitigations

1. Risk: Cache corruption during concurrent access
   - Mitigation: File locking implementation
   - Validation: Concurrent access tests
   - Fallback: Auto-recovery on corruption

2. Risk: Performance regression in search
   - Mitigation: Comprehensive benchmarking
   - Validation: A/B testing with/without cache
   - Fallback: Feature flag to disable

---

## Documentation & Change Management

### CLI/Behavior Changes
- Update `docs/reference.md` with cache commands
- Add cache section to troubleshooting guide

### ADR Impact
- Consider ADR for cache eviction strategy if needed

---

## Implementation Guidelines

### Error Handling
- Use `KopiError::CacheCorrupted` for integrity failures
- Clear error messages with recovery suggestions

### Naming & Structure
- Avoid generic names like `Manager` or `Utils`
- Use specific names like `CacheStore`, `MetadataRepository`

### Safety & Clarity
- No `unsafe` code in cache implementation
- Prefer clarity over micro-optimizations

---

## Definition of Done

- [x] `cargo check`
- [x] `cargo fmt`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib --quiet`
- [ ] Integration/perf/bench: `cargo it`, `cargo perf`, `cargo bench`
- [ ] Requirements verified: FR-0001, FR-0002, FR-0003 functional
- [ ] Performance: NFR-0001 (<100ms), NFR-0002 (<100MB) met
- [ ] `docs/reference.md` updated with cache commands
- [ ] ADRs added for significant decisions
- [x] Error messages actionable and in English
- [ ] Platform verification completed (Unix/Windows)
- [x] No `unsafe` and no vague naming (no "manager"/"util")

---

## Status Tracking

- Phase 1: Completed 2024-02-01
- Phase 2: In Progress (70% complete)
- Phase 3: Not Started

---

## External References (optional)
<!-- External standards, specifications, articles, or documentation only -->
- [Git Index Format](https://git-scm.com/docs/index-format) - Efficient cache file format reference

## Open Questions

- Should cache warm on startup? → Team → Sprint planning
- Max cache size limits? → SRE → Capacity planning

---

## Visual/UI Reference (optional)
```
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