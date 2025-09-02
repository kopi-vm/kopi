# ADR-016: Flexible Version Format Support

## Status
Proposed

## Context

During the development of uninstall functionality and while fixing Windows test failures, we discovered that Amazon Corretto uses a 4-5 component version format (e.g., `21.0.7.6.1`) that differs from the standard 3-component OpenJDK format (e.g., `21.0.7+6`). Further investigation of the foojay.io API revealed even more complex formats:

- **Alibaba Dragonwell**: 6 components (e.g., `21.0.7.0.7.6`)
- **Amazon Corretto**: 4-5 components (e.g., `21.0.7.6.1`)
- **JetBrains Runtime**: Very large build numbers (e.g., `21.0.7+895130`)
- **Corretto Java 8**: Non-standard format without leading zeros (e.g., `8.452.9.1`)

The current Kopi version parser only supports up to 3 components, explicitly rejecting versions with more components. This limitation causes:

1. **Parsing failures**: Corretto and Dragonwell versions cannot be parsed
2. **Version matching issues**: Commands like `kopi uninstall 21` fail to match Corretto installations
3. **Search limitations**: Users cannot search by `distribution_version`, only by `java_version`
4. **Test failures**: Integration tests expecting Corretto versions fail

## Decision

We will replace the current fixed 3-component version structure with a flexible N-component design that can handle any number of version components. The new version structure will:

1. Store all numeric components in a dynamic vector
2. Support optional build numbers (after `+`) as a separate vector
3. Support optional pre-release identifiers (after `-`)
4. Maintain backward compatibility through helper methods
5. Enable searching by both `java_version` and `distribution_version`

### New Version Structure

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub components: Vec<u32>,        // All numeric components (unlimited)
    pub build: Option<Vec<u32>>,     // Build numbers as numeric array
    pub pre_release: Option<String>, // Pre-release string
}
```

### Key Design Choices

1. **Unlimited Components**: No artificial limit on the number of version components
2. **Numeric Build Numbers**: Build numbers are parsed as numeric arrays for proper comparison
3. **Automatic Format Detection**: The system will auto-detect whether a version string is a `java_version` or `distribution_version` based on its format
4. **Backward Compatibility**: Existing `major()`, `minor()`, `patch()` methods will continue to work

## Consequences

### Positive

1. **Universal Distribution Support**: Can handle all JDK distributions found in foojay.io
2. **Future-Proof**: No need to modify the parser when new distributions use different formats
3. **Better User Experience**: Users can search and install using distribution-specific versions
4. **Accurate Version Matching**: Numeric comparison works correctly for all components
5. **Maintains Compatibility**: Existing code using the version API continues to work

### Negative

1. **Increased Complexity**: Version comparison logic becomes more complex with variable components
2. **Performance Impact**: Dynamic vectors may have slight overhead compared to fixed fields
3. **Migration Effort**: All code using the Version struct needs careful review and testing

### Neutral

1. **Memory Usage**: Variable-length vectors use memory proportional to version complexity
2. **API Surface**: The public API remains largely unchanged, with internal implementation changes

## Implementation Notes

### Version Parsing Examples

The new parser will handle all these formats:

```
"21.0.7.6.1"     → components: [21, 0, 7, 6, 1]
"21.0.7.0.7.6"   → components: [21, 0, 7, 0, 7, 6]  
"21.0.7+6"       → components: [21, 0, 7], build: Some([6])
"21.0.1+12.1"    → components: [21, 0, 1], build: Some([12, 1])
"8.452.9.1"      → components: [8, 452, 9, 1]
"21.0.7-rc.1"    → components: [21, 0, 7], pre_release: Some("rc.1")
```

### Version Matching Logic

Pattern matching will compare components up to the length specified in the pattern:

- Pattern `"21"` matches `"21.0.7.6.1"` (compares first component)
- Pattern `"21.0"` matches `"21.0.7.6.1"` (compares first two components)
- Pattern `"21.0.7"` matches `"21.0.7.6.1"` (compares first three components)
- Pattern `"21.0.7.6"` matches `"21.0.7.6.1"` (compares first four components)

### Search Enhancement

Users will be able to search by distribution version:

```bash
# Auto-detects as distribution_version (4+ components)
kopi install corretto@21.0.7.6.1

# Auto-detects as java_version (has build number)
kopi install temurin@21.0.7+6

# Manual override if needed
kopi install corretto@21.0.7 --distribution-version
```

## References

- [Corretto Version Format Investigation](/docs/reviews/2025-07-11-corretto-version-format.md)
- [foojay.io API Documentation](https://api.foojay.io/swagger/index.html)
- [Version Parser Enhancement Plan](/docs/tasks/archive/version/plan.md)
- [Original Version Parser Implementation](/src/models/version.rs:127)