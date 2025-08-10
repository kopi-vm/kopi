# macOS JDK Bundle Structure Implementation Plan

## Overview

This document outlines the implementation plan for handling diverse JDK directory structures on macOS, particularly the application bundle format (`Contents/Home`) used by distributions like Temurin. The implementation is divided into phases that can be completed independently with context resets (`/clear`) between each phase.

## Phase 1: Core Structure Support (MVP)

**Goal**: Enable Kopi to work with all major macOS JDK distributions by implementing runtime structure detection.

### Input Materials
- **Documentation**:
  - `/docs/adr/018-macos-jdk-bundle-structure-handling.md` - ADR with structure analysis
  - `/docs/tasks/ap-bundle/design.md` - Detailed design specification

- **Source Code to Modify**:
  - `/src/archive/mod.rs` - Add structure detection
  - `/src/storage/listing.rs` - Add path resolution to InstalledJdk
  - `/src/commands/install.rs` - Update installation process
  - `/src/shim/mod.rs` - Update shim to use InstalledJdk methods
  - `/src/commands/env.rs` - Update env command

- **Reference Code**:
  - `/src/platform/mod.rs` - Platform detection utilities
  - `/src/error/mod.rs` - Error types and handling
  - `/src/storage/mod.rs` - Storage utilities

### Deliverables
1. **Source Code Changes**:
   - `detect_jdk_root()` function implemented and tested
   - `InstalledJdk::resolve_java_home()` method implemented
   - `InstalledJdk::resolve_bin_path()` method implemented
   - Updated shim using new path resolution
   - Updated env command using new path resolution

2. **Test Results**:
   - Unit tests passing for all structure types
   - Manual test: Successfully install and run Temurin@21 on macOS
   - Manual test: Successfully install and run Liberica@21 on macOS
   - Manual test: Successfully install and run Azul Zulu@21 on macOS
   - Shim execution time < 50ms confirmed

3. **Verification Commands**:
   ```bash
   # All should succeed after Phase 1
   kopi install temurin@21
   kopi use temurin@21
   java --version
   kopi env
   ```

### 1.1 Structure Detection Module
- [ ] Create `detect_jdk_root()` function in `src/archive/mod.rs`
- [ ] Implement detection algorithm:
  - [ ] Check for `bin/` at root (direct structure)
  - [ ] Check for `Contents/Home/bin/` (bundle structure)
  - [ ] Check for symlinks at root pointing to bundle (hybrid structure)
  - [ ] Return structure type and appropriate path
- [ ] Add platform conditional compilation (`#[cfg(target_os = "macos")]`)
- [ ] Add logging for detected structure type
- [ ] Create unit tests for each structure type

### 1.2 Common Path Resolution
- [ ] Add `resolve_java_home()` method to `InstalledJdk` in `src/storage/listing.rs`
  - [ ] Implement runtime detection for macOS
  - [ ] Return path directly for other platforms
  - [ ] Add debug logging for resolved paths
- [ ] Add `resolve_bin_path()` method to `InstalledJdk`
  - [ ] Call `resolve_java_home()` and append `bin`
  - [ ] Verify bin directory exists
- [ ] Add unit tests for path resolution on different platforms
- [ ] Test with mock directory structures

### 1.3 Installation Integration
- [ ] Update `extract_and_install()` in `src/commands/install.rs`
  - [ ] Call `detect_jdk_root()` after extraction
  - [ ] Move correct directory to final location
  - [ ] Log structure type at INFO level
- [ ] Update error handling for invalid structures
- [ ] Test installation with real JDK archives:
  - [ ] Temurin (bundle structure)
  - [ ] Liberica (direct structure)
  - [ ] Azul Zulu (hybrid structure)

### 1.4 Shim Enhancement
- [ ] Modify `find_jdk_installation()` in `src/shim/mod.rs`
  - [ ] Return `InstalledJdk` instance instead of just path
  - [ ] Update error types to match
- [ ] Update `build_tool_path()` to use `InstalledJdk::resolve_bin_path()`
  - [ ] Remove hardcoded `bin` path construction
  - [ ] Use resolved bin path from `InstalledJdk`
- [ ] Update shim tests for new behavior
- [ ] Verify shim works with all structure types

### 1.5 Env Command Integration
- [ ] Update `env` command in `src/commands/env.rs`
  - [ ] Use `InstalledJdk::resolve_java_home()` for JAVA_HOME
  - [ ] Remove any hardcoded path assumptions
