# Cache Search Feature Design

## Current Implementation Analysis

### Overview
The `kopi cache search` command searches and displays packages from locally cached JDK metadata that match specified criteria.

### Current Display Columns
| Column | Content | Evaluation |
|--------|---------|------------|
| ► | Auto-selection marker | × Should be removed |
| Version | Version number | ○ Required |
| LibC | glibc/musl type (Linux) | ○ Important |
| Type | JDK/JRE | ○ Required |
| Size | Download size (MB) | ○ Useful |
| Archive | tar.gz/zip format | × Unnecessary |
| JavaFX | JavaFX bundled | △ Conditional display |

#### Why the Auto-selection Marker (►) Should Be Removed
The `install` and `cache search` commands serve different purposes:

- **`install` version string**: Written in `.java-version` files to specify exactly one JDK
- **`cache search` query**: Flexible criteria for exploratory searching

Considering future support for project-specific `.java-version` files, having an auto-selection marker in search results:
1. Deviates from the search's primary purpose (discovering options)
2. Duplicates the role of `install --dry-run`
3. May confuse users

Therefore, the auto-selection marker should be removed, leaving the "which will be selected" confirmation to `install --dry-run`.

### Current Search Query Format
```bash
# Version only (required)
kopi cache search 21

# Distribution@version
kopi cache search temurin@21

# Package type specification
kopi cache search jre@21
kopi cache search jdk@corretto@21
```

## Issues and Improvement Proposals

### 1. Display Column Issues

#### Problems
- **Archive column**: Information that doesn't affect user selection
- **Missing platform information**: Showing only LibC without OS/Arch is unbalanced
- **Missing LTS information**: Difficult to identify long-term support versions

#### Improvements
- Remove auto-selection marker (►)
- Remove Archive column
- Add OS/Arch information (detailed view only)
- Add LTS display column (using foojay API's `term_of_support` field)
- Add Status column (detailed view only, using foojay API's `release_status` field)

### 2. Search Functionality Limitations

#### Problems
- **Version requirement** is too restrictive
- Cannot search by distribution alone
- Cannot search for latest versions

#### Improvements
```bash
# Search by distribution only
kopi cache search corretto

# Search for latest versions
kopi cache search latest
kopi cache search --latest

# Show LTS only
kopi cache search --lts-only 21
```

### 3. Output Format Rigidity

#### Problems
- Fixed table format only
- Difficult to use programmatically

#### Improvements
```bash
# Compact display (default)
kopi cache search 21

# Detailed display
kopi cache search 21 --detailed

# JSON output (for processing with jq, etc.)
kopi cache search 21 --json
```


## Technical Considerations

### Platform-Specific Caching
Kopi only caches metadata for the current execution environment. This means:
- Cache contains only packages compatible with current OS
- Cache contains only packages for current CPU architecture
- Cache contains only packages for current LibC type (Linux)

This approach:
- Reduces cache size significantly
- Simplifies search operations (no platform filtering needed)
- Ensures all cached packages are installable on current system

Users can verify the detected platform using `kopi doctor`, which displays the current execution environment among other diagnostic information.

### Leveraging foojay API
The API response contains the following unused fields:
- `term_of_support`: LTS/STS identification (for display)
- `release_status`: GA/EA identification (for display)
- `latest_build_available`: Latest build flag (internal filtering only)

Adding these to the `Package` model enables richer search and filtering capabilities.

### Final Display Column Recommendations

| Display Mode | Columns |
|--------------|---------|
| Compact | Distribution, Version, LTS |
| Detailed | Above + Status(GA/EA), Type(JDK/JRE), OS/Arch, LibC, Size, JavaFX |
| JSON | All fields (for programmatic processing) |

### Performance and UX
- Default to compact display for quick overview
- Use `--detailed` option only when needed
- Delegate complex processing to external tools (jq, etc.) via JSON output
- Advanced searches like version range queries achieved through JSON output + jq combination

## Summary

These improvements provide the following value:

1. **Intuitive Search**: Relaxed version requirements enable more natural searching
2. **Appropriate Information Display**: Shows only necessary and sufficient information for decision-making
3. **Flexible Output**: Choose display format based on use case
4. **Programmable**: JSON output enables easy automation and scripting

These improvements deliver a more efficient and user-friendly interface for the fundamental user need of finding JDKs.

## Enhanced Version Search Capabilities

### Version Field Selection

The foojay.io API provides two distinct version fields for each JDK package:
- **`java_version`**: Standard OpenJDK format (e.g., `21.0.7+6`)
- **`distribution_version`**: Distribution-specific format (e.g., `21.0.7.6.1` for Corretto)

### Automatic Version Type Detection

Kopi can automatically detect which version field to search based on the format:

```bash
# Auto-detects as java_version (has build number +6)
kopi cache search temurin@21.0.7+6

# Auto-detects as distribution_version (4+ components)
kopi cache search corretto@21.0.7.6.1

# Auto-detects as distribution_version (6 components)
kopi cache search dragonwell@21.0.7.0.7.6
```

#### Detection Rules
1. **Contains `+` (build number)** → Search by `java_version`
2. **4 or more version components** → Search by `distribution_version`
3. **3 or fewer components** → Search both fields (fallback)

### Manual Version Field Override

For cases where automatic detection might be ambiguous:

```bash
# Force search by java_version
kopi cache search corretto@21.0.7 --java-version

# Force search by distribution_version
kopi cache search corretto@21.0.7 --distribution-version
```

### Use Cases

#### Finding Specific Corretto Patch Versions
```bash
# Search for exact Corretto patch version
kopi cache search corretto@21.0.7.6.1

# Install the specific version found
kopi install corretto@21.0.7.6.1
```

#### Handling Multiple Versions with Same java_version
When multiple distribution versions share the same java_version:
```bash
# Shows all Corretto packages with java_version 21.0.7
kopi cache search corretto@21.0.7 --java-version

# Shows specific distribution version
kopi cache search corretto@21.0.7.6.1
```

### Distribution Examples

| Distribution | Example java_version | Example distribution_version |
|--------------|---------------------|----------------------------|
| Temurin | 21.0.7+6 | 21.0.7 |
| Corretto | 21.0.7+6 | 21.0.7.6.1 |
| Dragonwell | 21.0.7 | 21.0.7.0.7.6 |
| JetBrains | 21.0.7+895130 | 21.0.7 |
| GraalVM CE | 21.3.3.1 | 21.3.3.1 |

### Implementation Notes

1. **Backward Compatibility**: All existing search queries continue to work
2. **Performance**: Version type detection adds minimal overhead
3. **Error Handling**: Clear messages when no matches found in either field
4. **Display**: Both version fields shown in detailed output mode