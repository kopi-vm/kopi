# Corretto Version Format Investigation (2025-07-11)

This document details the investigation into Amazon Corretto's 4-component version format and its implications for the Kopi version parser. This issue was discovered while fixing Windows test failures.

## Summary

Amazon Corretto uses a 4-component version format (e.g., `21.0.5.11.1`) that differs from the standard 3-component OpenJDK format (e.g., `21.0.5+11`). The current Kopi version parser only supports up to 3 components, causing parsing failures and test issues.

## Discovery Context

The issue was discovered while investigating Windows test failures in `tests/uninstall_integration.rs`:
- Test `test_uninstall_with_version_pattern` was failing
- The test creates a Corretto JDK with version `21.0.5.11.1`
- When attempting to uninstall with pattern `21`, the Corretto version was not being matched

## Version Format Comparison

### Standard OpenJDK Format
```
<major>.<minor>.<patch>+<build>
Examples:
- 21.0.5+11 (Temurin)
- 17.0.9+9 (AdoptOpenJDK)
- 21.0.5+11 (Azul Zulu)
```

### Amazon Corretto Format
```
<major>.<minor>.<patch>.<corretto-specific>
Examples:
- 21.0.5.11.1
- 17.0.13.11.1
- 11.0.25.9.1
- 8.432.06.1
```

### Component Breakdown

For example `21.0.5.11.1`:
1. **Major (21)**: Java major version
2. **Minor (0)**: Minor version (typically 0)
3. **Patch (5)**: Security/bug fix patch number
4. **Corretto-specific (11.1)**: Amazon's build identifier
   - First number (11): Corresponds to the OpenJDK build number
   - Second number (1): Corretto-specific revision

## Current Implementation Limitation

### Version Parser (`src/models/version.rs:127`)
```rust
impl FromStr for Version {
    fn from_str(s: &str) -> Result<Self> {
        // ...
        let components: Vec<&str> = version_part.split('.').collect();
        if components.is_empty() || components.len() > 3 {
            return Err(KopiError::InvalidVersionFormat(s.to_string()));
        }
        // ...
    }
}
```

The parser explicitly rejects versions with more than 3 components.

## Impact on Functionality

### 1. Version Parsing Failures
- `Version::from_str("21.0.5.11.1")` returns an error
- Corretto JDKs cannot be properly parsed from directory names

### 2. Version Matching Issues
- Pattern matching fails because Corretto versions cannot be parsed
- Commands like `kopi uninstall 21` don't match Corretto installations
- The `resolve_jdks_by_version` function cannot match Corretto JDKs

### 3. Test Failures
- `test_uninstall_with_version_pattern` expects an error for multiple matches
- Instead, it succeeds because Corretto version fails to parse and is excluded

## Other Distributions with Special Formats

### IBM Semeru
Sometimes uses 4 components: `21.0.1.0`

### GraalVM
Uses additional suffix: `21.0.1+12.1`

### Azul Zulu
Can use alternative numbering: `21.30.19` (their own build system)

## Corretto Usage in Codebase

Found references to Corretto 4-component versions:
- `tests/uninstall_integration.rs`: `21.0.5.11.1`
- `docs/adr/014-configuration-and-version-file-formats.md`: `17.0.5.8.1`
- `tests/shim_security.rs`: `11.0.21.9.1`
- `docs/tasks/uninstall/design.md`: Multiple examples
- `src/cache/mod.rs`: Test with `amazon-corretto-21.0.1.12.1-linux-x86_64.tar.gz`

## foojay.io API Investigation

### Additional Version Format Discoveries

Investigation of the foojay.io API revealed more complex version formats than initially documented:

#### Dragonwell (Alibaba)
- Uses **6 components**: `21.0.7.0.7.6`
- Example filenames:
  - `Alibaba_Dragonwell_Extended_21.0.7.0.7.6_x64_linux.tar.gz`
  - `Alibaba_Dragonwell_Standard_21.0.7.0.7.6_x64_linux.tar.gz`

#### Corretto (Java 8)
- Omits leading zeros: `8.452.9.1` (instead of `8.0.452.9.1`)
- Example filenames:
  - `amazon-corretto-8.452.09.1-linux-x64.tar.gz`

#### JetBrains Runtime
- Uses extremely large build numbers: `21.0.7+895130`
- Example filename:
  - `jbrsdk_jcef-21.0.7-linux-x64-b895.130.tar.gz`

