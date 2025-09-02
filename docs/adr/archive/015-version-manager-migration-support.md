# ADR-015: Version Manager Migration Support

## Status
Proposed

## Context
Many developers already use other Java version managers like jenv, asdf, or sdkman. To facilitate adoption of Kopi, we need to provide a smooth migration path that allows users to:
- Convert existing version files to Kopi format
- Preserve their project configurations
- Handle monorepo scenarios with multiple projects
- Map vendor-specific naming conventions to Kopi's distribution format

## Decision

### Migration Command Structure

Implement a `kopi migrate` command with the following capabilities:

```bash
kopi migrate                             # Auto-detect and migrate
kopi migrate jenv                        # Migrate from jenv
kopi migrate asdf                        # Migrate from asdf
```

### Command Options

- `--keep-original`: Preserve original version files after migration
- `--dry-run`: Preview changes without applying them
- `--recursive`: Handle monorepos (migrate all subdirectories)

### Usage Examples

```bash
# Migrate from jenv, keeping original files
kopi migrate jenv --keep-original

# Preview migration from asdf
kopi migrate asdf --dry-run

# Migrate entire monorepo
kopi migrate --recursive
```

### Migration Mappings

#### jenv Migration
- File: `.java-version`
- Format: `openjdk64-11.0.15` → `temurin@11.0.15`
- Common patterns:
  - `openjdk64-X.Y.Z` → `temurin@X.Y.Z`
  - `oracle64-X.Y.Z` → `oracle@X.Y.Z`
  - `graalvm64-X.Y.Z` → `graalvm@X.Y.Z`

#### asdf Migration
- File: `.tool-versions`
- Format: `java temurin-21.0.1+12` → `temurin@21.0.1+12`
- Common patterns:
  - `java temurin-X.Y.Z` → `temurin@X.Y.Z`
  - `java corretto-X.Y.Z` → `corretto@X.Y.Z`
  - `java zulu-X.Y.Z` → `zulu@X.Y.Z`

#### sdkman Migration (Future)
- File: `.sdkmanrc`
- Format: `java=21.0.1-tem` → `temurin@21.0.1`
- Vendor mappings:
  - `-tem` → `temurin`
  - `-amzn` → `corretto`
  - `-zulu` → `zulu`
  - `-oracle` → `oracle`

### Implementation Details

1. **Auto-detection Logic**
   - Check for `.java-version` (jenv)
   - Check for `.tool-versions` with java entry (asdf)
   - Check for `.sdkmanrc` (sdkman)
   - Use first found, or prompt if multiple exist

2. **Migration Process**
   - Read source version file
   - Parse version and vendor information
   - Map to Kopi's `distribution@version` format
   - Create `.kopi-version` file
   - Optionally remove or preserve original

3. **Monorepo Support**
   - Walk directory tree when `--recursive` flag is used
   - Skip already migrated directories (those with `.kopi-version`)
   - Provide summary of all migrations performed

4. **Error Handling**
   - Unknown vendor mappings → suggest closest match or use default distribution
   - Invalid version formats → show clear error with expected format
   - Write conflicts → prompt user for action

## Rationale

1. **Smooth Adoption**: Reduces friction for users switching to Kopi
2. **Preserve Investment**: Respects existing project configurations
3. **Flexibility**: Options for dry-run and keeping originals reduce risk
4. **Monorepo-Friendly**: Recursive option handles complex project structures
5. **Clear Mappings**: Transparent vendor name conversions

## Consequences

### Positive
- Easy migration path for existing users
- Maintains project version consistency
- Supports gradual migration in large codebases
- Clear audit trail with dry-run option

### Negative
- Need to maintain mapping tables for various tools
- Version format edge cases may require updates
- Additional code complexity for parsing different formats

## Implementation Priority

This is a Phase 3 feature, to be implemented after core functionality is stable. It's not critical for initial release but important for adoption.

## References
- jenv: https://www.jenv.be/
- asdf-java: https://github.com/halcyon/asdf-java
- SDKMAN!: https://sdkman.io/