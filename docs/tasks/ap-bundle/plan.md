# macOS JDK Bundle Structure Implementation Plan

## Overview

This document outlines the implementation plan for handling diverse JDK directory structures on macOS, particularly the application bundle format (`Contents/Home`) used by distributions like Temurin. The implementation is divided into phases that can be completed independently with context resets (`/clear`) between each phase.

## Phase 1: Structure Detection Module

**Goal**: Create the core structure detection functionality for identifying JDK directory layouts.

### Input Materials
- **Documentation**:
  - `/docs/adr/018-macos-jdk-bundle-structure-handling.md` - ADR with structure analysis
  - `/docs/tasks/ap-bundle/design.md` - Detailed design specification

- **Source Code to Modify**:
  - `/src/archive/mod.rs` - Add structure detection

- **Reference Code**:
  - `/src/platform/mod.rs` - Platform detection utilities
  - `/src/error/mod.rs` - Error types and handling

### Tasks
- [x] Create `detect_jdk_root()` function in `src/archive/mod.rs`
- [x] Implement detection algorithm:
  - [x] Check for `bin/` at root (direct structure)
  - [x] Check for `Contents/Home/bin/` (bundle structure)
  - [x] Check for symlinks at root pointing to bundle (hybrid structure)
  - [x] Return structure type and appropriate path
- [x] Add platform conditional compilation (`#[cfg(target_os = "macos")]`)
- [x] Add logging for detected structure type
- [x] **Write unit tests**:
  - [x] Test direct structure detection with mock filesystem
  - [x] Test bundle structure detection (Contents/Home)
  - [x] Test hybrid structure detection (symlinks)
  - [x] Test error cases (invalid structures)
  - [x] Test platform-specific behavior

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib archive::tests
```

---

## Phase 2: Common Path Resolution

**Goal**: Implement path resolution methods for JDK installations.

### Input Materials
- **Source Code to Modify**:
  - `/src/storage/listing.rs` - Add path resolution to InstalledJdk

- **Reference Code**:
  - `/src/storage/mod.rs` - Storage utilities
  - Phase 1 deliverables

### Tasks
- [x] Add `resolve_java_home()` method to `InstalledJdk` in `src/storage/listing.rs`
  - [x] Implement runtime detection for macOS
  - [x] Return path directly for other platforms
  - [x] Add debug logging for resolved paths
- [x] Add `resolve_bin_path()` method to `InstalledJdk`
  - [x] Call `resolve_java_home()` and append `bin`
  - [x] Verify bin directory exists
- [x] **Write unit tests**:
  - [x] Test `resolve_java_home()` for macOS bundle structure
  - [x] Test `resolve_java_home()` for direct structure
  - [x] Test `resolve_java_home()` for Linux/Windows (passthrough)
  - [x] Test `resolve_bin_path()` returns correct path
  - [x] Test error handling when bin directory missing
  - [x] Create mock directory structures for testing

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib storage::listing::tests
```

---

## Phase 3: Installation Integration

**Goal**: Update the installation process to handle different JDK structures.

### Input Materials
- **Source Code to Modify**:
  - `/src/commands/install.rs` - Update installation process

- **Dependencies**:
  - Phase 1 (Structure Detection Module)
  - Phase 2 (Common Path Resolution)

### Tasks
- [x] Update `extract_and_install()` in `src/commands/install.rs`
  - [x] Call `detect_jdk_root()` after extraction
  - [x] Move correct directory to final location
  - [x] Log structure type at INFO level
- [x] Update error handling for invalid structures
- [x] **Write unit tests**:
  - [x] Mock extraction and structure detection
  - [x] Test correct directory movement for each structure type
  - [x] Test error handling for invalid JDK structures
  - [x] Test logging output
- [ ] **Manual testing** with real JDK archives:
  - [ ] Temurin (bundle structure)
  - [ ] Liberica (direct structure)
  - [ ] Azul Zulu (hybrid structure)

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib commands::install::tests
# Manual testing
kopi install temurin@21
kopi install liberica@21
kopi install zulu@21
```

---

## Phase 4: Shim Enhancement

**Goal**: Update the shim to use the new path resolution methods.

### Input Materials
- **Source Code to Modify**:
  - `/src/shim/mod.rs` - Update shim to use InstalledJdk methods

- **Dependencies**:
  - Phase 2 (Common Path Resolution)

### Tasks
- [ ] Modify `find_jdk_installation()` in `src/shim/mod.rs`
  - [ ] Return `InstalledJdk` instance instead of just path
  - [ ] Update error types to match
- [ ] Update `build_tool_path()` to use `InstalledJdk::resolve_bin_path()`
  - [ ] Remove hardcoded `bin` path construction
  - [ ] Use resolved bin path from `InstalledJdk`
- [ ] **Write unit tests**:
  - [ ] Test `find_jdk_installation()` returns correct `InstalledJdk`
  - [ ] Test `build_tool_path()` uses resolved paths
  - [ ] Test shim execution with different structure types
  - [ ] Test error handling for missing JDK
  - [ ] Performance test: ensure < 50ms execution

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib shim::tests
# Manual testing
kopi use temurin@21
java --version
time ~/.kopi/shims/java --version  # Should be < 50ms
```