#### Semeru (IBM)
- java_version and distribution_version: `21.0.7` (standard format)
- Note: Filename contains underscore (`21.0.7_6`) but actual version fields do not

### Version Format Summary from API Data

| Distribution | java_version | distribution_version | Notes |
|-------------|--------------|---------------------|--------|
| Corretto | 21.0.7+6 | 21.0.7.6.1 | 5 components |
| Dragonwell | 21.0.7 | 21.0.7.0.7.6 | 6 components |
| JetBrains | 21.0.7+895130 | 21.0.7 | Very large build number |
| Semeru | 21.0.7 | 21.0.7 | Standard format |
| GraalVM CE | 21.3.3.1 | 21.3.3.1 | 4 components |

### Implications for Version Parser Design

Based on actual API version fields (not filenames), the proposed unified version format with separators (`. + -`) needs to handle:

1. **6+ components**: Dragonwell's `21.0.7.0.7.6` exceeds typical expectations
2. **Inconsistent leading zeros**: Corretto Java 8 uses `8.452.9.1` instead of `8.0.452.9.1`
3. **Large build numbers**: JetBrains uses `+895130`

### Revised Solution Approach

A flexible version structure that handles actual API version formats:

```rust
pub struct Version {
    pub components: Vec<u32>,  // All numeric components
    pub pre_release: Option<String>,  // -rc.1, etc.
}

// Separators: . + (hyphen starts pre-release)
```

This requires careful handling of:
- Version comparison logic for varying component counts
- Normalization of formats (e.g., adding missing zeros)
- Distribution-specific parsing rules

## Kopi Cache Search Implementation Analysis

### How Kopi Searches Versions

Investigation of Kopi's codebase reveals that version searching uses only the `java_version` field from the foojay.io API:

```rust
// src/cache/mod.rs:370
version: Version::from_str(&api_package.java_version)
    .unwrap_or_else(|_| Version::new(0, None, None)),
```

The `distribution_version` field is stored but **not used for searching**:
- It's saved for display purposes
- It's used for directory naming (`~/.kopi/jdks/corretto-21.0.7.6.1/`)
- It cannot be specified by users in search queries

### Impact on Corretto Users

For Corretto distributions where `java_version` and `distribution_version` differ:
- **Corretto Java 21**: 
  - `java_version`: `21.0.7+6` (used for searching)
  - `distribution_version`: `21.0.7.6.1` (not searchable)
- **Corretto Java 8**:
  - `java_version`: `8.0.452+9` (used for searching)
  - `distribution_version`: `8.452.9.1` (not searchable)

### Current Limitations for Users

1. **Cannot specify exact Corretto patch versions**: 
   - User cannot request `corretto@21.0.7.6.1` specifically
   - Can only request `corretto@21.0.7` which matches by `java_version`

2. **Multiple Corretto versions with same java_version**:
   - If Corretto releases `21.0.7.6.1` and `21.0.7.6.2` with same `java_version`
   - Users cannot distinguish between them in searches

3. **Workarounds**:
   - Use `kopi cache search corretto@21.0.7 --detailed --json` to see `distribution_version`
   - Check installed versions in `~/.kopi/jdks/` directory
   - Manual download from foojay.io for specific versions

### Example User Scenario

A user wanting Corretto `21.0.7.6.1` specifically:
```bash
# This doesn't work - cannot specify distribution_version
kopi install corretto@21.0.7.6.1  # Error: Invalid version format

# Current approach - might get any 21.0.7 variant
kopi install corretto@21.0.7

# Must verify manually
ls ~/.kopi/jdks/ | grep corretto-21
```

## Recommended Solution: Unified Version Structure

### Version Structure Redesign

