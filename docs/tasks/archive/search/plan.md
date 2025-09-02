# Cache Search Command Implementation Plan

## Overview
This document outlines the phased implementation plan for improving the `kopi cache search` command based on the design decisions in `/docs/tasks/search/design.md`. The improvements focus on enhanced display options, flexible search queries, and programmatic output formats.

## Command Syntax

### Search Commands
- `kopi cache search <version>` - Search for specific version
- `kopi cache search <distribution>` - Search all versions of a distribution
- `kopi cache search <distribution>@<version>` - Search specific distribution and version
- `kopi cache search latest` - Show latest version of each distribution
- `kopi cache search <query> [options]` - Search with display options
  - `--compact` - Minimal display (default)
  - `--detailed` - Full information display
  - `--json` - JSON output for programmatic use
  - `--lts-only` - Filter to show only LTS versions

### List Commands
- `kopi cache list-distributions` - List all available distributions in cache

## Phase 1: Display Improvements

### Input Resources
- `/docs/tasks/search/design.md` - Design decisions and requirements
- `/src/commands/cache.rs` - Existing cache search implementation
- `/src/api/models.rs` - Current Package model structure
- foojay.io API documentation for additional fields

### Deliverables
1. **Enhanced Package Model** (update `/src/api/models.rs`)
   - Add `term_of_support: Option<String>` field
   - Add `release_status: Option<String>` field
   - Add `latest_build_available: Option<bool>` field (internal use)
   - Update deserialization to handle new fields gracefully

2. **Display Column Updates** (update `/src/commands/cache.rs`)
   - Remove auto-selection marker (►) logic
   - Remove Archive column from display
   - Add LTS column showing term_of_support
   - Add OS/Arch columns for detailed view
   - Add Status (GA/EA) column for detailed view

3. **Display Mode Implementation** (update `/src/commands/cache.rs`)
   - Add `--compact` flag (default behavior)
   - Add `--detailed` flag for full information
   - Add `--json` flag for JSON output
   - Implement display logic for each mode

4. **Unit Tests**
   - `src/api/models.rs` - Model serialization with new fields
   - `src/commands/cache.rs` - Display mode formatting tests
   - JSON output structure validation

5. **Integration Tests** (`/tests/cache_search_display.rs`)
   - Verify compact display shows minimal columns
   - Verify detailed display includes all information
   - Verify JSON output contains all fields
   - Test with real cached metadata

### Success Criteria
- Compact view shows only Distribution, Version, LTS columns
- Detailed view includes Status, Type, OS/Arch, LibC, Size, JavaFX
- JSON output is valid and parseable by jq
- No auto-selection marker displayed
- New API fields are properly parsed and stored

## Phase 2: Flexible Search Queries

### Input Resources
- Phase 1 deliverables
- `/src/version/parser.rs` - Version parsing logic
- `/src/search/mod.rs` - Search implementation

### Deliverables
1. **Enhanced Version Parser** (update `/src/version/parser.rs`)
   - Make version optional in `ParsedVersionRequest`
   - Support distribution-only queries (e.g., "corretto")
   - Support "latest" keyword for newest versions
   - Maintain backward compatibility

2. **Search Logic Updates** (update `/src/search/mod.rs`)
   - Implement distribution-only search
   - Add latest version filtering
   - Update `matches_package` logic for optional version
   - Optimize search performance

3. **Command Argument Updates** (update `/src/commands/cache.rs`)
   - Update search command to accept flexible queries
   - Add validation for new query formats
   - Improve error messages for invalid queries

4. **Unit Tests**
   - `src/version/parser.rs` - Parse distribution-only queries
   - `src/search/mod.rs` - Search with optional version
   - Query validation edge cases

5. **Integration Tests** (`/tests/cache_search_queries.rs`)
   - Test distribution-only searches
   - Test latest version searches
   - Verify backward compatibility
   - Test error handling for invalid queries

### Success Criteria
- `kopi cache search corretto` lists all Corretto versions
- `kopi cache search latest` shows newest version per distribution
- Existing version queries continue to work
- Clear error messages for unsupported queries

## Phase 3: Filtering and List Commands

### Input Resources
- Phase 1 & 2 deliverables
- `/src/commands/cache.rs` - Cache command structure
- LTS version information from Package model

### Deliverables
1. **Filtering Options** (update `/src/commands/cache.rs`)
   - Add `--lts-only` flag to filter LTS versions
   - Implement filtering logic in search results
   - Update help text with new options