---

## Phase 5: Env Command Integration

**Goal**: Update the env command to use proper path resolution.

### Input Materials
- **Source Code to Modify**:
  - `/src/commands/env.rs` - Update env command

- **Dependencies**:
  - Phase 2 (Common Path Resolution)

### Tasks
- [ ] Update `env` command in `src/commands/env.rs`
  - [ ] Use `InstalledJdk::resolve_java_home()` for JAVA_HOME
  - [ ] Remove any hardcoded path assumptions
- [ ] **Write unit tests**:
  - [ ] Test JAVA_HOME set correctly for bundle structure
  - [ ] Test JAVA_HOME set correctly for direct structure
  - [ ] Test PATH includes correct bin directory
  - [ ] Test output format for different shells (bash, zsh, fish)
  - [ ] Test error handling when no JDK selected

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib commands::env::tests
# Manual testing
kopi env
eval "$(kopi env)"
echo $JAVA_HOME
```

---

## Phase 6: Core Functionality Integration

**Goal**: Verify all core components work together correctly.

### Dependencies
- Phases 1-5 complete with their unit tests

### Tasks
- [ ] **Integration tests** for core workflow:
  - [ ] Test full installation flow with structure detection
  - [ ] Test shim execution with resolved paths
  - [ ] Test env command with correct JAVA_HOME
  - [ ] Test version switching between different structures
- [ ] **End-to-end tests**:
  - [ ] Install Temurin (bundle) → use → run java
  - [ ] Install Liberica (direct) → switch → run java
  - [ ] Test `kopi current` reports correct version
- [ ] **Performance validation**:
  - [ ] Measure shim execution time < 50ms
  - [ ] Profile memory usage during operations
- [ ] Fix any integration issues discovered

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
# Run unit tests for phases 1-5
cargo test --lib --quiet
# Run integration tests if available
cargo test --test '*' --quiet
# Manual testing
kopi install temurin@21 && kopi use temurin@21 && java --version
```

---

## Phase 7: Metadata Structure Design

**Goal**: Design and implement the metadata structure for caching JDK information.

