# Metadata Fetch Progress Indicator Implementation Plan

## Overview

This document outlines the implementation plan for adding progress indicator support to metadata fetching operations in Kopi. The implementation follows a bottom-up approach to maintain a compilable codebase throughout the process, with temporary `SilentProgress` instances used to resolve compilation errors during migration.

**Current Status**: Phase 1 - 10 Completed ✅

## Phase 1: MetadataSource Trait and All Implementations - Minimal Update ✅

**Goal**: Update the `MetadataSource` trait and ALL implementations with new signatures to maintain compilation.

### Input Materials
- **Documentation**:
  - `/docs/tasks/indicator/design_metadata.md` - Design specification
  
- **Source Code to Modify**:
  - `/src/metadata/source.rs` - MetadataSource trait definition
  - `/src/metadata/foojay.rs` - Foojay implementation
  - `/src/metadata/http.rs` - HTTP implementation
  - `/src/metadata/local.rs` - Local implementation
  - `/src/metadata/provider_tests.rs` - Mock implementation

### Tasks
- [x] **Update MetadataSource trait**:
  - [x] Add `progress: &mut dyn ProgressIndicator` to `fetch_all()`
  - [x] Add `progress: &mut dyn ProgressIndicator` to `fetch_distribution()`
  - [x] Add `progress: &mut dyn ProgressIndicator` to `fetch_package_details()` (note: no `ensure_complete()` method exists)
  - [x] Import `ProgressIndicator` trait in source.rs
- [x] **Minimal update to ALL implementations** (just add parameter, ignore it):
  - [x] FoojayMetadataSource: Add `_progress` parameter, mark with `// TODO: Phase 2`
  - [x] HttpMetadataSource: Add `_progress` parameter, mark with `// TODO: Phase 3`
  - [x] LocalDirectorySource: Add `_progress` parameter, mark with `// TODO: Phase 4`
  - [x] MockMetadataSource (tests): Add `_progress` parameter
- [x] **Fix all test calls**:
  - [x] Add `&mut SilentProgress` to all test invocations
  - [x] Import SilentProgress where needed
- [x] Update trait documentation

### Example Implementation
```rust
// In foojay.rs
impl MetadataSource for FoojayMetadataSource {
    fn fetch_all(&self, _progress: &mut dyn ProgressIndicator) -> Result<Vec<JdkMetadata>> {
        // TODO: Phase 2 - Add actual progress reporting
        
        // Existing implementation unchanged
        let packages = self.client.get_packages(None)?;
        // ...
    }
}
```

### Deliverables
- Updated trait with new signatures
- All implementations updated with minimal changes
- All tests compilable with SilentProgress
- Fully compilable codebase

### Verification ✅
```bash
cargo fmt                              # ✅ Completed
cargo clippy --all-targets -- -D warnings  # ✅ No warnings
cargo build --lib                      # ✅ Builds successfully
cargo test --lib metadata --no-run     # ✅ All tests compile
```

---

## Phase 2: FoojayMetadataSource Progress Implementation ✅

**Goal**: Add actual progress reporting to FoojayMetadataSource.

### Input Materials
- **Dependencies**:
  - Phase 1 (All signatures updated)

- **Source Code to Modify**:
  - `/src/metadata/foojay.rs` - Foojay source implementation

### Tasks
- [x] **Replace `_progress` with actual usage**:
  - [x] Remove underscore from parameter name
  - [x] Remove `// TODO: Phase 2` comment
- [x] **Add progress reporting in `fetch_all()`**:
  - [x] Before API call: `progress.set_message("Connecting to Foojay API...")`
  - [x] After receiving packages: `progress.set_message(format!("Retrieved {} packages from Foojay", packages.len()))`
  - [x] During conversion: `progress.set_message("Processing Foojay metadata...")`
  - [x] After completion: `progress.set_message(format!("Processed {} packages", count))`
- [x] **Add progress reporting in `fetch_distribution()`**:
  - [x] Similar progress messages for distribution-specific fetch
- [x] **Add progress reporting in `fetch_package_details()`**:
  - [x] Message when fetching package details
- [x] **Test with actual progress** (optional):
  - [x] Tests verified with all 4 tests passing

### Deliverables
- Fully implemented progress reporting in FoojayMetadataSource
- Progress messages at key operation points
- TODO comments removed

### Verification ✅
```bash
cargo fmt                              # ✅ Completed
cargo clippy --all-targets -- -D warnings  # ✅ No warnings
cargo test --lib metadata::foojay::tests  # ✅ All tests pass (4/4)
```