2. **List Distributions Command** (new in `/src/commands/cache.rs`)
   - Add `kopi cache list-distributions` subcommand
   - Extract unique distributions from cache
   - Display distribution names and display names
   - Include version count per distribution

3. **Unit Tests**
   - Filter option combinations
   - Distribution list extraction

4. **Integration Tests** (`/tests/cache_filtering.rs`)
   - LTS-only filtering accuracy
   - Distribution list completeness

### Success Criteria
- `--lts-only` shows only LTS versions (8, 11, 17, 21, etc.)
- `list-distributions` provides complete distribution list
- Cached metadata only includes packages for current platform
- Platform information available via `kopi doctor` command

## Phase 4: Performance and User Experience

### Input Resources
- All previous phase deliverables
- Performance profiling data
- User feedback on display formats

### Deliverables
1. **Performance Optimizations** (update `/src/search/mod.rs`)
   - Optimize search algorithms for large caches
   - Implement lazy loading where applicable
   - Add progress indicators for long operations
   - Cache search results within command execution

2. **Output Format Refinements** (update `/src/commands/cache.rs`)
   - Improve table formatting and alignment
   - Add color coding for LTS/EA versions (if terminal supports)
   - Implement smart column width calculation
   - Add result count summary

3. **Error Message Improvements**
   - Enhance error messages with suggestions
   - Add hints for common mistakes
   - Provide examples in error output
   - Link to documentation where relevant

4. **Documentation Updates**
   - Update `/docs/reference.md` with new search features
   - Add examples for all query types
   - Document JSON output schema
   - Create troubleshooting guide

5. **Performance Tests** (`/tests/cache_performance.rs`)
   - Benchmark search performance with large caches
   - Measure display rendering time
   - Profile memory usage during searches

### Success Criteria
- Search completes in <100ms for typical cache sizes
- Display is readable on various terminal widths
- Error messages guide users to solutions
- Documentation covers all new features

## Phase 5: Integration and Polish

### Input Resources
- All previous phase deliverables
- Integration test results
- Code review feedback

### Deliverables
1. **Cross-Feature Integration Tests** (`/tests/cache_search_integration.rs`)
   - Combined filter and display mode tests
   - Edge case handling across features
   - Regression test suite
   - Platform-specific behavior validation

2. **Code Cleanup and Refactoring**
   - Remove deprecated code
   - Consolidate duplicate logic
   - Improve code organization
   - Update inline documentation

3. **Final Documentation**
   - Complete API documentation
   - Update CHANGELOG.md
   - Create migration guide for users
   - Add advanced usage examples

### Success Criteria
- All tests pass on Linux, macOS, and Windows
- Code meets quality standards (fmt, clippy, test coverage)
- Documentation is complete and accurate
- No regressions from current functionality

## Implementation Guidelines

### For Each Phase:
1. Start with `/clear` command to reset context
2. Load this plan.md and relevant design docs
3. Implement features incrementally with tests
4. Run quality checks after each change:
   - `cargo fmt`
   - `cargo clippy`
   - `cargo check`
   - `cargo test`
5. Commit completed phase before proceeding

### Testing Strategy

#### Unit Tests
- Test individual functions in isolation
- Mock external dependencies where needed
- Focus on edge cases and error conditions
- Ensure high code coverage

#### Integration Tests
- Test complete search workflows
- Use real cached metadata
- Verify actual output formatting
- Test across different platforms

### Priority Considerations
1. **High Priority**: Display improvements (Phase 1)
   - Directly improves user experience
   - Foundation for other improvements
   - Relatively straightforward implementation

2. **Medium Priority**: Flexible queries (Phase 2) and Filtering (Phase 3)
   - Enables key use cases
   - Builds on Phase 1 changes
   - Moderate complexity

3. **Low Priority**: Performance and Polish (Phase 4 & 5)
   - Refinements and optimizations
   - Can be done incrementally
   - Lower user impact

### Related Commands
- **Platform Information**: The `kopi doctor` command will display the detected platform (OS, architecture, LibC type) along with other diagnostic information. This is more appropriate than a dedicated cache subcommand since platform detection is a system-wide concern.

### Backward Compatibility
- Existing search queries must continue to work
- Default behavior should feel familiar to users
- New features are opt-in via flags
- Clear migration path for any breaking changes

## Next Steps
Begin with Phase 1, focusing on the display improvements that will immediately enhance the user experience and lay the groundwork for subsequent phases.