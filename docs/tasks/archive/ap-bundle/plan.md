# macOS JDK Bundle Structure Implementation Plan

## Overview

This document outlines the implementation plan for handling diverse JDK directory structures on macOS, particularly the application bundle format (`Contents/Home`) used by distributions like Temurin. The implementation is divided into phases that can be completed independently with context resets (`/clear`) between each phase.

**Current Status**: Phases 1-5 and 7-14 completed ✅. Phase 6 (Core Functionality Integration) and Phase 15 (Documentation Updates) are pending.

## Phase 1: Structure Detection Module

**Goal**: Create the core structure detection functionality for identifying JDK directory layouts.

### Input Materials
- **Documentation**:
  - `/docs/adr/archive/018-macos-jdk-bundle-structure-handling.md` - ADR with structure analysis
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
- [x] **Manual testing** with real JDK archives:
  - [x] Temurin (bundle structure) - Verify that installation_metadata is actually saved
  - [x] Liberica (direct structure) - Verify saved with java_home_suffix: ""
  - [x] Azul Zulu (hybrid structure) - Verify saved with java_home_suffix: "Contents/Home"

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
- [x] Modify `find_jdk_installation()` in `src/shim/mod.rs`
  - [x] Return `InstalledJdk` instance instead of just path
  - [x] Update error types to match
- [x] Update `build_tool_path()` to use `InstalledJdk::resolve_bin_path()`
  - [x] Remove hardcoded `bin` path construction
  - [x] Use resolved bin path from `InstalledJdk`
- [x] **Write unit tests**:
  - [x] Test `find_jdk_installation()` returns correct `InstalledJdk`
  - [x] Test `build_tool_path()` uses resolved paths
  - [x] Test shim execution with different structure types
  - [x] Test error handling for missing JDK
  - [x] Performance test: ensure < 50ms execution

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
- [x] Update `env` command in `src/commands/env.rs`
  - [x] Use `InstalledJdk::resolve_java_home()` for JAVA_HOME
  - [x] Remove any hardcoded path assumptions
- [x] **Write unit tests**:
  - [x] Test JAVA_HOME set correctly for bundle structure
  - [x] Test JAVA_HOME set correctly for direct structure
  - [x] Test PATH includes correct bin directory
  - [x] Test output format for different shells (bash, zsh, fish)
  - [x] Test error handling when no JDK selected

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
- [x] Define `InstallationMetadata` struct in `src/storage/mod.rs`
  - [x] Add `java_home_suffix` field (e.g., "Contents/Home")
  - [x] Add `structure_type` field (bundle/direct/hybrid)
  - [x] Add `platform` field for platform-specific info
  - [x] Add `metadata_version` for future compatibility
- [x] Update existing metadata structures to include installation metadata
- [x] **Write unit tests**:
  - [x] Test serialization to JSON
  - [x] Test deserialization from JSON
  - [x] Test backward compatibility (missing fields)
  - [x] Test forward compatibility (extra fields)
  - [x] Test invalid JSON handling

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
- [x] Extend `save_jdk_metadata()` in `src/storage/mod.rs`
  - [x] Accept installation metadata parameter
  - [x] Include in saved JSON structure
  - [x] Maintain backward compatibility
- [x] Update installation process to save metadata
  - [x] Create metadata after successful detection
  - [x] Save alongside JDK installation
- [x] Add error handling for metadata save failures
- [x] **Write unit tests**:
  - [x] Test metadata file creation
  - [x] Test metadata file content
  - [x] Test error handling for write failures
  - [x] Test atomic file operations
  - [x] Integration test with install command
- [x] **Manual testing** with real JDK archives:
  - [x] Temurin 22 (bundle) - installation_metadata.structure_type: "bundle"
  - [x] Liberica 21 (direct) - installation_metadata.structure_type: "direct"
  - [x] Azul Zulu 21 (hybrid) - installation_metadata.structure_type: "hybrid"

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib storage::tests
# Manual testing
kopi install temurin@24
# Metadata file will be created as ~/.kopi/jdks/temurin-24.0.2.meta.json (version may vary)
ls ~/.kopi/jdks/temurin-24*.meta.json
cat ~/.kopi/jdks/temurin-24*.meta.json | jq .installation_metadata
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
- [x] Add metadata caching to `InstalledJdk`
  - [x] Add optional metadata field to struct
  - [x] Implement lazy loading on first access
  - [x] Cache in memory for process lifetime
- [x] Update `resolve_java_home()` to use cached metadata
  - [x] Try metadata first
  - [x] Fall back to runtime detection
  - [x] Log when using fallback
- [x] Update `resolve_bin_path()` to use cached metadata
- [x] **Write unit tests**:
  - [x] Test lazy loading of metadata
  - [x] Test cache hit performance (< 1ms)
  - [x] Test fallback when metadata missing
  - [x] Test concurrent access to cached data
  - [x] Test memory usage with multiple JDKs

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
- [x] Implement graceful fallback when metadata missing
  - [x] Log warning about missing metadata
  - [x] Perform runtime detection
  - [x] Continue operation normally
- [x] Handle corrupted metadata files
  - [x] Validate JSON structure
  - [x] Fall back on parse errors
  - [x] Log errors for debugging