Replace the current fixed-component structure with a flexible design that handles N components:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub components: Vec<u32>,           // All numeric components
    pub build: Option<Vec<u32>>,        // Build numbers as numeric array
    pub pre_release: Option<String>,    // Pre-release string (rarely used)
}
```

### Parser Implementation

```rust
impl FromStr for Version {
    fn from_str(s: &str) -> Result<Self> {
        // Handle pre-release
        let (version_part, pre_release) = if let Some(dash_pos) = s.find('-') {
            (&s[..dash_pos], Some(s[dash_pos + 1..].to_string()))
        } else {
            (s, None)
        };

        // Handle build number
        let (version_part, build) = if let Some(plus_pos) = version_part.find('+') {
            let build_str = &version_part[plus_pos + 1..];
            let build_components: Vec<u32> = build_str
                .split('.')
                .filter_map(|s| s.parse().ok())
                .collect();
            
            (&version_part[..plus_pos], 
             if build_components.is_empty() { None } else { Some(build_components) })
        } else {
            (version_part, None)
        };

        // Parse main components (no limit on count)
        let components: Vec<u32> = version_part
            .split('.')
            .map(|s| s.parse::<u32>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| KopiError::InvalidVersionFormat(s.to_string()))?;

        if components.is_empty() {
            return Err(KopiError::InvalidVersionFormat(s.to_string()));
        }

        Ok(Version { components, build, pre_release })
    }
}
```

### Version Matching

```rust
impl Version {
    pub fn matches_pattern(&self, pattern: &str) -> bool {
        let pattern_version = match Version::from_str(pattern) {
            Ok(v) => v,
            Err(_) => return false,
        };

        // Compare numeric components
        for (i, &pattern_comp) in pattern_version.components.iter().enumerate() {
            match self.components.get(i) {
                Some(&self_comp) if self_comp == pattern_comp => continue,
                _ => return false,
            }
        }

        // Compare build if specified in pattern
        if let Some(ref pattern_build) = pattern_version.build {
            match &self.build {
                None => return false,
                Some(self_build) => {
                    for (i, &pattern_comp) in pattern_build.iter().enumerate() {
                        match self_build.get(i) {
                            Some(&self_comp) if self_comp == pattern_comp => continue,
                            _ => return false,
                        }
                    }
                }
            }
        }

        true
    }

    // Helper methods for backward compatibility
    pub fn major(&self) -> u32 {
        self.components.get(0).copied().unwrap_or(0)
    }
    
    pub fn minor(&self) -> Option<u32> {
        self.components.get(1).copied()
    }
    
    pub fn patch(&self) -> Option<u32> {
        self.components.get(2).copied()
    }
}
```

### Version Search Enhancement

To support Corretto users who need specific patch versions:

```rust
// Automatic version type detection
fn detect_version_field(version: &str) -> VersionField {
    if version.contains('+') {
        VersionField::JavaVersion      // Has build number
    } else {
        let component_count = version.split('.').count();
        if component_count >= 4 {
            VersionField::DistributionVersion  // 4+ components
        } else {
            VersionField::Both            // Try both fields
        }
    }
}

// Search implementation
pub fn search_packages(version: &str, flags: SearchFlags) -> Vec<Package> {
    let field = match flags.version_field {
        Some(field) => field,
        None => detect_version_field(version),
    };
    
    match field {
        VersionField::JavaVersion => search_by_java_version(version),
        VersionField::DistributionVersion => search_by_distribution_version(version),
        VersionField::Both => {
            let mut results = search_by_java_version(version);
            if results.is_empty() {
                results = search_by_distribution_version(version);
            }
            results
        }
    }
}
```

### Examples

This design handles all discovered version formats:

```rust
// Corretto 5-component
"21.0.7.6.1" → components: [21, 0, 7, 6, 1]

// Dragonwell 6-component  
"21.0.7.0.7.6" → components: [21, 0, 7, 0, 7, 6]

// Standard with build
"21.0.7+6" → components: [21, 0, 7], build: Some([6])

// GraalVM with complex build
"21.0.1+12.1" → components: [21, 0, 1], build: Some([12, 1])

// Corretto Java 8 (no leading zero)
"8.452.9.1" → components: [8, 452, 9, 1]
```

### Benefits

1. **Handles any number of components**: Supports Corretto (5), Dragonwell (6), and future formats
2. **Correct numeric comparison**: Build numbers like 8 vs 10 compare correctly
3. **Backward compatible**: Helper methods maintain existing API
4. **User-friendly**: Automatic detection of version format
5. **Flexible search**: Users can search by either java_version or distribution_version

## Conclusion

The 4-component Corretto version format is a legitimate use case that the current parser cannot handle. This affects not just Corretto but potentially other distributions that use extended version formats. The issue should be addressed to ensure Kopi can manage all JDK distributions effectively.

The quickest fix would be Option 1 (ignore extra components), which would resolve the immediate test failures while maintaining backward compatibility. However, Option 2 (full support) would be more future-proof and accurate for version comparisons.