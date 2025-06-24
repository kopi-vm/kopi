# ADR-003: JDK Storage Format for Toolchain Compatibility

## Status
Proposed

## Context
Kopi needs to store downloaded JDKs in a format that is compatible with existing build tool toolchain mechanisms, specifically Gradle and Maven. These tools have their own JDK discovery mechanisms that expect certain directory structures and naming conventions.

### Gradle Toolchain Requirements
- Automatically discovers JDKs in platform-specific directories
- Supports manual configuration via `org.gradle.java.installations.paths`
- Identifies JDKs by version, vendor, and architecture
- Prefers JDKs over JREs

### Maven Toolchain Requirements
- Uses `toolchains.xml` configuration file
- Requires explicit `<jdkHome>` paths
- Identifies JDKs by version and vendor attributes
- Supports environment variables like `JAVA17_HOME`

### Existing Version Manager Conventions
- SDKMAN: `~/.sdkman/candidates/java/<version>-<vendor>`
- Jabba: `~/.jabba/jdk/<vendor>@<version>`
- jenv: Uses symlinks to system-installed JDKs

## Decision

Adopt the following JDK storage format:

### Directory Structure
```
~/.kopi/jdks/<vendor>-<version>-<arch>/
```

Examples:
- `~/.kopi/jdks/temurin-17.0.9-x64/`
- `~/.kopi/jdks/corretto-11.0.21-aarch64/`
- `~/.kopi/jdks/zulu-21.0.1-x64/`

### Platform-Specific Adaptations
- **macOS**: Preserve `.jdk/Contents/Home` structure for compatibility
- **Linux/Windows**: Standard JDK structure directly under version directory

### Metadata Storage
Store JDK metadata in `.kopi-metadata.json` within each JDK directory:
```json
{
  "vendor": "temurin",
  "version": "17.0.9",
  "architecture": "x64",
  "os": "linux",
  "java_version": "17",
  "java_home": "/home/user/.kopi/jdks/temurin-17.0.9-x64",
  "installation_date": "2024-01-15T10:30:00Z",
  "foojay_distribution": "temurin",
  "foojay_package_type": "jdk"
}
```

### Global Configuration for Toolchain Integration

Kopi will use a global configuration file `~/.kopi/config.toml` to manage toolchain integrations. These integrations will be enabled by default.

#### Configuration File Structure:
```toml
[toolchains]
# Enable automatic toolchain integration (default: true)
enabled = true

[toolchains.gradle]
# Automatically update Gradle properties (default: true)
auto_configure = true
# Path to gradle.properties file (default: ~/.gradle/gradle.properties)
properties_file = "~/.gradle/gradle.properties"
# Create symlinks in standard directories (default: true)
create_symlinks = true

[toolchains.maven]
# Automatically update Maven toolchains.xml (default: true)
auto_configure = true
# Path to toolchains.xml file (default: ~/.m2/toolchains.xml)
toolchains_file = "~/.m2/toolchains.xml"
# Export version-specific environment variables (default: true)
export_java_home_vars = true

[toolchains.vendor_mapping]
# Map Foojay distribution names to toolchain vendor names
temurin = ["temurin", "eclipse", "adoptium"]
corretto = ["corretto", "amazon"]
zulu = ["zulu", "azul"]
liberica = ["liberica", "bellsoft"]
microsoft = ["microsoft"]
oracle = ["oracle"]
graalvm_ce22 = ["graalvm", "graalvm-ce"]
```

#### Default Behavior:
1. **On JDK Installation**: Automatically update Gradle and Maven configurations
2. **On JDK Removal**: Clean up references from toolchain configurations
3. **On Kopi Init**: Create default configuration file with integrations enabled

#### For Gradle:
1. Automatically append to `~/.gradle/gradle.properties`:
   ```properties
   # Kopi-managed JDKs
   org.gradle.java.installations.paths=/home/user/.kopi/jdks/temurin-17.0.9-x64,/home/user/.kopi/jdks/corretto-11.0.21-x64
   ```
2. Create platform-specific symlinks for auto-discovery
3. Update configurations when JDKs are added/removed

#### For Maven:
1. Automatically generate/update `~/.m2/toolchains.xml`
2. Export environment variables in shell initialization scripts
3. Map Foojay distribution names to Maven vendor identifiers

## Consequences

### Positive
- **Compatibility**: Works with existing Gradle and Maven toolchain mechanisms
- **Discoverability**: JDKs can be found by build tools without modification
- **Clarity**: Clear naming convention includes vendor, version, and architecture
- **Flexibility**: Supports multiple integration methods for different user preferences
- **Consistency**: Similar to established version manager conventions

### Negative
- **Storage overhead**: Longer directory names than some alternatives
- **Migration complexity**: Users of other tools may need to adjust paths
- **Maintenance**: Must maintain vendor name mappings between Foojay and toolchains

### Neutral
- **Automatic configuration**: Toolchain integration happens automatically by default
- **Platform differences**: macOS requires special handling for JDK structure
- **Configuration flexibility**: Users can disable or customize integration via config file

## Implementation Notes

1. **Registry File**: Maintain `~/.kopi/registry.json` for quick JDK lookups
2. **Configuration Management**: Create and manage `~/.kopi/config.toml` with sensible defaults
3. **Atomic Operations**: Use temporary directories during installation
4. **Verification**: Validate JDK after installation with `java -version`
5. **Toolchain Updates**: Automatically update Gradle/Maven configurations on JDK changes
6. **Rollback Support**: Backup toolchain configurations before modifications

## References
- [Gradle Toolchains Documentation](https://docs.gradle.org/current/userguide/toolchains.html)
- [Maven Toolchains Plugin](https://maven.apache.org/guides/mini/guide-using-toolchains.html)
- [Foojay DiscoAPI](https://github.com/foojayio/discoapi)