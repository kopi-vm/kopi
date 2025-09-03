# Metadata Abstraction Implementation Plan

This work plan outlines the phased implementation of metadata abstraction in Kopi. Each phase is designed to be completed in a single session with a `/clear` command between phases to refresh context.

## Overview

The implementation follows a bottom-up approach, starting with core abstractions and progressively building toward a complete multi-source metadata system with fallback capabilities.

## Phase 1: Core Abstractions and FoojayMetadataSource

**Goal**: Establish the foundation and maintain backward compatibility

### Input

- Design documents: `02-architecture.md`, `03-lazy-loading.md`, `sources/01-foojay.md`
- Existing code: `src/api/client.rs`, `src/models/metadata.rs`
- Current tests for API client functionality

### Output

- New modules: `src/metadata/mod.rs`, `src/metadata/source.rs`, `src/metadata/foojay.rs`
- Updated `src/models/metadata.rs` with optional fields
- Tests for FoojayMetadataSource

### TODOs

1. Create `MetadataSource` trait definition
2. Add optional fields to `JdkMetadata` struct
3. Implement `FoojayMetadataSource` wrapping `ApiClient`
4. Create basic `MetadataProvider` with single source support
5. Write unit tests for FoojayMetadataSource

---

**/clear**

## Phase 2: Integration with Existing Code

**Goal**: Replace direct ApiClient usage with MetadataProvider

### Input

- Implementation from Phase 1
- Existing code using `ApiClient`: `src/cache/mod.rs`, `src/commands/*.rs`
- Integration test suite

### Output

- Updated cache module using MetadataProvider
- Updated commands using MetadataProvider
- All existing tests passing

### TODOs

1. Update `src/cache/mod.rs` to use MetadataProvider
2. Update install command to use MetadataProvider
3. Update cache command to use MetadataProvider
4. Update list command to use MetadataProvider
5. Run full test suite and fix any issues
6. Performance comparison with direct API usage

---

**/clear**

## Phase 3: Metadata Generator Tool

**Goal**: Create tool to generate metadata files from foojay API

### Input

- Design document: `04-metadata-generator.md`
- FoojayMetadataSource from Phase 1
- Platform detection utilities

### Output

- New binary: `src/bin/kopi-metadata-gen.rs`
- Generated metadata files for testing
- Release script: `scripts/generate-metadata.sh`

### TODOs

1. Create CLI structure with clap
2. Implement generate command
3. Add platform filtering logic
4. Implement index.json generation
5. Add progress reporting
6. Implement validate command
7. Add configuration file support (toml) for default settings
8. Implement diff reporting for update command
9. Create release script for CI/CD
10. Test with real foojay API
11. Generate test metadata archives

---

**/clear**

## Phase 4: HttpMetadataSource Implementation

**Goal**: Implement primary metadata source for production

### Input

- Design document: `sources/02-http-web.md`
- Test metadata generated in Phase 3
- HTTP client library (attohttpc)

### Output

- New module: `src/metadata/http.rs`
- Tests with mock HTTP server

### TODOs

1. Implement HttpMetadataSource
2. Add index.json parsing
3. Implement platform filtering
4. Add response caching logic
5. Handle HTTP errors gracefully
6. Write tests with mock server
7. Test with real GitHub Pages URL

---

**/clear**

## Phase 5: LocalDirectorySource Implementation

**Goal**: Implement directory-based metadata source

### Input

- Design document: `sources/03-local-directory.md`
- Test metadata archives from Phase 3
- File system utilities

### Output

- New module: `src/metadata/local.rs`
- Tests with test directories

### TODOs

1. Implement LocalDirectorySource
2. Add directory structure validation
3. Implement platform filtering (reuse logic)
4. Handle missing/corrupt files gracefully
5. Write tests with test data
6. Test with extracted metadata

_Note: This source will read from any configured directory. Future installer integration will use this for bundled metadata._

---

**/clear**

## Phase 6: Multi-Source Provider and Fallback

**Goal**: Complete the metadata provider with fallback logic

### Input

- All metadata sources from previous phases
- Design documents: `02-architecture.md`, `06-configuration.md`
- Configuration system

### Output

- Enhanced `src/metadata/provider.rs`
- Updated configuration structures
- Fallback tests

### TODOs

1. Enhance MetadataProvider for multiple sources
2. Implement fallback logic
3. Add configuration parsing for sources
4. Implement MetadataResolver for lazy loading
5. Add source health checking
6. Write comprehensive fallback tests
7. Test various failure scenarios

---

**/clear**

## Phase 7: Final Integration and Testing

**Goal**: Ensure everything works together seamlessly

### Input

- All implementations from previous phases
- Full test suite
- Performance benchmarks

### Output

- Complete working system
- Performance report
- Migration guide

### TODOs

1. End-to-end testing of all scenarios
2. Performance testing vs old implementation
3. Load testing with concurrent requests
4. Test in various network conditions
5. Create migration guide for users
6. Update main documentation
7. Create troubleshooting guide
8. Final code review and cleanup

## Success Criteria

- All existing functionality continues to work
- HTTP source provides faster metadata access than foojay API
- Fallback to local cache works transparently on network failure
- No performance regression for common operations
- Clear documentation for configuration and troubleshooting
- System ready for future installer integration

## Risk Mitigation

- Each phase maintains backward compatibility
- Extensive testing at each phase
- Gradual rollout with feature flags if needed
- Ability to revert to direct API usage if issues arise

## Future Work

### Installer Integration

Once the metadata system is proven in production, integrate with installers:

- Bundle metadata archives with installer packages
- Extract bundled metadata during installation to `${KOPI_HOME}/bundled-metadata/`
- Ensure proper offline capability from first install
- Test upgrade scenarios with bundled metadata
- See design document: `07-installer-bundling.md`

This work is deferred to ensure the core metadata system is stable before modifying installer processes.
