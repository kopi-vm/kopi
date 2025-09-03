# Version Parser Enhancement Plan

## Overview

This plan outlines the work required to enhance Kopi's version parser to support multiple JDK distribution version formats, particularly Amazon Corretto's 4-component format and other extended formats discovered in the foojay.io API.

## Background

Based on the investigation in `/docs/reviews/2025-07-11-corretto-version-format.md`:

- Current parser only supports up to 3 components
- Amazon Corretto uses 4-5 components (e.g., `21.0.7.6.1`)
- Alibaba Dragonwell uses 6 components (e.g., `21.0.7.0.7.6`)
- Other distributions have their own unique formats
- Users cannot search by distribution_version, only java_version

## Implementation Plan

> **Note**: Documentation is being completed as Phase 1 to ensure all design decisions and expected behaviors are clearly defined before implementation begins. This "documentation-first" approach helps prevent misunderstandings and serves as a specification for the implementation phases.

### Phase 1: Documentation Updates (Completed)

#### 1.1 Architecture Decision Record

- **Task**: Create new ADR for version format changes
- **File**: `/docs/adr/archive/016-flexible-version-format.md`
- **Content**:
  - Document the decision to support N-component versions
  - Explain the new flexible version structure
  - Detail positive/negative consequences
  - Include implementation examples for all distribution formats

#### 1.2 Update Existing Documentation

- **Files updated**:
  - `/docs/reference.md`:
    - Added "Extended Version Formats" section with distribution examples
    - Added "Version Search Behavior" section explaining auto-detection
    - Added "Version Pattern Matching" section for partial matches
    - Updated version specification examples to include 4-6 component formats
  - `/docs/adr/archive/014-configuration-and-version-file-formats.md`:
    - Added extended format examples in `.kopi-version` section
    - Added migration mapping examples for Corretto and Dragonwell
    - Added note about support for extended version formats
  - `/docs/tasks/archive/uninstall/design.md`:
    - Added "Version Pattern Matching" subsection
    - Included examples for 4-6 component version matching
    - Explained flexible matching for all distribution formats
  - `/docs/tasks/archive/search/design.md`:
    - Added comprehensive "Enhanced Version Search Capabilities" section
    - Documented automatic version type detection rules
    - Added manual override options (--java-version, --distribution-version)
    - Included distribution-specific version format examples

#### 1.3 User Documentation

- **Task**: Document new version search capabilities
- **Content documented**:
  - **Automatic Version Detection**:
    - Rules for detecting java_version vs distribution_version
    - Format-based auto-detection (4+ components, build numbers)
  - **Manual Override Options**:
    - `--java-version` flag for forcing java_version search
    - `--distribution-version` flag for forcing distribution_version search
  - **Distribution Examples**:
    - Comprehensive table showing java_version vs distribution_version for each distribution
    - Real-world examples from Temurin, Corretto, Dragonwell, JetBrains, GraalVM
  - **Use Cases**:
    - Finding specific Corretto patch versions
    - Handling multiple versions with same java_version
    - Installing exact distribution versions

#### 1.4 Documentation Gaps Identified

- **Not Required**: `/README.md` doesn't exist at the project root
- **Future Work**: Migration guide for existing users (depends on implementation)

### Phase 2: Refactoring and Structure Changes

#### 2.1 Module Reorganization

- **Task**: Move `src/models/version.rs` to `src/version/mod.rs`
- **Rationale**: Better organization for version-related functionality
- **Actions**:
  - Create `src/version/` directory
  - Move version.rs to mod.rs
  - Update all imports throughout the codebase
  - Update `src/models/mod.rs` to remove version module

#### 2.2 Version Structure Redesign

- **Task**: Replace fixed 3-component structure with flexible N-component design
- **New Structure**:
  ```rust
  pub struct Version {
      pub components: Vec<u32>,        // All numeric components
      pub build: Option<Vec<u32>>,     // Build numbers as numeric array
      pub pre_release: Option<String>, // Pre-release string
  }
  ```

### Phase 3: Parser Implementation

#### 3.1 Enhanced Version Parser

