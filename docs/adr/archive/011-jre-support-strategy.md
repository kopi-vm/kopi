# ADR-011: JRE Support Strategy

## Status

Proposed

## Context

Kopi currently focuses exclusively on JDK (Java Development Kit) management. However, there are valid use cases for JRE (Java Runtime Environment) support:

1. **Production Environments**: JREs are lighter weight (~50% smaller) and more secure for running applications
2. **CI/CD Pipelines**: Test execution environments often only need JRE capabilities
3. **Container Images**: Smaller JRE-based images reduce deployment size and attack surface
4. **User Flexibility**: Some users only need to run Java applications, not develop them

The foojay.io API already provides JRE packages alongside JDK packages, distinguished by the `package_type` field in API responses. Our current implementation filters for JDK packages only and ignores available JRE packages.

## Decision Drivers

1. **User Needs**: Support both development and runtime use cases
2. **API Compatibility**: Leverage existing foojay.io JRE package availability
3. **Backward Compatibility**: Ensure existing JDK-only workflows continue to work
4. **Clear Semantics**: Make JDK vs JRE selection explicit and understandable
5. **Storage Organization**: Maintain clear separation between JDKs and JREs
6. **Implementation Simplicity**: Minimize code changes and complexity

## Considered Options

### Option 1: Suffix Notation

```
temurin@21-jre
corretto@17.0.9-jre
21-jre
```

**Advantages:**

- Natural extension of existing version syntax
- Similar to version tags like `-ea` or `-lts`

**Disadvantages:**

- Could be confused with version suffixes
- Parsing complexity with existing version patterns

### Option 2: Prefix Notation (Jabba Style)

```
jre@temurin@21
jre@17.0.9
jre@corretto@17
```

**Advantages:**

- Clear distinction that this is a JRE
- Similar to Jabba's `sjre@` prefix for Server JRE
- Easy to parse (check prefix before passing to version parser)
- Backward compatible (no prefix = JDK)

**Disadvantages:**

- Double `@` symbol might be confusing
- Slightly longer syntax

### Option 3: Separate Parameter

```
temurin@21 --type=jre
17.0.9 --jre
```

**Advantages:**

- Clear separation of version and package type
- Familiar command-line pattern

**Disadvantages:**

- Cannot be used in `.kopi-version` files
- Inconsistent with version-only specification

## Decision

We will adopt **Option 2: Prefix Notation** using the `jre@` prefix, following Jabba's established pattern, with an additional `jdk@` prefix for explicit JDK specification to ensure consistency.

### Version Specification Format

#### Command Line

```bash
# JDK (implicit default - backward compatible)
kopi install temurin@21
kopi install 17.0.9

# JDK (explicit)
kopi install jdk@temurin@21
kopi install jdk@17.0.9
kopi install jdk@corretto@17

# JRE
kopi install jre@temurin@21
kopi install jre@17.0.9
kopi install jre@corretto@17
```

#### .kopi-version File

```
# JDK (implicit default - backward compatible)
temurin@21
17.0.9

# JDK (explicit)
jdk@temurin@21
jdk@17.0.9

# JRE
jre@temurin@21
jre@17.0.9
```

### Directory Layout

JREs will be stored in a separate `jres/` directory parallel to `jdks/`:

```
~/.kopi/
├── jdks/
│   ├── temurin-21.0.1/
│   └── corretto-17.0.2/
├── jres/
│   ├── temurin-21.0.1/
│   └── corretto-17.0.2/
├── bin/
├── cache/
└── config.toml
```

**Rationale:**

- Maintains backward compatibility with existing `jdks/` directory
- Clear separation between JDKs and JREs
- Simplifies listing and management operations
- Prevents accidental mixing of JDK and JRE installations
- Consistent prefix notation: both `jdk@` and `jre@` are supported
- No prefix defaults to JDK for backward compatibility

## Implementation Plan

### Phase 1: Core Support

1. Add `package_type` field to API models
2. Extend `VersionParser` to handle `jre@` prefix
3. Update `ParsedVersionRequest` to include `PackageType`
4. Modify API client to accept package type parameter

### Phase 2: Storage Support

1. Add `jres_dir()` method to `JdkRepository`
2. Update installation paths based on package type
3. Extend metadata to include package type information
4. Update listing commands to show both JDKs and JREs

### Phase 3: Command Integration

1. Update all commands to respect package type
2. Add package type indicator to list output
3. Update shell integration to handle both types
4. Extend `.kopi-version` file parsing

### Example Implementation

```rust
// Version parsing with JDK/JRE support
pub fn parse_version_spec(input: &str) -> Result<(ParsedVersionRequest, PackageType)> {
    let trimmed = input.trim();

    if let Some(spec) = trimmed.strip_prefix("jre@") {
        let parsed = VersionParser::parse(spec)?;
        Ok((parsed, PackageType::Jre))
    } else if let Some(spec) = trimmed.strip_prefix("jdk@") {
        let parsed = VersionParser::parse(spec)?;
        Ok((parsed, PackageType::Jdk))
    } else {
        // Default to JDK for backward compatibility
        let parsed = VersionParser::parse(trimmed)?;
        Ok((parsed, PackageType::Jdk))
    }
}

// Storage path determination
pub fn package_install_path(&self, package_type: &PackageType,
                           distribution: &Distribution,
                           version: &str) -> PathBuf {
    let dir = match package_type {
        PackageType::Jdk => self.jdks_dir(),
        PackageType::Jre => self.jres_dir(),
    };
    dir.join(format!("{}-{}", distribution.id(), version))
}
```

## Consequences

### Positive

- **Expanded Use Cases**: Support for production and runtime environments
- **Reduced Footprint**: Smaller installations for runtime-only needs
- **Security**: JREs have smaller attack surface for production use
- **Flexibility**: Users can choose appropriate package type for their needs
- **Clear Semantics**: `jre@` and `jdk@` prefixes make intent explicit
- **Consistency**: Symmetric prefix notation for both package types

### Negative

- **Increased Complexity**: Two package types to manage and test
- **Storage Overhead**: Separate directories for JDKs and JREs
- **Migration**: Existing users need to understand new syntax
- **Tooling Updates**: Shell completions and integrations need updates

### Neutral

- **Documentation**: Requires clear explanation of when to use JDK vs JRE
- **Testing**: Doubles the test matrix for package operations
- **Cache Management**: Metadata cache must distinguish package types

## Future Considerations

1. **Server JRE Support**: Could add `sjre@` prefix for server JREs
2. **Automatic Selection**: Could default to JRE for `java` command, JDK for `javac`
3. **Size Optimization**: Could share common files between JDK and JRE of same version
4. **Package Conversion**: Could support extracting JRE from installed JDK

## References

- [Jabba Version Manager](https://github.com/shyiko/jabba) - Inspiration for prefix notation
- [foojay.io API Documentation](https://api.foojay.io/swagger-ui/) - Package type support
- [ADR-003: JDK Storage Format](./003-jdk-storage-format.md) - Current storage structure
