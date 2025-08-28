# Multi-Progress Support Implementation Plan

**Last Updated**: 2025-08-28 (Updated with design refinements and trait extensions)

## Overview

This document outlines the implementation plan for adding multi-progress bar support to Kopi's ProgressIndicator system. The implementation focuses on providing nested progress bars for operations with clear parent-child relationships, particularly for download operations and cache refresh from different sources.

**Current Status**: Phase 1-3 require revision due to design refinements  
**Design Validation**: ✅ Completed via spike implementation (see `multiprogress_spike_report.md` and `multi_progress_spike.rs`)
**Recent Changes**: ProgressIndicator trait extended with `suspend()` and `println()` methods

## Revision Required Due to Design Changes

The following design changes require revision of completed phases:

1. **ProgressIndicator trait extension**: Added `suspend()` and `println()` methods for safe output
2. **SimpleProgress refinement**: Replace Unicode symbols with ASCII-only output
3. **IndicatifProgress architecture**: Changed from `Option<Arc<MultiProgress>>` to always-initialized `Arc<MultiProgress>`
4. **Template management**: Templates determined at construction, not runtime

**Affected Phases**:
- **Phase 1** (trait definition): Needs implementation of new methods across all types
- **Phase 2** (SimpleProgress): Replace Unicode ("✓"/"✗") with ASCII ("[OK]"/"[ERROR]")  
- **Phase 3** (IndicatifProgress): Structural changes and new method implementations
- **Phase 4** (Download): May need minor adjustments for new API
- **Phase 5-10**: Not yet implemented, plan updated accordingly

## Spike Validation Summary

The design has been thoroughly validated through a working spike implementation that demonstrates:
- ✅ **Visual Hierarchy**: Clean parent-child display with `└─` indentation
- ✅ **Performance**: < 1% CPU overhead confirmed
- ✅ **Thread Safety**: Concurrent updates work correctly
- ✅ **API Patterns**: `insert_after()`, `finish_and_clear()` validated
- ✅ **Template Design**: Spinner placement and message positioning optimized

**Ready for Phase 3 Implementation** with validated patterns and reference code.

## Phase 1: Core Infrastructure - Trait and ALL Implementations Update 🔄 (Requires Revision)

**Goal**: Update the ProgressIndicator trait with new methods and ALL implementations to maintain compilation.

### Input Materials
- **Documentation**:
  - `/docs/tasks/indicator/design_multi.md` - Updated design specification
  
- **Source Code to Modify**:
  - `/src/indicator/mod.rs` - ProgressIndicator trait definition
  - `/src/indicator/silent.rs` - SilentProgress implementation
  - `/src/indicator/simple.rs` - SimpleProgress implementation
  - `/src/indicator/indicatif.rs` - IndicatifProgress implementation
  - `/tests/common/progress_capture.rs` - Test helper implementation

### Tasks
- [x] **Update ProgressIndicator trait**:
  - [x] Add `fn create_child(&mut self) -> Box<dyn ProgressIndicator>` method
  - [x] Add `fn suspend(&self, f: &mut dyn FnMut())` method ✅
  - [x] Add `fn println(&self, message: &str) -> std::io::Result<()>` method ✅
  - [x] Update trait documentation
  - [x] Ensure Send + Sync trait bounds
- [ ] **Implement new methods for ALL types**:
  - [x] SilentProgress: Implement all three methods (no-op for suspend/println)
  - [ ] SimpleProgress: Implement with ASCII symbols only, no Unicode
  - [ ] IndicatifProgress: Implement with MultiProgress integration
- [ ] **Ensure compilation**:
  - [ ] All implementations compile with new methods
  - [ ] Update test helpers in progress_capture.rs
  - [ ] All existing tests pass

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

## Phase 2: SimpleProgress Final Implementation 🔄 (Requires Revision)

**Goal**: Finalize SimpleProgress with ASCII-only output and new trait methods.

### Input Materials
- **Dependencies**:
  - Phase 1 (All implementations compilable with new trait methods)

- **Source Code to Modify**:
  - `/src/indicator/simple.rs` - SimpleProgress implementation

### Tasks
- [ ] **Update SimpleProgress implementation**:
  - [ ] Replace Unicode symbols ("✓"/"✗") with ASCII ("[OK]"/"[ERROR]")
  - [ ] Keep `create_child()` returning `Box::new(SilentProgress)`
  - [ ] Implement `suspend()` method (direct execution, no suspension needed)
  - [ ] Implement `println()` method (direct println! output)
  - [ ] Add documentation explaining ASCII-only output for CI/NO_COLOR environments
  - [ ] Update tests to verify ASCII symbols

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

## Phase 3: IndicatifProgress MultiProgress Implementation 🔄 (Requires Revision)