---

## Phase 3: HttpMetadataSource Progress Implementation ✅

**Goal**: Add actual progress reporting to HttpMetadataSource.

### Input Materials
- **Dependencies**:
  - Phase 1 (All signatures updated)

- **Source Code to Modify**:
  - `/src/metadata/http.rs` - HTTP source implementation

### Tasks
- [x] **Replace `_progress` with actual usage**:
  - [x] Remove underscore from parameter name
  - [x] Remove `// TODO: Phase 3` comment
- [x] **Add progress reporting in `fetch_all()`**:
  - [x] Start fetch: `progress.set_message("Fetching metadata from HTTP source...")`
  - [x] If byte counts available: Update with download progress
  - [x] Processing: `progress.set_message("Processing HTTP metadata...")`
  - [x] Completion: `progress.set_message(format!("Loaded {} packages", count))`
- [x] **Add progress reporting in `fetch_distribution()`**:
  - [x] Similar messages for distribution-specific fetch
- [x] **Add progress reporting in `fetch_package_details()`**:
  - [x] Message when completing package details
- [x] **Consider HTTP-specific features**:
  - [x] Show URL being fetched (if not sensitive)
  - [x] Show download size if available from headers

### Deliverables ✅
- ✅ Fully implemented progress reporting in HttpMetadataSource
- ✅ HTTP-specific progress information (file paths, URLs) 
- ✅ TODO comments removed

### Verification ✅
```bash
cargo fmt                               # ✅ Completed
cargo clippy --all-targets -- -D warnings  # ✅ No warnings
cargo test --lib metadata::http_tests  # ✅ All tests pass
```

---

## Phase 4: LocalDirectorySource Progress Implementation ✅

**Goal**: Add actual progress reporting to LocalDirectorySource.

### Input Materials
- **Dependencies**:
  - Phase 1 (All signatures updated)

- **Source Code to Modify**:
  - `/src/metadata/local.rs` - Local source implementation

### Tasks
- [x] **Replace `_progress` with actual usage**:
  - [x] Remove underscore from parameter name
  - [x] Remove `// TODO: Phase 4` comment
- [x] **Add progress reporting in `fetch_all()`**:
  - [x] Start: `progress.set_message("Reading local metadata directory...")`
  - [x] Per file: `progress.set_message(format!("Reading {}", filename))`
  - [x] After each file: Update count if multiple files
  - [x] Completion: `progress.set_message(format!("Loaded {} local packages", count))`
- [x] **Add progress reporting in `fetch_distribution()`**:
  - [x] Similar messages for distribution-specific loading
- [x] **Add progress reporting in `fetch_package_details()`**:
  - [x] Message when looking up package details
- [x] **Consider file system specifics**:
  - [x] Show directory path being scanned
  - [x] Show file count if known in advance

### Deliverables
- Fully implemented progress reporting in LocalDirectorySource
- File-by-file progress updates  
- TODO comments removed

### Verification ✅
```bash
cargo fmt                              # ✅ Completed
cargo clippy --all-targets -- -D warnings  # ✅ No warnings
cargo test --lib metadata::local::tests    # ✅ All 9 tests passing
```

---

## Phase 5: MetadataProvider Update ✅

**Goal**: Update MetadataProvider to propagate progress indicators and manage step-based progress.

### Input Materials
- **Dependencies**:
  - Phase 1 (All implementations have progress parameters)
  - Phases 2-4 (Optional - sources may or may not have actual progress yet)

- **Source Code to Modify**:
  - `/src/metadata/provider.rs` - Provider implementation

### Tasks
- [x] **Update `MetadataProvider` signatures**:
  - [x] Add `progress: &mut dyn ProgressIndicator` to `fetch_all()`
  - [x] Add `progress: &mut dyn ProgressIndicator` to `fetch_distribution()`
  - [x] Add `pub fn source_count(&self) -> usize` method (already existed)
- [x] **Propagate progress to sources**:
  - [x] In `fetch_all()`: Pass progress to `source.fetch_all(progress)`
  - [x] In `fetch_distribution()`: Pass progress to `source.fetch_distribution(distribution, progress)`
  - [x] Remove any existing `SilentProgress` usage if present
- [x] **Note: No step counting yet** (will be done in caller):
  - [x] Just propagate progress to sources
  - [x] Let caller manage step counting
