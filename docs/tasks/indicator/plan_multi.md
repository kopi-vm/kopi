# Multi-Progress Support Implementation Plan

## Overview

This document outlines the implementation plan for adding multi-progress bar support to Kopi's ProgressIndicator system. The implementation focuses on providing nested progress bars for operations with clear parent-child relationships, particularly for download operations and cache refresh from different sources.

**Current Status**: Phase 2 Completed

## Phase 1: Core Infrastructure - Trait and ALL Implementations Update ✅

**Goal**: Update the ProgressIndicator trait and ALL implementations with minimal changes to maintain compilation.

### Input Materials
- **Documentation**:
  - `/docs/tasks/indicator/design_multi.md` - Design specification
  
- **Source Code to Modify**:
  - `/src/indicator/mod.rs` - ProgressIndicator trait definition
  - `/src/indicator/silent.rs` - SilentProgress implementation
  - `/src/indicator/simple.rs` - SimpleProgress implementation
  - `/src/indicator/indicatif.rs` - IndicatifProgress implementation

### Tasks
- [x] **Update ProgressIndicator trait**:
  - [x] Add `fn create_child(&mut self) -> Box<dyn ProgressIndicator>` method
  - [x] Update trait documentation
  - [x] Ensure Send trait bound remains
- [x] **Minimal implementation for ALL types**:
  - [x] SilentProgress: `Box::new(SilentProgress)`
  - [x] SimpleProgress: `Box::new(SilentProgress)` with `// TODO: Phase 2` comment
  - [x] IndicatifProgress: `Box::new(SilentProgress)` with `// TODO: Phase 3` comment
- [x] **Ensure compilation**:
  - [x] All implementations compile
  - [x] All existing tests pass

### Example Implementation
Each implementation gets a minimal stub that maintains functionality:
- SilentProgress returns another SilentProgress (final implementation)
- SimpleProgress temporarily returns SilentProgress
- IndicatifProgress temporarily returns SilentProgress

### Deliverables
- Updated trait with new method signature
- All three implementations with minimal `create_child()` method
- Fully compilable and testable codebase

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo build --lib
cargo test --lib indicator
```

---

## Phase 2: SimpleProgress Final Implementation ✅

**Goal**: Finalize `create_child()` for SimpleProgress with appropriate behavior.

### Input Materials
- **Dependencies**:
  - Phase 1 (All implementations compilable)

- **Source Code to Modify**:
  - `/src/indicator/simple.rs` - SimpleProgress implementation

### Tasks
- [x] **Finalize SimpleProgress implementation**:
  - [x] Keep `create_child()` returning `Box::new(SilentProgress)`
  - [x] Remove `// TODO: Phase 2` comment
  - [x] Add documentation explaining why children are silent in CI environments
  - [x] Add tests for child creation behavior

### Deliverables
- SimpleProgress with finalized `create_child()` behavior
- Documentation explaining the design choice
- Unit tests verifying silent children

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib indicator::simple
```

---

## Phase 3: IndicatifProgress MultiProgress Implementation

**Goal**: Implement full MultiProgress support in IndicatifProgress for nested progress bars.

### Input Materials
- **Dependencies**:
  - Phase 1 (Trait updated)
  - `indicatif` crate with MultiProgress support

- **Source Code to Modify**:
  - `/src/indicator/indicatif.rs` - IndicatifProgress implementation

### Tasks
- [ ] **Refactor IndicatifProgress structure**:
  - [ ] Add `multi_progress: Arc<MultiProgress>` field
  - [ ] Update `new()` to create MultiProgress instance
  - [ ] Add `new_with_parent()` for child instances
- [ ] **Implement create_child()**:
  - [ ] Create child IndicatifProgress sharing parent's MultiProgress
  - [ ] Register child bar with parent's MultiProgress
  - [ ] Handle proper bar positioning
- [ ] **Update existing methods**:
  - [ ] Modify `start()` to work with MultiProgress
  - [ ] Ensure `complete()` properly cleans up bars
  - [ ] Handle terminal resizing gracefully
- [ ] **Add tests**:
  - [ ] Test parent-child bar creation
  - [ ] Test multiple children
  - [ ] Test cleanup on completion
  - [ ] Test nested progress depth
  - [ ] Test child with error handling
  - [ ] Test child spinner without total

### Deliverables
- IndicatifProgress with full MultiProgress support
- Proper visual nesting of progress bars
- Comprehensive tests for multi-bar scenarios

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib indicator::indicatif
# Manual visual test
cargo run --example progress_nested # Create example if needed
```

---

## Phase 4: Download Module Integration ✅

**Goal**: Update download module to support child progress indicators.

### Input Materials
- **Dependencies**:
  - Phases 1-3 (All ProgressIndicator implementations ready)

- **Source Code to Modify**:
  - `/src/download/mod.rs` - Download functions
  - `/src/download/progress.rs` - DownloadProgressAdapter

### Tasks
- [x] **Analyze current download progress**:
  - [x] Identify where progress is created and used
  - [x] Determine Content-Length retrieval points
