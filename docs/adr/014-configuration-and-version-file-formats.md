# ADR-014: Configuration and Version File Formats

## Status
Proposed

## Context
Kopi needs to support both project-specific and global configuration for Java version management. We must consider:

1. Compatibility with existing tools (jenv, asdf-java, GitHub Actions setup-java)
2. Support for distribution selection alongside version specification
3. Clear migration paths from existing tools
4. Avoiding ambiguity in version specifications
5. Development-focused usage (not production environments)

### Existing Tool Analysis

**jenv** uses `.java-version` files with formats like:
- `11` (major version)
- `11.0.15` (full version)
- `openjdk64-11.0.15` (distribution-arch-version with `-` separator)

**asdf-java** uses `.tool-versions` files with formats like:
- `java temurin-21.0.1+12`
- `java corretto-17.0.5.8.1`

**GitHub Actions setup-java** supports `.java-version` files containing:
- Simple version numbers: `11`, `17`, `21`
- Full versions: `11.0.15`, `21.0.1`

### Problems with Existing Formats

Both jenv and asdf use `-` as a separator between distribution and version, which creates ambiguity because Java version strings can contain `-`:
- `21-ea` (early access)
- `22-ea+27-2262` (early access with build info)
- `11.0.2+9-LTS` (LTS tag)

## Decision

### Version File Formats

1. **`.java-version`** (Compatibility Mode)
   - Support existing formats for compatibility
   - Simple version numbers only: `21`, `11.0.2`, `21-ea`
   - No distribution specification
   - Maintains compatibility with GitHub Actions and other tools

2. **`.kopi-version`** (Native Format)
   - Uses `@` as separator: `distribution@version`
   - Examples:
     - `temurin@21`
     - `corretto@11.0.2+9`
     - `zulu@21-ea+35`
   - Clear separation without ambiguity
   - Default distribution used when only version specified
   - **No version ranges**: Does not support Maven-style (`[1.7,1.8)`, `[1.5,)`) or npm-style (`^1.2.3`, `~1.2.3`, `>=1.2.3`) version specifications
   - **Exact versions only**: Must specify precise version numbers

### Version Resolution Behavior

When a major version only is specified (e.g., `21`), kopi will:
- Automatically select the latest available minor and patch version
- For example, `21` might resolve to `21.0.2+13` if that's the latest available
- This provides convenience while maintaining reproducibility once installed
- The actual installed version is recorded for future reference

### Configuration Hierarchy

Version resolution order (highest to lowest priority):
1. Environment variable: `KOPI_JAVA_VERSION`
2. `.kopi-version` file (walk up directory tree)
3. `.java-version` file (walk up directory tree, compatibility)
4. Global configuration (`~/.kopi/config.toml`)

### Migration Strategy

Provide migration commands for existing users:

```bash
# Auto-detect and migrate
kopi migrate

# Specific migrations
kopi migrate jenv     # Migrate from jenv
kopi migrate asdf     # Migrate from asdf

# Options
kopi migrate --keep-original  # Preserve original files
kopi migrate --dry-run       # Preview changes
kopi migrate --recursive     # Handle monorepos
```

Migration mapping examples:
- `openjdk64-11.0.15` → `temurin@11.0.15`
- `corretto-17.0.5.8.1` → `corretto@17.0.5.8.1`

### Design Principles

1. **No Environment Variables in Config**: Since kopi uses shims, environment variables set in config files won't affect parent processes (like gradlew)

2. **No Production Features**: Focus on development environments only. Production uses containers.

3. **No Build Tool Management**: Respect gradle wrapper and maven wrapper. Don't complicate the ecosystem.

4. **No Hooks**: Avoid environment complexity. Keep it simple.

5. **Clean Design Over Legacy Compatibility**: Provide migration tools rather than perpetuating ambiguous formats.

6. **Exact Versions Only**: No support for version ranges or constraints. This decision keeps the tool simple and predictable:
   - No Maven-style ranges: `[1.7,1.8)`, `(,1.8]`, `[1.5,)`
   - No npm-style ranges: `^1.2.3`, `~1.2.3`, `>=1.2.3 <2.0.0`
   - No wildcards: `21.*`, `11.0.*`
   - Rationale: Development environments benefit from exact, reproducible versions

## Consequences

### Positive
- Clear, unambiguous version specification format
- Smooth migration path from existing tools
- Maintains compatibility where appropriate
- Simple design focused on core functionality
- Avoids technical debt from ambiguous formats
- No complex project configuration to manage

### Negative
- Users must run migration commands to adopt kopi
- Temporary duplication of version files during migration
- New format to learn (though more intuitive)
- No support for advanced version constraints (must use exact versions)

### Neutral
- Multiple configuration files may exist during transition period
- Need to maintain migration code for existing formats