- [x] **Temporarily fix callers**:
  - [x] Add `&mut SilentProgress` in cache module calls
  - [x] Mark with `// TODO: Phase 6 - Replace with actual progress indicator`
- [x] **Update ensure_complete and ensure_complete_batch methods**:
  - [x] Add progress parameter to both methods
  - [x] Propagate to fetch_package_details
- [x] **Fix install command temporary usages**:
  - [x] Add SilentProgress to ensure_complete calls
  - [x] Mark with TODO comments for Phase 8

### Deliverables
- Updated provider methods accepting progress parameter
- Progress properly propagated to all sources
- `source_count()` method for caller's step calculation
- Temporary fixes in cache module

### Verification ✅
```bash
cargo fmt                              # ✅ Completed
cargo clippy --all-targets -- -D warnings  # ✅ No warnings
cargo test --lib metadata::provider    # ✅ All tests pass (17/17)
```

---

## Phase 6: Cache Module Functions Update ✅

**Goal**: Update cache module functions to support progress indicators with step tracking.

### Input Materials
- **Dependencies**:
  - Phases 1-5 (Provider and sources updated)

- **Source Code to Modify**:
  - `/src/cache/mod.rs` - Cache module functions
  - `/src/cache/tests.rs` - Cache tests

### Tasks
- [x] Update function signatures:
  - [x] Add progress and current_step to `fetch_and_cache_metadata()`
  - [x] Add progress and current_step to `fetch_and_cache_distribution()`
  - [x] Update `get_metadata()` to use `SilentProgress` internally
- [x] Implement step-based progress:
  - [x] Processing metadata step
  - [x] Grouping by distribution step
  - [x] Saving to cache step
  - [x] Completion step
- [x] **Update tests**:
  - [x] Pass `SilentProgress` and step counter in tests (not needed - using wrapper functions)
  - [x] Add progress tracking tests (deferred to Phase 10)
- [x] **Create temporary wrapper functions**:
  - [x] Keep old signatures for backward compatibility
  - [x] Call new functions with `SilentProgress`
  - [x] Mark with TODO comments

### Deliverables
- Updated `src/cache/mod.rs` with progress support
- Step-based progress reporting
- Temporary wrapper functions for compatibility

### Verification ✅
```bash
cargo fmt                              # ✅ Completed
cargo clippy --all-targets -- -D warnings  # ✅ No warnings
cargo test --lib cache::tests          # ✅ All tests pass (36/36)
```

---

## Phase 7: Cache Command Integration ✅

**Goal**: Update the cache refresh command to use step-based progress indicators.

### Input Materials
- **Dependencies**:
  - Phases 1-6 (All lower-level components)

- **Source Code to Modify**:
  - `/src/commands/cache.rs` - Cache command implementation

### Tasks
- [x] Update `refresh_cache()` function:
  - [x] Calculate total steps: `5 + provider.source_count()`
  - [x] Initialize progress with `ProgressStyle::Count`
  - [x] Start with total steps using `with_total()`
  - [x] Initialize step counter
- [x] Implement step updates:
  - [x] Step 1: Initialization
  - [x] Steps 2-N: Source attempts (delegated)
  - [x] Steps N+1 to N+4: Processing steps (delegated)
- [x] Pass progress to `fetch_and_cache_metadata_with_progress()`
- [x] Add summary output after completion
- [x] **Update for new signatures**:
  - [x] Update cache refresh to use new signatures directly
  - [x] Update search_cache to use new signatures for distribution fetching
  - [x] Keep backward compatibility wrappers for Phase 8

### Deliverables ✅
- ✅ Updated `src/commands/cache.rs` with step-based progress
- ✅ Proper progress calculation and initialization
- ✅ Backward compatibility wrappers retained for Phase 8

### Verification ✅
```bash
cargo fmt                              # ✅ Completed
cargo clippy --all-targets -- -D warnings  # ✅ No warnings
cargo test --lib commands::cache::tests    # ✅ All 8 tests passing
# Manual testing - Ready for testing
kopi cache refresh
kopi cache refresh --no-progress
```

---

## Phase 8: Install Command - Cache Refresh Support ✅

**Goal**: Update install command's cache refresh to use progress indicators.

### Input Materials
- **Dependencies**:
  - Phases 1-7 (Cache functions updated)

- **Source Code to Modify**:
  - `/src/commands/install.rs` - Install command