### Input Materials
- **Documentation**:
  - `/docs/tasks/ap-bundle/design.md` - Metadata Extension specification (Component #2)

- **Source Code to Modify**:
  - `/src/storage/mod.rs` - Define metadata structures

- **Reference Code**:
  - `/src/models/metadata.rs` - Existing metadata structures

### Tasks
- [ ] Define `InstallationMetadata` struct in `src/storage/mod.rs`
  - [ ] Add `java_home_suffix` field (e.g., "Contents/Home")
  - [ ] Add `structure_type` field (bundle/direct/hybrid)
  - [ ] Add `platform` field for platform-specific info
  - [ ] Add `metadata_version` for future compatibility
- [ ] Update existing metadata structures to include installation metadata
- [ ] **Write unit tests**:
  - [ ] Test serialization to JSON
  - [ ] Test deserialization from JSON
  - [ ] Test backward compatibility (missing fields)
  - [ ] Test forward compatibility (extra fields)
  - [ ] Test invalid JSON handling

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib storage::metadata::tests
```

---

## Phase 8: Metadata Persistence

**Goal**: Implement saving metadata during JDK installation.

### Input Materials
- **Source Code to Modify**:
  - `/src/storage/mod.rs` - Extend save_jdk_metadata function
  - `/src/commands/install.rs` - Save metadata during installation

- **Dependencies**:
  - Phase 7 (Metadata Structure Design)

### Tasks
- [ ] Extend `save_jdk_metadata()` in `src/storage/mod.rs`
  - [ ] Accept installation metadata parameter
  - [ ] Include in saved JSON structure
  - [ ] Maintain backward compatibility
- [ ] Update installation process to save metadata
  - [ ] Create metadata after successful detection
  - [ ] Save alongside JDK installation
- [ ] Add error handling for metadata save failures
- [ ] **Write unit tests**:
  - [ ] Test metadata file creation
  - [ ] Test metadata file content
  - [ ] Test error handling for write failures
  - [ ] Test atomic file operations
  - [ ] Integration test with install command

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib storage::tests
# Manual testing
kopi install temurin@24
cat ~/.kopi/jdks/temurin-24.*/metadata.json | jq .installation_metadata
```

---

## Phase 9: Metadata Loading

**Goal**: Implement lazy loading and caching of metadata.

### Input Materials
- **Source Code to Modify**:
  - `/src/storage/listing.rs` - Add metadata caching to InstalledJdk

- **Dependencies**:
  - Phase 7 (Metadata Structure Design)
  - Phase 2 (Common Path Resolution)

### Tasks
- [ ] Add metadata caching to `InstalledJdk`
  - [ ] Add optional metadata field to struct
  - [ ] Implement lazy loading on first access
  - [ ] Cache in memory for process lifetime
- [ ] Update `resolve_java_home()` to use cached metadata
  - [ ] Try metadata first
  - [ ] Fall back to runtime detection
  - [ ] Log when using fallback
- [ ] Update `resolve_bin_path()` to use cached metadata
- [ ] **Write unit tests**:
  - [ ] Test lazy loading of metadata
  - [ ] Test cache hit performance (< 1ms)
  - [ ] Test fallback when metadata missing
  - [ ] Test concurrent access to cached data
  - [ ] Test memory usage with multiple JDKs

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib storage::listing::tests
# Manual testing
RUST_LOG=debug kopi use temurin@24
# Should see logs about using metadata
```

---

## Phase 10: Fallback Behavior

**Goal**: Implement graceful fallback when metadata is missing or corrupted.

### Dependencies
- Phase 9 (Metadata Loading)

### Tasks
- [ ] Implement graceful fallback when metadata missing
  - [ ] Log warning about missing metadata
  - [ ] Perform runtime detection
  - [ ] Continue operation normally
- [ ] Handle corrupted metadata files
  - [ ] Validate JSON structure
  - [ ] Fall back on parse errors
  - [ ] Log errors for debugging
- [ ] **Write unit tests**:
  - [ ] Test with missing metadata file
  - [ ] Test with corrupted JSON
  - [ ] Test with incomplete metadata
  - [ ] Test logging output for debugging
  - [ ] Test no user-visible errors on fallback

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib storage::tests
# Manual testing - verify fallback works
rm ~/.kopi/jdks/*/metadata.json
kopi use temurin@21
java --version
```

---

## Phase 11: Performance Testing

**Goal**: Benchmark and verify performance improvements.

### Dependencies
- Phases 7-10 complete

### Tasks
- [ ] Create benchmark suite for path resolution
  - [ ] Measure with metadata (cache hit)
  - [ ] Measure without metadata (fallback)
  - [ ] Compare before/after implementation
- [ ] Verify shim performance improvement
- [ ] Test with various cache states
- [ ] **Write performance tests**:
  - [ ] Benchmark metadata loading time
  - [ ] Benchmark structure detection time
  - [ ] Test shim startup time < 50ms (< 10ms with cache)
  - [ ] Memory usage benchmarks
  - [ ] Create performance regression tests

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet
cargo bench --bench path_resolution
# Manual performance testing
time ~/.kopi/shims/java --version  # Should be < 10ms with metadata
```

---

## Phase 12: Migration Support

**Goal**: Ensure backward compatibility with existing installations.

### Dependencies
- Phases 7-11 complete

### Tasks
- [ ] Ensure existing installations work without metadata
- [ ] Document that metadata is created for new installations only
- [ ] Test upgrade scenarios
- [ ] Verify no breaking changes
- [ ] **Write migration tests**:
  - [ ] Test existing JDK installations continue working
  - [ ] Test mixed environments (with/without metadata)
  - [ ] Test upgrade from old to new version
  - [ ] Test rollback scenarios
  - [ ] Integration test with real user scenarios

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib storage::tests
# Manual testing with old installation
kopi use existing-jdk-without-metadata
java --version
```

---

## Phase 13: Test Coverage Analysis

**Goal**: Verify comprehensive test coverage and add any missing tests.

### Dependencies
- Phases 1-12 complete with their unit tests

### Tasks
- [ ] Run coverage analysis with `cargo tarpaulin`
- [ ] Identify untested code paths
- [ ] Add tests for any uncovered edge cases:
  - [ ] Race conditions in concurrent access
  - [ ] Platform-specific edge cases
  - [ ] Error recovery scenarios
- [ ] Verify all error types have tests
- [ ] Ensure >90% code coverage for new functionality
- [ ] Create test documentation

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet
# Generate and review coverage report
cargo tarpaulin --lib --out Html
# Ensure >90% coverage for new functionality
```

---

## Phase 14: Integration Test Suite

**Goal**: Test real JDK distributions end-to-end.

### Dependencies
- Phases 1-12 complete

### Tasks
- [ ] Download and test real JDK distributions:
  - [ ] Temurin 11, 17, 21, 24
  - [ ] Liberica 8, 17, 21
  - [ ] Azul Zulu 8, 17, 21
  - [ ] GraalVM 17, 21
- [ ] Test installation, execution, and removal
- [ ] Test version switching scenarios
- [ ] Cross-platform compatibility tests

### Verification
```bash
cargo test --quiet --features integration_tests
```

---

## Phase 15: Documentation Updates

**Goal**: Update all documentation to reflect the new functionality.

### Input Materials
- **Documentation to Update**:
  - `/docs/adr/018-macos-jdk-bundle-structure-handling.md` - ADR to update
  - `/docs/reference.md` - User documentation to update
  - `README.md` - Add macOS support notes

### Tasks
- [ ] Update user documentation:
  - [ ] Add macOS-specific notes to README
  - [ ] Document supported JDK distributions
  - [ ] Add troubleshooting section
- [ ] Update developer documentation:
  - [ ] Document structure detection algorithm
  - [ ] Explain metadata format
  - [ ] Add architecture diagrams
- [ ] Update ADR-018 with implementation results

### Verification
```bash
# Review documentation files manually
ls -la docs/
cat README.md | head -20
cat docs/reference.md | head -20
```

---

## Implementation Order

### Core Structure Support (Phases 1-6)
1. **Phase 1**: Structure Detection Module
2. **Phase 2**: Common Path Resolution  
3. **Phase 3**: Installation Integration
4. **Phase 4**: Shim Enhancement
5. **Phase 5**: Env Command Integration
6. **Phase 6**: Core Integration Testing

### Metadata Optimization (Phases 7-12)
7. **Phase 7**: Metadata Structure Design
8. **Phase 8**: Metadata Persistence
9. **Phase 9**: Metadata Loading
10. **Phase 10**: Fallback Behavior
11. **Phase 11**: Performance Testing
12. **Phase 12**: Migration Support

### Testing and Documentation (Phases 13-15)
13. **Phase 13**: Test Coverage Analysis
14. **Phase 14**: Integration Test Suite
15. **Phase 15**: Documentation Updates

## Dependencies

- Existing modules:
  - `src/archive/mod.rs` for extraction logic
  - `src/storage/listing.rs` for JDK management
  - `src/shim/mod.rs` for shim implementation
  - `src/commands/install.rs` for installation process
  - `src/commands/env.rs` for environment setup

## Risks & Mitigations

1. **Risk**: Unknown JDK structure variations
   - **Mitigation**: Test with multiple distributions early (Phase 1)
   - **Fallback**: Add support incrementally as discovered

2. **Risk**: Performance regression from structure detection
   - **Mitigation**: Implement metadata caching (Phases 7-12)
   - **Fallback**: Optimize detection algorithm if needed

3. **Risk**: Breaking existing installations
   - **Mitigation**: Careful backward compatibility testing (Phase 12)
   - **Fallback**: Feature flag for new behavior if needed

4. **Risk**: Platform-specific bugs
   - **Mitigation**: Extensive testing on macOS Intel and Apple Silicon (Phase 14)
   - **Fallback**: Platform-specific workarounds if needed

## Success Metrics

- ✅ All major macOS JDK distributions work correctly
- ✅ Shim execution time < 50ms (with metadata: < 10ms)
- ✅ Zero regression on Linux/Windows platforms
- ✅ >90% code coverage for new functionality
- ✅ User-transparent operation (no manual configuration needed)

## Notes for Implementation

- Each phase is designed to be self-contained
- Use `/clear` between phases to reset context if needed
- Commit working code at the end of each phase
- Run relevant tests after each phase (`cargo test --lib` for unit tests)
- Document any deviations from the plan in commit messages
- Phases within a group can be done sequentially, but groups can be parallelized if multiple developers are involved