- [x] **Write unit tests**:
  - [x] Test with missing metadata file
  - [x] Test with corrupted JSON
  - [x] Test with incomplete metadata
  - [x] Test logging output for debugging
  - [x] Test no user-visible errors on fallback

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib storage::tests
# Manual testing - verify fallback works
rm ~/.kopi/jdks/*.meta.json
kopi use temurin@21
java --version
```

---

## Phase 11: Performance Testing

**Goal**: Benchmark and verify performance improvements.

### Dependencies
- Phases 7-10 complete

### Tasks
- [x] Create benchmark suite for path resolution
  - [x] Measure with metadata (cache hit)
  - [x] Measure without metadata (fallback)
  - [x] Compare before/after implementation
- [x] Verify shim performance improvement
- [x] Test with various cache states
- [x] **Write performance tests**:
  - [x] Benchmark metadata loading time
  - [x] Benchmark structure detection time
  - [x] Test shim startup time < 50ms (< 10ms with cache)
  - [x] Memory usage benchmarks
  - [x] Create performance regression tests

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
- [x] Ensure existing installations work without metadata
- [x] Document that metadata is created for new installations only
- [x] Test upgrade scenarios
- [x] Verify no breaking changes
- [x] **Write migration tests**:
  - [x] Test existing JDK installations continue working
  - [x] Test mixed environments (with/without metadata)
  - [x] Test upgrade from old to new version
  - [x] Test rollback scenarios
  - [x] Integration test with real user scenarios

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

## Phase 13: Test Coverage Analysis ✅

**Goal**: Verify comprehensive test coverage and add any missing tests.

### Dependencies
- Phases 1-12 complete with their unit tests

### Tasks
- [x] Run coverage analysis with `cargo llvm-cov` (switched from tarpaulin due to environment variable issues)
- [x] Identify untested code paths
- [x] Add tests for any uncovered edge cases:
  - [x] Race conditions in concurrent access (identified RefCell thread-safety issue)
  - [x] Platform-specific edge cases
  - [x] Error recovery scenarios
- [x] Verify all error types have tests (30+ error types covered)
- [x] Ensure >90% code coverage for new functionality
- [x] Create test documentation

### Results
- **Overall Project Coverage**: 69.73% line coverage
- **New Functionality Coverage** (all exceeded 90% target):
  - `error/tests.rs`: 99.70%
  - `storage/listing.rs`: 93.02%
  - `archive/mod.rs`: 90.30%
- **Key Finding**: Thread-safety issue with `RefCell` in `InstalledJdk` (needs `RwLock` or `OnceCell`)
- **Documentation**: Created at `/docs/tasks/ap-bundle/phase-13-test-documentation.md`

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet
# Generate and review coverage report
cargo llvm-cov --lib --html
cargo llvm-cov --lib --summary-only
# HTML report generated in target/llvm-cov/html/
```

---

## Phase 14: Integration Test Suite ✅

**Goal**: Test real JDK distributions end-to-end.

### Dependencies
- Phases 1-12 complete

### Tasks
- [x] Download and test real JDK distributions:
  - [x] Temurin 11, 17, 21, 24
  - [x] Liberica 8, 17, 21
  - [x] Azul Zulu 8, 17, 21
  - [x] GraalVM 17, 21
- [x] Test installation, execution, and removal
- [x] Test version switching scenarios
- [x] Cross-platform compatibility tests
- [x] Performance testing (shim execution < 50ms)

### Implementation Details
- Created comprehensive integration test suite in `/tests/jdk_distributions_integration.rs`
- Tests cover all major JDK distributions with multiple versions
- Includes tests for:
  - Installation and verification of each JDK
  - Execution of Java commands through shims
  - Version switching between different distributions
  - Uninstallation of JDKs
  - Environment variable setup (JAVA_HOME) for different structures
  - Performance validation of shim execution
- All tests use isolated test environments via `TestHomeGuard`
- Tests are marked with `#[cfg_attr(not(feature = "integration_tests"), ignore)]` to run only when explicitly requested

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet
cargo test --quiet --features integration_tests
```

---

## Phase 15: Documentation Updates ✅

**Goal**: Update all documentation to reflect the new functionality.

### Input Materials
- **Documentation to Update**:
  - `/docs/adr/archive/018-macos-jdk-bundle-structure-handling.md` - ADR to update
  - `/docs/reference.md` - User documentation to update
  - `README.md` - Add macOS support notes

### Tasks
- [x] Update user documentation:
  - [x] Add macOS-specific notes to README
  - [x] Document supported JDK distributions
  - [x] Add troubleshooting section
- [x] Update developer documentation:
  - [x] Document structure detection algorithm
  - [x] Explain metadata format
  - [x] Add architecture diagrams
- [x] Update ADR-018 with implementation results

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
13. **Phase 13**: Test Coverage Analysis ✅
14. **Phase 14**: Integration Test Suite ✅
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

- ✅ All major macOS JDK distributions work correctly (Temurin, Liberica, Zulu tested)
- ✅ Shim execution time < 50ms (with metadata: < 10ms achieved)
- ✅ Zero regression on Linux/Windows platforms
- ✅ >90% code coverage for new functionality (achieved in Phase 13)
- ✅ User-transparent operation (no manual configuration needed)

## Notes for Implementation

- Each phase is designed to be self-contained
- Use `/clear` between phases to reset context if needed
- Commit working code at the end of each phase
- Run relevant tests after each phase (`cargo test --lib` for unit tests)
- Document any deviations from the plan in commit messages
- Phases within a group can be done sequentially, but groups can be parallelized if multiple developers are involved