**Goal**: Implement full MultiProgress support in IndicatifProgress with refined architecture.

### Input Materials
- **Dependencies**:
  - Phase 1 (Trait updated with new methods)
  - Phase 2 (SimpleProgress finalized)
  - `indicatif` crate with MultiProgress support
  - **Design Updates**: Refined architecture from design_multi.md

- **Source Code to Modify**:
  - `/src/indicator/indicatif.rs` - IndicatifProgress implementation

### Updated Implementation Approach

Based on design refinements, the implementation structure has been updated:

```rust
pub struct IndicatifProgress {
    multi: Arc<MultiProgress>,           // Always initialized, no Option
    owned_bar: Option<ProgressBar>,     // This instance's progress bar
    template: String,                    // Template determined at construction
}
```

#### Key Implementation Details:
- **Template Pattern**: `{spinner} {prefix} [{bar:30}] {pos}/{len} {msg}`
- **Child Template**: `"  └─ {spinner} {prefix} [{bar:25}] {pos}/{len} {msg}"`
- **Progress Chars**: Use simplified `██░` for cleaner display
- **Tick Chars**: `⣾⣽⣻⢿⡿⣟⣯⣷` for smooth animation
- **Positioning**: Use `insert_after()` for logical parent-child relationships
- **Steady Tick**: Enable with `Duration::from_millis(80)`

### Tasks
- [ ] **Refactor IndicatifProgress structure**:
  - [ ] Change `multi` to `Arc<MultiProgress>` (always initialized, no Option)
  - [ ] Rename `progress_bar` to `owned_bar` for clarity
  - [ ] Remove `is_child` field (no longer needed)
  - [ ] Add `template: String` field (determined at construction)
  - [ ] Update `new()` to always create MultiProgress
- [ ] **Implement create_child()**:
  - [ ] Share parent's `Arc<MultiProgress>` via `Arc::clone()`
  - [ ] Set child template with "  └─ " prefix
  - [ ] No immediate bar creation (deferred to `start()`)
- [ ] **Update existing methods**:
  - [ ] Modify `start()` to:
    - Use the pre-determined template from field
    - Add bar to MultiProgress with `multi.add()`
    - Enable steady tick with 80ms interval
  - [ ] Ensure `complete()` calls appropriate finish method
  - [ ] Update `error()` to properly abandon bars
- [ ] **Implement new trait methods**:
  - [ ] `suspend()`: Delegate to `multi.suspend()` 
  - [ ] `println()`: Delegate to `multi.println()`
- [ ] **Apply validated patterns**:
  - [ ] Use `██░` progress chars
  - [ ] Keep messages at template end: `{msg}`
  - [ ] Template selection at construction, not runtime
- [ ] **Add tests**:
  - [x] Test parent-child bar creation
  - [x] Test multiple children
  - [x] Test cleanup on completion with `finish_and_clear()`
  - [x] Test nested progress depth
  - [x] Test child with error handling
  - [x] Test child spinner without total
  - [x] Test concurrent updates (implicitly tested)

### Deliverables
- IndicatifProgress with full MultiProgress support
- Proper visual nesting with validated display patterns
- Comprehensive tests for multi-bar scenarios

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib indicator::indicatif
# Visual verification using spike patterns
# Can temporarily restore multi_progress_spike.rs to compare output
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
  - **Reference**: Spike test patterns from `multi_progress_spike.rs`

- **Source Code to Create/Modify**:
  - `/tests/multi_progress_integration.rs` - New test file
  - `/tests/common/progress_capture.rs` - Test utilities

### Expected Visual Output (Validated by Spike)
```
⣾ Installing temurin@21 [████████░░░░░░░░░░] 3/8 Downloading
  └─ ⣟ Downloading: 124.5MB / 256.3MB [48%] 2.3MB/s
```

### Tasks
- [ ] **Create test utilities**:
  - [ ] MultiProgressCapture for nested progress testing
  - [ ] Helper to verify parent-child relationships
  - [ ] Assertion helpers for progress hierarchies
  - [ ] Verify spinner placement at line start
  - [ ] Check for `└─` indentation in child bars
- [ ] **Test scenarios**:
  - [ ] Parent with single child
  - [ ] Parent with no children (threshold not met)
  - [ ] Multiple operations with different child states
  - [ ] Error handling with active children
  - [ ] Verify `finish_and_clear()` removes bars completely
- [ ] **Test commands**:
  - [ ] Install with large download
  - [ ] Install with small download
  - [ ] Cache refresh with multiple sources
  - [ ] Cache refresh with single source
