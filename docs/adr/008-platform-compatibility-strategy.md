# ADR-008: Platform Compatibility Strategy

## Status
Proposed

## Context
Alpine Linux uses musl libc instead of glibc, creating binary compatibility challenges for JDK distributions. Research of similar tools (SDKMAN, volta, nvm, pyenv) and Gradle's approach reveals that most version managers struggle with Alpine support, often leading to confusing user experiences when incompatible binaries are installed.

The key issue is that Alpine binaries (musl-linked) cannot run on standard Linux systems (glibc-linked) and vice versa without compatibility layers, which introduce performance penalties and potential runtime issues.

For macOS and Windows, the libc compatibility issue doesn't apply as they use their own system libraries (libSystem on macOS, MSVCRT on Windows). However, JDK distributions provide platform-specific builds for these operating systems.

A key insight is that on Linux systems, kopi itself must be linked against either musl or glibc, and this creates an opportunity: by detecting kopi's own libc linkage, we can ensure that all JDKs managed by kopi use the same libc type, guaranteeing compatibility.

## Decision

### Platform Detection Strategy
Kopi will detect its own libc linkage and ensure downloaded JDKs match the same libc type. This approach guarantees binary compatibility between kopi and the JDKs it manages.

#### Primary Approach: Self-Detection
```rust
// Platform-specific libc detection
#[cfg(all(target_os = "linux", target_env = "musl"))]
const PLATFORM_LIBC: &str = "musl";

#[cfg(all(target_os = "linux", target_env = "gnu"))]
const PLATFORM_LIBC: &str = "glibc";

#[cfg(target_os = "macos")]
const PLATFORM_LIBC: &str = "darwin";  // macOS uses its own system libraries

#[cfg(target_os = "windows")]
const PLATFORM_LIBC: &str = "windows";  // Windows uses MSVCRT

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
const PLATFORM_LIBC: &str = "unknown";



// Get Foojay API lib_c_type value for current platform
fn get_foojay_libc_type() -> &'static str {
    match PLATFORM_LIBC {
        "musl" => "musl",
        "glibc" => "libc",  // Foojay uses "libc" for glibc
        "darwin" => "libc",  // macOS uses "libc" in Foojay API
        "windows" => "c_std_lib",  // Windows uses "c_std_lib" in Foojay API
        _ => "libc"  // Default fallback
    }
}

// Match against Foojay API lib_c_type values
fn matches_foojay_libc_type(foojay_libc: &str) -> bool {
    match (PLATFORM_LIBC, foojay_libc) {
        ("musl", "musl") => true,
        ("glibc", "libc") | ("glibc", "glibc") => true,
        ("darwin", "libc") => true,  // macOS uses "libc" in Foojay API
        ("windows", "c_std_lib") => true,  // Windows uses "c_std_lib" in Foojay API
        _ => false
    }
}
```

#### Usage Example
Example of how these functions work together:

```rust
// Main function to get the lib_c_type for Foojay API queries
pub fn get_required_libc_type() -> &'static str {
    get_foojay_libc_type()
}

// Example API query construction
fn build_foojay_query(version: &str, distribution: &str) -> String {
    let libc_type = get_required_libc_type();
    format!(
        "https://api.foojay.io/disco/v3.0/packages?\
         version={}&distribution={}&lib_c_type={}&archive_type=tar.gz",
        version, distribution, libc_type
    )
}

// Example validation when downloading JDK
fn validate_jdk_metadata(metadata: &JdkMetadata) -> Result<()> {
    if let Some(libc_type) = &metadata.lib_c_type {
        if !matches_foojay_libc_type(libc_type) {
            bail!(
                "JDK lib_c_type '{}' is not compatible with kopi's platform '{}'",
                libc_type, PLATFORM_LIBC
            );
        }
    }
    Ok(())
}

```

### Architecture Naming Convention
To prevent accidental cross-usage of incompatible binaries:
- Linux Alpine/musl variants: `linux-x64-musl`, `linux-aarch64-musl`
- Linux standard/glibc variants: `linux-x64`, `linux-aarch64`
- macOS variants: `macos-x64`, `macos-aarch64`
- Windows variants: `windows-x64`, `windows-aarch64`