### Tasks
- [x] Update `ensure_fresh_cache()` method:
  - [x] Add `progress: &mut dyn ProgressIndicator` and `current_step` parameters
  - [x] Pass progress to `fetch_and_cache_metadata_with_progress()`
  - [x] Handle progress in fallback scenarios
- [x] Update callers of `ensure_fresh_cache()`:
  - [x] Update `find_matching_package()` to accept progress parameters
  - [x] Pass progress indicator from execute()
  - [x] Handle step counting
- [x] **Create temporary fix for execute()**:
  - [x] Use local SilentProgress for now
  - [x] Mark with TODO for Phase 9
- [x] **Update ensure_complete calls**:
  - [x] Use same progress variable instead of creating new SilentProgress
  - [x] Remove redundant TODO comments

### Deliverables ✅
- ✅ Updated `ensure_fresh_cache()` with progress support
- ✅ Progress propagation during cache refresh
- ✅ Temporary progress usage in execute()
- ✅ All ensure_complete calls using shared progress

### Verification ✅
```bash
cargo fmt                               # ✅ Completed
cargo clippy --all-targets -- -D warnings  # ✅ No warnings
cargo test --lib commands::install::tests  # ✅ All 19 tests passing
```

---

## Phase 9: Install Command - Full Progress Integration ✅

**Goal**: Implement complete step-based progress for the install command.

### Input Materials
- **Dependencies**:
  - Phase 8 (ensure_fresh_cache updated)

- **Source Code to Modify**:
  - `/src/commands/install.rs` - Full install flow

### Tasks
- [x] Create overall progress indicator:
  - [x] Calculate base steps (8)
  - [x] Add optional steps (checksum, shims)
  - [x] Detect if cache refresh needed
  - [x] Update total steps dynamically
- [x] Implement step progression:
  - [x] Parse version step
  - [x] Check cache step  
  - [x] Find package step
  - [x] Check installation step
  - [x] Download step (maintain separate progress)
  - [x] Extract and install steps
  - [x] Shim creation step (if enabled)
- [x] Handle download progress:
  - [x] Keep download progress independent
  - [x] Show as sub-progress of overall step
- [x] Complete with success message
- [x] **Remove all temporary code**

### Deliverables ✅
- ✅ Full step-based progress for install command
- ✅ Dynamic step calculation based on cache refresh and options
- ✅ Independent download progress bar
- ✅ Clean removal of all TODOs from Phase 9

### Verification ✅
```bash
cargo fmt                              # ✅ Completed
cargo clippy --all-targets -- -D warnings  # ✅ No warnings
cargo test --lib commands::install::tests  # ✅ All 19 tests passing
# Manual testing - Ready for testing
kopi install temurin@21 --dry-run
kopi install --no-progress temurin@21 --dry-run
```

---

## Phase 10: Integration Tests Update ✅

**Goal**: Update all integration tests to work with new progress parameters.

### Input Materials
- **Dependencies**:
  - Phases 1-9 (All implementations complete)

- **Source Code to Modify**:
  - `/tests/cache_integration.rs`
  - Other integration test files

### Tasks
- [x] Update cache integration tests:
  - [x] Import `SilentProgress` from indicator module
  - [x] Update `test_fetch_and_cache_metadata()`
  - [x] Add current_step parameter
- [x] Create new progress tests:
  - [x] Test step counting accuracy
  - [x] Test progress message content
  - [x] Test error handling with progress
- [x] Add `TestProgressCapture` helper:
  - [x] Create in `tests/common/mod.rs`
  - [x] Capture progress updates for verification
  - [x] Use in progress-specific tests
- [x] Update install integration tests (if any)

### Deliverables ✅
- ✅ All integration tests updated and passing
- ✅ New test helper for progress verification (`TestProgressCapture`)
- ✅ Comprehensive progress behavior tests

### Verification ✅
```bash
cargo fmt                                    # ✅ Completed
cargo clippy --all-targets -- -D warnings   # ✅ No warnings
cargo test --test cache_integration         # ✅ All tests pass (10/10)
cargo test --test progress_indicator_integration  # ✅ All tests pass (30/30)
```

---

## Phase 11: Cleanup and Optimization

**Goal**: Remove all temporary code and optimize progress reporting.

### Input Materials
- **Dependencies**:
  - Phases 1-10 (All code migrated)

### Tasks
- [ ] Search for remaining TODOs:
  - [ ] `grep -r "TODO.*progress" src/`
  - [ ] Remove all temporary `SilentProgress` usage
  - [ ] Remove wrapper functions