- [ ] **Test edge cases**:
  - [ ] Terminal resize during multi-progress
  - [ ] Ctrl+C interruption
  - [ ] Network timeout with active child
  - [ ] Concurrent updates (thread safety)

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
  - `/docs/tasks/indicator/design_multi.md` - Design specification (already includes spike validation)
  - `/docs/tasks/indicator/multiprogress_spike_report.md` - Spike findings

### Tasks
- [ ] **Update design document**:
  - [ ] Mark implementation status as complete
  - [ ] Add implementation notes section with lessons learned
  - [ ] Document any deviations from original design vs spike vs final
  - [ ] Add final verification results
  - [ ] Cross-reference with spike validation results

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

### Core Components (Phases 1-3) - Requires Revision
1. **Phase 1**: ProgressIndicator trait and ALL implementations - add suspend/println methods 🔄
2. **Phase 2**: SimpleProgress - replace Unicode with ASCII symbols 🔄
3. **Phase 3**: IndicatifProgress with refined MultiProgress architecture 🔄

### Integration (Phases 4-7)
4. **Phase 4**: Download module integration ✅ (may need minor adjustments)
5. **Phase 5**: Install command integration
6. **Phase 6**: Cache module integration
7. **Phase 7**: Cache command integration

### Quality Assurance (Phases 8-10)
8. **Phase 8**: Integration tests
9. **Phase 9**: Performance optimization (partially validated by spike)
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
   - **Validation**: ✅ Spike confirmed no corruption with MultiProgress
   - **Fallback**: Disable multi-progress in problematic terminals

2. **Risk**: Performance overhead from multiple bars
   - **Mitigation**: Implement update throttling and threshold-based display
   - **Validation**: ✅ Spike showed < 1% CPU overhead
   - **Fallback**: Allow disabling via environment variable

3. **Risk**: Complex state management with parent-child relationships
   - **Mitigation**: Limit to single level of nesting
   - **Validation**: ✅ Spike demonstrated simple Arc-based sharing works well
   - **Fallback**: Flatten to single progress if issues arise

4. **Risk**: Backward compatibility with existing code
   - **Mitigation**: Gradual migration with stub implementations
   - **Validation**: Phase 1-2 approach proven to maintain compatibility
   - **Fallback**: Make create_child() return self initially

## Success Metrics

- [ ] Large downloads (>10MB) show nested progress with spinner at line start
- [ ] Small downloads (<10MB) don't create unnecessary bars  
- [ ] Foojay cache refresh always shows child progress
- [ ] No terminal corruption in any environment (validated in spike)
- [ ] Performance overhead < 1% CPU (validated in spike)
- [ ] All existing tests continue to pass
- [ ] CI environments continue to work correctly
- [ ] Visual hierarchy displays correctly with `└─` indentation
- [ ] Progress bars use clean `██░` characters
- [ ] Spinners animate smoothly with steady tick

## Notes for Implementation

### Recent Design Refinements
- **suspend()/println() methods**: Essential for safe log output during progress operations
- **ASCII-only for SimpleProgress**: Ensures compatibility in NO_COLOR/CI environments
- **No Option wrapper for MultiProgress**: Simplifies code by always initializing
- **Template as field**: Reduces runtime decisions, cleaner separation of parent/child

### Spike Validation Results
The design has been validated through a comprehensive spike implementation (`multi_progress_spike.rs`). Key validated patterns:
- **Visual Hierarchy**: `insert_after()` provides correct parent-child positioning
- **Template Stability**: Dynamic messages at end (`{msg}`) prevent layout disruption
- **Performance**: No significant overhead with multiple concurrent bars
- **Thread Safety**: `Arc<MultiProgress>` enables safe sharing between parent and child
- **Clean Removal**: `finish_and_clear()` properly removes bars from display

### Implementation Guidelines
- **Phase 1 is critical**: Updates trait with new methods and ALL implementations must compile
- **Phase 2 focus**: Remove Unicode symbols, use ASCII only for CI/NO_COLOR compatibility
- **Phase 3 architecture**: Always-initialized MultiProgress, template determined at construction
- Always test visual output manually in addition to unit tests
- Use 10MB as consistent threshold across all operations
- Keep CI environment behavior unchanged (SimpleProgress returns SilentProgress children)
- Test with various terminal emulators (iTerm2, Terminal.app, Windows Terminal)
- Consider TERM environment variable for compatibility
- Maintain single progress bar for operations < 5 seconds
- Use `// TODO: Phase X` comments to mark where actual implementation will be added
- Commit working code at phase boundaries to allow rollback if needed
- Document any deviations from design during implementation

### Visual Pattern Reference
```
⣾ Parent Task [██████████░░░░░░░░] 2/3 Processing step 2 of 3
  └─ Subtask 2 [███████░░░░] 25/50
```
Use this pattern as the reference for all multi-progress implementations