### JDK Distribution Support
Support JDK distributions that provide platform-specific builds.

#### Foojay API lib_c_type Mapping
The Foojay API returns different `lib_c_type` values based on the platform:
- Linux with glibc: `"libc"` (or `"glibc"` for some distributions)
- Linux with musl/Alpine: `"musl"`
- macOS: `"libc"`
- Windows: `"c_std_lib"`

#### Supported Distributions with Alpine/musl variants:
- Eclipse Temurin (Alpine builds available)
- BellSoft Liberica (Alpine variants)
- Amazon Corretto (musl-compatible builds)
- Azul Zulu (Alpine support)

### Error Handling Strategy
Fail fast with clear, actionable error messages based on platform mismatch:

1. **Linux: Kopi built with musl, attempting to install glibc JDK**:
   ```
   Error: This JDK is built for glibc systems but kopi is using musl libc.
   Please install an Alpine-compatible JDK using:
   kopi install temurin@11-alpine
   ```

2. **Linux: Kopi built with glibc, attempting to install musl JDK**:
   ```
   Error: This JDK is built for Alpine Linux (musl libc) but kopi is using glibc.
   Please install a standard JDK using:
   kopi install temurin@11
   ```

3. **Cross-platform mismatch**:
   ```
   Error: This JDK is built for [target_platform] but kopi is running on [current_platform].
   Please install a JDK for your platform using:
   kopi install temurin@11
   ```

4. **Automatic selection message**:
   ```
   Info: Detected platform: [linux-musl/linux-glibc/macos/windows], selecting compatible JDK variant...
   ```

### Implementation Phases

**Phase 1**: Self-Detection Implementation (Required for MVP)
- Implement compile-time platform detection using `cfg` attributes
- Create `PLATFORM_LIBC` constant based on target environment
- Implement `get_foojay_libc_type()` to map platform to API values
- Ensure correct mapping: musl→"musl", glibc→"libc", macOS→"libc", Windows→"c_std_lib"

**Phase 2**: JDK Selection Logic
- Implement `get_foojay_libc_type()` to get API query value for current platform
- Use this value in Foojay API queries: `?lib_c_type={value}`
- Implement `matches_foojay_libc_type()` to validate downloaded JDKs
- Filter JDKs using the mapping: musl→"musl", glibc→"libc", macOS→"libc", Windows→"c_std_lib"
- Update install command to automatically select correct variant
- Add informational messages about automatic platform matching

**Phase 3**: Error Handling and Validation
- Add pre-download validation to check JDK libc compatibility
- Implement clear error messages for libc mismatches
- Add `kopi doctor` command to show kopi's libc type and validate installed JDKs
- Create migration guide for users switching between musl/glibc environments

**Phase 4**: Distribution and Testing
- Set up CI/CD to build both musl and glibc variants of kopi
- Create release artifacts with clear naming (e.g., `kopi-linux-x64-musl`, `kopi-linux-x64-glibc`)
- Add integration tests for both libc variants
- Document the self-matching behavior in user documentation

## Consequences

### Positive
- **Guaranteed compatibility**: JDKs will always match kopi's libc type
- **Simplified user experience**: No need for users to understand musl vs glibc
- **Automatic selection**: Kopi can automatically choose the correct variant
- **Prevents runtime failures**: Binary incompatibility issues are avoided entirely
- **Single source of truth**: Kopi's own binary determines the libc requirement

### Negative
- **Cross-compilation complexity**: Building kopi for different libc targets requires careful setup
- **Distribution limitations**: Users can't override libc selection if needed
- **Testing requirements**: Need to test both musl and glibc builds of kopi

### Neutral
- Requires clear documentation about the self-matching behavior
- Distribution packages must be careful about which kopi binary they ship
- May require providing both musl and glibc builds of kopi itself

## References
- musl libc functional differences: https://wiki.musl-libc.org/functional-differences-from-glibc.html
- Alpine Linux glibc compatibility: https://wiki.alpinelinux.org/wiki/Running_glibc_programs
- Gradle Alpine images documentation: https://github.com/docker-library/docs/tree/master/gradle