- [ ] Optimize progress updates:
  - [ ] Reduce redundant updates
  - [ ] Batch message updates where appropriate
  - [ ] Profile performance impact
- [ ] Add missing `total_packages()` method to MetadataCache
- [ ] Ensure consistent error handling:
  - [ ] Call `progress.error()` on failures
  - [ ] Clean progress state properly
- [ ] Final code review pass

### Deliverables
- Clean codebase with no temporary code
- Optimized progress reporting
- Consistent error handling

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --all
cargo bench --bench progress_indicator_bench
```

---

## Phase 12: Documentation and Examples

**Goal**: Document the metadata progress implementation for developers.

### Input Materials
- **Documentation to Update**:
  - `/docs/developer/progress-indicators.md`
  - `/docs/tasks/indicator/design_metadata.md`

### Tasks
- [ ] Update developer guide:
  - [ ] Add metadata fetching examples
  - [ ] Document step calculation formula
  - [ ] Explain progress propagation pattern
- [ ] Create usage examples:
  - [ ] Example of adding progress to new metadata source
  - [ ] Example of step-based progress calculation
  - [ ] Example of nested progress (install with download)
- [ ] Update design document:
  - [ ] Mark as implemented
  - [ ] Add lessons learned
  - [ ] Document any deviations
- [ ] Add inline code documentation:
  - [ ] Document progress parameters in public APIs
  - [ ] Add examples in doc comments

### Deliverables
- Updated developer documentation
- Code examples for common scenarios
- Design document marked complete
- Comprehensive inline documentation

### Verification
```bash
cargo doc --no-deps --open
# Review generated documentation
```

---

## Implementation Order Summary

### Lower-Level Components (Phases 1-4)
1. **Phase 1**: MetadataSource trait and ALL implementations - minimal signature update (maintains compilation) ✅
2. **Phase 2**: FoojayMetadataSource - add actual progress reporting ✅
3. **Phase 3**: HttpMetadataSource - add actual progress reporting ✅
4. **Phase 4**: LocalDirectorySource - add actual progress reporting ✅

### Mid-Level Components (Phases 5-6)
5. **Phase 5**: MetadataProvider update ✅
6. **Phase 6**: Cache module functions update ✅

### Command Integration (Phases 7-9)
7. **Phase 7**: Cache command integration ✅
8. **Phase 8**: Install command - cache refresh support ✅
9. **Phase 9**: Install command - full progress integration ✅

### Testing and Cleanup (Phases 10-12)
10. **Phase 10**: Integration tests update ✅
11. **Phase 11**: Cleanup and optimization
12. **Phase 12**: Documentation and examples

## Dependencies

- Internal modules:
  - `src/indicator/` - Progress indicator system (must be implemented first)
  - `src/metadata/` - Metadata fetching system
  - `src/cache/` - Cache management
  - `src/commands/` - Command implementations

## Risks & Mitigations

1. **Risk**: Breaking existing functionality during migration
   - **Mitigation**: Use SilentProgress temporarily to maintain compatibility
   - **Fallback**: Keep old signatures until fully migrated

2. **Risk**: Complex step counting logic
   - **Mitigation**: Start with simple counting, refine iteratively
   - **Fallback**: Use indeterminate progress if counting fails

3. **Risk**: Performance impact from progress updates
   - **Mitigation**: Batch updates, benchmark before/after
   - **Fallback**: Reduce update frequency if needed

4. **Risk**: Test failures from signature changes
   - **Mitigation**: Update tests incrementally with each phase
   - **Fallback**: Use test helpers to abstract progress

## Success Metrics

- [ ] All metadata operations show progress
- [ ] Step-based progress with accurate counts
- [ ] No performance regression (< 5ms overhead)
- [ ] All tests passing with new signatures
- [ ] Clean codebase with no temporary code
- [ ] Download progress remains independent
- [ ] Cache refresh shows detailed progress

## Notes for Implementation

- **Phase 1 is critical**: Updates ALL implementations at once to maintain compilation
- Always keep code compilable between phases
- Use `// TODO: Phase X` comments to mark where actual implementation will be added
- Use `// TODO: Replace with actual progress` for temporary `SilentProgress` usage
- Test after each phase completion
- Commit working code at phase boundaries
- Use `SilentProgress` in tests unless testing progress behavior
- Maintain independent download progress bars
- Calculate steps dynamically based on configuration
- Phases 2-4 can be done in any order after Phase 1