- **Task**: Implement new `FromStr` trait for flexible version parsing
- **Features**:
  - Support unlimited numeric components separated by `.`
  - Support build numbers after `+` (can be multi-component)
  - Support pre-release after `-`
  - Handle edge cases like Corretto Java 8 (`8.452.9.1` without leading zero)

#### 3.2 Backward Compatibility

- **Task**: Maintain compatibility with existing code
- **Actions**:
  - Keep helper methods: `major()`, `minor()`, `patch()`
  - Update `new()` constructor to use new structure
  - Ensure existing tests pass

### Phase 4: Version Matching Enhancement

#### 4.1 Pattern Matching Logic

- **Task**: Update `matches_pattern()` for flexible components
- **Logic**:
  - Compare components up to the length specified in pattern
  - Support partial matching (e.g., "21" matches "21.0.7.6.1")
  - Handle build number matching if specified

#### 4.2 Search Enhancement

- **Task**: Support searching by distribution_version
- **Features**:
  - Auto-detect version type based on format
  - Allow users to specify `--java-version` or `--distribution-version` flags
  - Fallback search strategy
- **Important Change**: Parse distribution_version as Version struct (currently kept as string)

### Phase 5: Code Updates

#### 5.1 Update Version Usage

- **Files to update**:
  - `src/cache/mod.rs` - Update Package struct to store distribution_version as Version instead of String; parse both java_version and distribution_version
  - `src/commands/install.rs` - Version validation
  - `src/uninstall/mod.rs` - Version pattern matching
  - `src/storage/listing.rs` - Parse versions from installed JDK directory names
  - `src/search/searcher.rs` - Version filtering (update to parse distribution_version)

#### 5.2 Update Tests

- **Test files to update**:
  - `src/version/mod.rs` - Unit tests for new parser
  - `tests/uninstall_integration.rs` - Fix Corretto test expectations
  - `tests/shim_security.rs` - Update version strings
  - Add new test cases for all discovered formats

### Phase 6: Testing and Validation

#### 6.1 Unit Tests

- **Test cases**:
  - Corretto 4-5 component versions
  - Dragonwell 6 component versions
  - JetBrains large build numbers
  - GraalVM complex build identifiers
  - Edge cases (empty components, invalid formats)

#### 6.2 Integration Tests

- **Scenarios**:
  - Install Corretto with full version
  - Uninstall with partial version patterns
  - List and display extended versions
  - Search by distribution_version

#### 6.3 Manual Testing

- **Distributions to test**:
  - Amazon Corretto (all Java versions)
  - Alibaba Dragonwell
  - IBM Semeru
  - JetBrains Runtime
  - GraalVM CE

## Implementation Order

1. **Documentation** (Phase 1) - Update all docs
2. **Refactoring** (Phase 2.1) - Move version module
3. **Core Implementation** (Phase 2.2, 3.1, 3.2) - New version structure and parser
4. **Matching Logic** (Phase 4.1) - Update pattern matching
5. **Search Enhancement** (Phase 4.2) - Add distribution_version search
6. **Code Updates** (Phase 5.1, 5.2) - Update all usage points
7. **Testing** (Phase 6) - Comprehensive testing

## Risk Mitigation

### Backward Compatibility

- All existing version strings must continue to work
- Existing API must remain stable
- Configuration files must remain compatible

### Performance Considerations

- Version comparison may be slower with dynamic components
- Consider caching parsed versions in hot paths
- Profile before and after changes

### Error Handling

- Clear error messages for invalid formats
- Graceful fallback for unexpected formats
- Detailed logging for debugging

## Success Criteria

1. All existing tests pass without modification
2. Corretto versions parse correctly
3. Version pattern matching works for all distributions
4. Users can search by distribution_version
5. Documentation is complete and accurate
6. No performance regression in version operations

## Timeline Estimate

- Phase 1: 1-2 hours (Documentation)
- Phase 2-3: 2-3 hours (Core implementation)
- Phase 4-5: 3-4 hours (Integration and updates)
- Phase 6: 2-3 hours (Testing)

Total: 8-12 hours of focused development