- [x] **Update download functions**:
  - [x] Add parent progress parameter where needed
  - [x] Check Content-Length for 10MB threshold
  - [x] Create child progress when appropriate
- [x] **Update DownloadProgressAdapter**:
  - [x] Support being created as a child
  - [x] Maintain backward compatibility
- [x] **Handle edge cases**:
  - [x] Unknown Content-Length (no child progress)
  - [x] Small files < 10MB (no child progress)
  - [x] Network errors during download

### Deliverables
- Download module with conditional child progress support
- 10MB threshold implementation
- Backward compatible API

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib download
```

---

## Phase 5: Install Command - Download Progress Integration

**Goal**: Integrate child progress bars for download operations in the install command.

### Input Materials
- **Dependencies**:
  - Phases 1-4 (Download module ready)
  
- **Source Code to Modify**:
  - `/src/commands/install.rs` - Install command implementation

### Tasks
- [ ] **Identify download locations**:
  - [ ] Find where `download_jdk()` is called
  - [ ] Locate where `no_progress=true` is forced
- [ ] **Remove forced suppression**:
  - [ ] Remove `no_progress=true` for downloads
  - [ ] Pass actual progress indicator
- [ ] **Implement threshold logic**:
  - [ ] Check package size before download
  - [ ] Create child progress for files >= 10MB
  - [ ] Update parent message for files < 10MB
- [ ] **Handle cache refresh**:
  - [ ] When using Foojay API, create child progress
  - [ ] Show package processing count
- [ ] **Test various scenarios**:
  - [ ] Large JDK download (> 10MB)
  - [ ] Small tool download (< 10MB)
  - [ ] Unknown size download
  - [ ] Cache refresh during install

### Deliverables
- Install command with child progress for large downloads
- Proper parent message updates for small downloads
- Cache refresh progress when needed

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib commands::install
# Manual testing
cargo run -- install temurin@21 --dry-run
cargo run -- install --no-progress temurin@21 --dry-run
```

---

## Phase 6: Cache Module Integration

**Goal**: Update cache module to support child progress for different metadata sources.

### Input Materials
- **Dependencies**:
  - Phases 1-3 (ProgressIndicator implementations ready)

- **Source Code to Modify**:
  - `/src/cache/mod.rs` - Cache functions
  - `/src/metadata/provider.rs` - MetadataProvider

### Tasks
- [ ] **Analyze metadata source handling**:
  - [ ] Identify source type detection
  - [ ] Find size estimation for HTTP sources
- [ ] **Implement source-specific logic**:
  - [ ] Foojay: Always create child progress
  - [ ] HTTP: Check size, create child if >= 10MB
  - [ ] Local: Never create child progress
- [ ] **Update fetch functions**:
  - [ ] Pass parent progress to sources
  - [ ] Create children based on source type
  - [ ] Properly complete child progress
- [ ] **Add size estimation**:
  - [ ] HEAD request for HTTP sources
  - [ ] Cache size information if available

### Deliverables
- Cache module with source-aware child progress
- Size-based threshold for HTTP sources
- Foojay always showing child progress

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib cache
cargo test --lib metadata
```

---

## Phase 7: Cache Command - Source Progress Integration

**Goal**: Update cache refresh command to show child progress for metadata sources.

### Input Materials
- **Dependencies**:
  - Phase 6 (Cache module ready)

- **Source Code to Modify**:
  - `/src/commands/cache.rs` - Cache command

### Tasks
- [ ] **Update refresh_cache function**:
  - [ ] Use updated cache module functions
  - [ ] Let cache module handle child creation
  - [ ] Maintain overall step counting
- [ ] **Ensure proper display**:
  - [ ] Parent shows overall steps
  - [ ] Children show source-specific progress
  - [ ] Summary remains after completion
- [ ] **Test with different configurations**:
  - [ ] Multiple sources configured
  - [ ] Large HTTP metadata files
  - [ ] Foojay-only configuration

### Deliverables
- Cache command with nested progress display
- Source-specific child progress bars
- Clean summary output

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib commands::cache
# Manual testing
cargo run -- cache refresh
cargo run -- cache refresh --no-progress
```

---

## Phase 8: Integration Tests

**Goal**: Create comprehensive tests for multi-progress functionality.

### Input Materials
- **Dependencies**:
  - Phases 1-7 (All implementation complete)

- **Source Code to Create/Modify**:
  - `/tests/multi_progress_integration.rs` - New test file
  - `/tests/common/progress_capture.rs` - Test utilities

### Tasks
- [ ] **Create test utilities**:
  - [ ] MultiProgressCapture for nested progress testing
  - [ ] Helper to verify parent-child relationships
  - [ ] Assertion helpers for progress hierarchies
- [ ] **Test scenarios**:
  - [ ] Parent with single child
  - [ ] Parent with no children (threshold not met)
  - [ ] Multiple operations with different child states
  - [ ] Error handling with active children
- [ ] **Test commands**:
  - [ ] Install with large download
  - [ ] Install with small download
  - [ ] Cache refresh with multiple sources
  - [ ] Cache refresh with single source