- [ ] Test env command output for each structure type
- [ ] Verify shell integration works correctly

### 1.6 Integration Testing
- [ ] Create integration test suite for macOS structures
- [ ] Test version switching between different structure types
- [ ] Test `kopi use` with various JDK distributions
- [ ] Test `kopi current` reports correct version
- [ ] Performance testing: ensure < 50ms shim execution

---

## Phase 2: Metadata Optimization

**Goal**: Improve performance by caching structure information in metadata files.

### Input Materials
- **Documentation**:
  - `/docs/tasks/ap-bundle/design.md` - Metadata Extension specification (Component #2)
  - Phase 1 deliverables and test results

- **Source Code to Modify**:
  - `/src/storage/mod.rs` - Extend save_jdk_metadata function
  - `/src/storage/listing.rs` - Add metadata caching to InstalledJdk
  - `/src/commands/install.rs` - Save metadata during installation

- **Reference Code**:
  - `/src/models/metadata.rs` - Existing metadata structures
  - Existing `save_jdk_metadata()` implementation

### Deliverables
1. **Source Code Changes**:
   - `InstallationMetadata` struct defined and integrated
   - Extended `save_jdk_metadata()` with installation metadata
   - Metadata lazy loading in `InstalledJdk`
   - Fallback detection when metadata missing

2. **Test Results**:
   - Metadata correctly saved for new installations
   - Path resolution uses metadata (verified via logs)
   - Fallback works for existing installations without metadata
   - Performance benchmark: < 1ms with metadata, ~5ms without

3. **Verification**:
   ```bash
   # Install new JDK and verify metadata created
   kopi install temurin@24
   cat ~/.kopi/jdks/temurin-24.*/metadata.json | jq .installation_metadata
   
   # Verify performance improvement
   time ~/.kopi/shims/java --version
   ```

### 2.1 Metadata Structure Design
- [ ] Define `InstallationMetadata` struct in `src/storage/mod.rs`
  - [ ] Add `java_home_suffix` field (e.g., "Contents/Home")
  - [ ] Add `structure_type` field (bundle/direct/hybrid)
  - [ ] Add `platform` field for platform-specific info
  - [ ] Add `metadata_version` for future compatibility
- [ ] Update existing metadata structures to include installation metadata
- [ ] Add serialization/deserialization tests

### 2.2 Metadata Persistence
- [ ] Extend `save_jdk_metadata()` in `src/storage/mod.rs`
  - [ ] Accept installation metadata parameter
  - [ ] Include in saved JSON structure
  - [ ] Maintain backward compatibility
- [ ] Update installation process to save metadata
  - [ ] Create metadata after successful detection
  - [ ] Save alongside JDK installation
- [ ] Add error handling for metadata save failures

### 2.3 Metadata Loading
- [ ] Add metadata caching to `InstalledJdk`
  - [ ] Add optional metadata field to struct
  - [ ] Implement lazy loading on first access
  - [ ] Cache in memory for process lifetime
- [ ] Update `resolve_java_home()` to use cached metadata
  - [ ] Try metadata first
  - [ ] Fall back to runtime detection
  - [ ] Log when using fallback
- [ ] Update `resolve_bin_path()` to use cached metadata

### 2.4 Fallback Behavior
- [ ] Implement graceful fallback when metadata missing
  - [ ] Log warning about missing metadata
  - [ ] Perform runtime detection
  - [ ] Continue operation normally
- [ ] Handle corrupted metadata files
  - [ ] Validate JSON structure
  - [ ] Fall back on parse errors
  - [ ] Log errors for debugging

### 2.5 Performance Testing
- [ ] Create benchmark suite for path resolution
  - [ ] Measure with metadata (cache hit)
  - [ ] Measure without metadata (fallback)
  - [ ] Compare before/after implementation
- [ ] Verify shim performance improvement
- [ ] Test with various cache states

### 2.6 Migration Support
- [ ] Ensure existing installations work without metadata
- [ ] Document that metadata is created for new installations only
- [ ] Test upgrade scenarios
- [ ] Verify no breaking changes

---

## Phase 3: Testing and Documentation

**Goal**: Comprehensive testing and documentation of the implementation.

### Input Materials
- **Documentation**:
  - `/docs/adr/018-macos-jdk-bundle-structure-handling.md` - ADR to update
  - `/docs/reference.md` - User documentation to update
  - Phase 1 and 2 deliverables

- **Test Resources**:
  - JDK distribution download URLs from foojay.io
  - Existing test suites in `/tests/`
  - ADR-018 contains structure analysis for test data

- **Source Code**:
  - All code changes from Phase 1 and 2
  - Existing test infrastructure

### Deliverables
1. **Test Suites**:
   - Unit tests with >90% coverage for new code
   - Integration tests for all supported distributions
   - Performance benchmarks in `benches/`
   - CI passing on all platforms

2. **Documentation**:
   - Updated README with macOS support notes
   - Updated `/docs/reference.md` with structure details
   - ADR-018 updated with implementation results
   - Troubleshooting guide for common issues

3. **Release Artifacts**:
   - CHANGELOG.md entry
   - Release notes draft
   - Migration guide (if needed)

4. **Verification**:
   ```bash
   # All tests passing
   cargo test --quiet
   cargo test --quiet --features integration_tests
   
   # Documentation build succeeds
   cargo doc --no-deps
   
   # Clean installation test
   rm -rf ~/.kopi
   kopi install temurin@21
   java --version
   ```

### 3.1 Unit Test Coverage
- [ ] Structure detection tests with mock filesystems
- [ ] Path resolution tests for all platforms
- [ ] Metadata serialization/deserialization tests
- [ ] Error handling tests for edge cases
- [ ] Performance regression tests

### 3.2 Integration Test Suite
- [ ] Download and test real JDK distributions:
  - [ ] Temurin 11, 17, 21, 24
  - [ ] Liberica 8, 17, 21
  - [ ] Azul Zulu 8, 17, 21
  - [ ] GraalVM 17, 21
- [ ] Test installation, execution, and removal
- [ ] Test version switching scenarios
- [ ] Cross-platform compatibility tests

### 3.3 Documentation Updates
- [ ] Update user documentation:
  - [ ] Add macOS-specific notes to README
  - [ ] Document supported JDK distributions
  - [ ] Add troubleshooting section
- [ ] Update developer documentation:
  - [ ] Document structure detection algorithm
  - [ ] Explain metadata format
  - [ ] Add architecture diagrams
- [ ] Update ADR-018 with implementation results

### 3.4 Release Preparation
- [ ] Create release notes highlighting macOS improvements
- [ ] Prepare migration guide for users
- [ ] Update changelog
- [ ] Create test plan for QA

---

## Implementation Order

1. **Phase 1**: Core Structure Support
   - Implement basic functionality
   - Get macOS JDKs working
   - `/clear` after completion

2. **Phase 2**: Metadata Optimization
   - Add performance improvements
   - Implement caching
   - `/clear` after completion

3. **Phase 3**: Testing and Documentation
   - Comprehensive testing
   - Documentation updates
   - Release preparation

## Dependencies

- Existing modules:
  - `src/archive/mod.rs` for extraction logic
  - `src/storage/listing.rs` for JDK management
  - `src/shim/mod.rs` for shim implementation
  - `src/commands/install.rs` for installation process
  - `src/commands/env.rs` for environment setup

## Risks & Mitigations

1. **Risk**: Unknown JDK structure variations
   - **Mitigation**: Test with multiple distributions early
   - **Fallback**: Add support incrementally as discovered

2. **Risk**: Performance regression from structure detection
   - **Mitigation**: Implement metadata caching in Phase 2
   - **Fallback**: Optimize detection algorithm if needed

3. **Risk**: Breaking existing installations
   - **Mitigation**: Careful backward compatibility testing
   - **Fallback**: Feature flag for new behavior if needed

4. **Risk**: Platform-specific bugs
   - **Mitigation**: Extensive testing on macOS Intel and Apple Silicon
   - **Fallback**: Platform-specific workarounds if needed

## Success Metrics

- ✅ All major macOS JDK distributions work correctly
- ✅ Shim execution time < 50ms (with metadata: < 10ms)
- ✅ Zero regression on Linux/Windows platforms
- ✅ >90% code coverage for new functionality
- ✅ User-transparent operation (no manual configuration needed)

## Notes for Implementation

- Each phase is designed to be self-contained
- Use `/clear` between phases to reset context
- Commit working code at the end of each phase
- Run full test suite after each phase
- Document any deviations from the plan in commit messages