- [ ] **Test edge cases**:
  - [ ] Terminal resize during multi-progress
  - [ ] Ctrl+C interruption
  - [ ] Network timeout with active child

### Deliverables
- Comprehensive integration test suite
- Test utilities for multi-progress verification
- Coverage of all multi-progress scenarios

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --test multi_progress_integration
cargo test # Run all tests
```

---

## Phase 9: Performance Optimization

**Goal**: Ensure multi-progress implementation has minimal performance impact.

### Input Materials
- **Dependencies**:
  - Phases 1-8 (Full implementation)

- **Source Code to Optimize**:
  - `/src/indicator/indicatif.rs` - MultiProgress handling
  - Download and cache operations

### Tasks
- [ ] **Profile current implementation**:
  - [ ] Measure CPU usage during multi-progress
  - [ ] Check memory allocation patterns
  - [ ] Identify update frequency bottlenecks
- [ ] **Optimize update frequency**:
  - [ ] Implement update throttling for children
  - [ ] Batch updates when possible
  - [ ] Reduce redundant redraws
- [ ] **Memory optimization**:
  - [ ] Ensure proper cleanup of completed bars
  - [ ] Minimize allocations in hot paths
  - [ ] Use Arc/Rc where appropriate
- [ ] **Benchmark**:
  - [ ] Create benchmark for multi-progress operations
  - [ ] Compare with single progress baseline
  - [ ] Ensure < 1% CPU overhead

### Deliverables
- Optimized multi-progress implementation
- Performance benchmarks
- Documentation of performance characteristics

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo bench --bench progress_performance
cargo test --release # Ensure no performance regressions
```

---

## Phase 10: Design Document Update

**Goal**: Update the design document to reflect the completed implementation.

### Input Materials
- **Documentation to Update**:
  - `/docs/tasks/indicator/design_multi.md` - Design specification

### Tasks
- [ ] **Update design document**:
  - [ ] Mark implementation status as complete
  - [ ] Add implementation notes section with lessons learned
  - [ ] Document any deviations from original design
  - [ ] Add final verification results

### Deliverables
- Updated design document marked as implemented
- Lessons learned and implementation notes

### Verification
```bash
# Review the updated design document
cat docs/tasks/indicator/design_multi.md
```

---

## Implementation Order Summary

### Core Components (Phases 1-3)
1. **Phase 1**: ProgressIndicator trait and ALL implementations - minimal update (maintains compilation)
2. **Phase 2**: SimpleProgress - finalize implementation
3. **Phase 3**: IndicatifProgress with MultiProgress

### Integration (Phases 4-7)
4. **Phase 4**: Download module integration
5. **Phase 5**: Install command integration
6. **Phase 6**: Cache module integration
7. **Phase 7**: Cache command integration

### Quality Assurance (Phases 8-10)
8. **Phase 8**: Integration tests
9. **Phase 9**: Performance optimization
10. **Phase 10**: Design document update

## Dependencies

- External crates:
  - `indicatif` >= 0.17 with MultiProgress support
  
- Internal modules:
  - `src/indicator/` - Progress indicator system
  - `src/download/` - Download management
  - `src/cache/` - Cache operations
  - `src/commands/` - Command implementations

## Risks & Mitigations

1. **Risk**: Terminal corruption with multiple progress bars
   - **Mitigation**: Use indicatif's MultiProgress for proper synchronization
   - **Fallback**: Disable multi-progress in problematic terminals

2. **Risk**: Performance overhead from multiple bars
   - **Mitigation**: Implement update throttling and threshold-based display
   - **Fallback**: Allow disabling via environment variable

3. **Risk**: Complex state management with parent-child relationships
   - **Mitigation**: Limit to single level of nesting
   - **Fallback**: Flatten to single progress if issues arise

4. **Risk**: Backward compatibility with existing code
   - **Mitigation**: Gradual migration with stub implementations
   - **Fallback**: Make create_child() return self initially

## Success Metrics

- [ ] Large downloads (>10MB) show nested progress
- [ ] Small downloads (<10MB) don't create unnecessary bars  
- [ ] Foojay cache refresh always shows child progress
- [ ] No terminal corruption in any environment
- [ ] Performance overhead < 1% CPU
- [ ] All existing tests continue to pass
- [ ] CI environments continue to work correctly

## Notes for Implementation

- **Phase 1 is critical**: Updates trait and ALL implementations at once to maintain compilation
- Phase 2 is mostly documentation and cleanup (implementation already correct in Phase 1)
- Phase 3 contains the main complexity with MultiProgress implementation
- Always test visual output manually in addition to unit tests
- Use 10MB as consistent threshold across all operations
- Keep CI environment behavior unchanged (SimpleProgress returns SilentProgress children)
- Test with various terminal emulators (iTerm2, Terminal.app, Windows Terminal)
- Consider TERM environment variable for compatibility
- Maintain single progress bar for operations < 5 seconds
- Use `// TODO: Phase X` comments to mark where actual implementation will be added
- Commit working code at phase boundaries to allow rollback if needed
- Document any deviations from design during implementation