# ADR-017: JDK Release Metadata Sources

## Status

Proposed

## Context

Kopi currently relies on the foojay.io Disco API for JDK metadata. To ensure resilience, accuracy, and completeness of metadata, we need to investigate alternative sources and understand the strengths and limitations of each metadata provider. This investigation covers direct vendor APIs, script-friendly URLs, and metadata collection methods.

## Decision

### Current Implementation: Foojay Disco API

#### API Characteristics

- **Base URL**: `https://api.foojay.io/disco/v3.0`
- **Coverage**: Comprehensive, covering 30+ JDK distributions
- **Standardized**: Consistent data format across all vendors
- **Real-time**: Live data with direct download links

#### Key Data Points Used

```rust
Package {
    id: String,                        // Unique package identifier
    distribution: String,              // Vendor name (lowercase)
    major_version: u32,                // Java major version
    java_version: String,              // Full version string
    archive_type: String,              // tar.gz, zip, etc.
    operating_system: String,          // linux, windows, macos
    architecture: String,              // x64, aarch64, etc.
    lib_c_type: Option<String>,        // glibc, musl
    package_type: String,              // jdk, jre
    term_of_support: Option<String>,   // lts, sts
    directly_downloadable: bool,
    links: {
        pkg_download_redirect: String,
        pkg_info_uri: Option<String>,
    },
    checksum: String,                  // Available via package info API
    checksum_type: String,             // sha256, sha1
}
```

### Alternative Metadata Sources

#### 1. Eclipse Adoptium API v3

**Characteristics**:

- **Direct API**: Official API from Eclipse Foundation
- **Coverage**: Eclipse Temurin builds only
- **Endpoint Example**: `https://api.adoptium.net/v3/assets/feature_releases/`

**Available Metadata**:

```json
{
  "release_name": "jdk-11.0.28+6",
  "vendor": "eclipse",
  "version": {
    "major": 11,
    "minor": 0,
    "security": 28,
    "build": 6,
    "openjdk_version": "11.0.28+6",
    "semver": "11.0.28+6"
  },
  "binaries": [
    {
      "os": "linux",
      "architecture": "x64",
      "image_type": "jdk",
      "package": {
        "checksum": "sha256:...",
        "checksum_link": "https://...",
        "download_count": 12345,
        "link": "https://...",
        "size": 195837568
      }
    }
  ]
}
```

**Comparison with Foojay**:

- ✅ More detailed version breakdown (security, build numbers)
- ✅ Direct checksum links
- ✅ Download statistics
- ❌ Limited to Temurin only
- ❌ No lib_c_type information

#### 2. Azul Metadata API v1

**Characteristics**:

- **Direct API**: Official Azul Systems API
- **Coverage**: Azul Zulu builds only
- **Endpoint Example**: `https://api.azul.com/metadata/v1/zulu/packages`

**Available Metadata**:

```json
{
  "availability_type": "CA",
  "distro_version": [21, 44, 17, 0],
  "download_url": "https://...",
  "java_version": [21, 0, 8],
  "latest": true,
  "name": "zulu21.44.17-ca-jdk21.0.8-linux_x64.rpm",
  "openjdk_build_number": 9,
  "package_uuid": "..."
}
```

**Comparison with Foojay**:

- ✅ Direct download URLs
- ✅ Detailed version arrays
- ❌ Limited to Zulu only
- ❌ No checksum information in main response
- ❌ Less OS/architecture metadata

#### 3. Oracle JDK (Script-Friendly URLs)

**Collection Methods**:

1. **Latest Versions API**: `https://java.oraclecloud.com/javaVersions`
   - Returns latest GA releases
   - Filters OTN-licensed versions

2. **Archive Pages**: `https://www.oracle.com/java/technologies/javase/jdk{version}-archive-downloads.html`
   - HTML scraping required
   - Historical versions available

3. **Checksum URLs**:
   - SHA256: Append `.sha256` to any download URL
   - Example: `https://download.oracle.com/java/24/latest/jdk-24_linux-x64_bin.tar.gz.sha256`

**Metadata Extraction Process**:

```bash
# From joschi/java-metadata
1. Fetch javaVersions JSON
2. For each version, get javaReleases/{version}
3. Parse download links and metadata
4. Fetch checksums by appending .sha256 to download URLs
```

**Comparison with Foojay**:

- ✅ Official Oracle source
- ✅ Includes archive versions
- ✅ Direct SHA256 checksum URLs
- ❌ No direct API for all metadata
- ❌ Requires HTML parsing for archives

#### 4. GraalVM (GitHub Releases)

**Collection Method**:

- **Source**: GitHub API for `graalvm/graalvm-ce-builds`
- **Pattern**: Release tags starting with "jdk"
- **Oracle GraalVM**: Available via Oracle Script-Friendly URLs
  - Example: `https://download.oracle.com/graalvm/24/latest/graalvm-jdk-24_linux-x64_bin.tar.gz`
  - Checksum: Append `.sha256` to download URL
- **Community Edition**: GitHub releases include `.sha256` files
  - Example: `graalvm-community-jdk-24.0.2_linux-x64_bin.tar.gz.sha256`

**Metadata Extraction Process**:

```bash
# From joschi/java-metadata
1. Fetch GitHub releases via API for Community Edition
2. Filter releases with "jdk" prefix
3. Parse filenames for version/OS/arch
4. For Oracle GraalVM, use script-friendly URLs with .sha256 for checksums
5. For Community Edition, download corresponding .sha256 files from release assets
```

**Comparison with Foojay**:

- ✅ Official source
- ✅ Includes release notes
- ✅ Both Oracle and Community editions provide SHA256 checksum files
- ❌ Requires GitHub API rate limits consideration for CE
- ❌ Complex filename parsing required

#### 5. Amazon Corretto (Permanent URLs)

**Collection Methods**:

1. **GitHub Releases**: `corretto/corretto-{version}` repositories
2. **Direct URLs**:
   - `https://corretto.aws/downloads/resources/{version}/{filename}`
   - `https://d3pxv6yz143wms.cloudfront.net/{version}/{filename}`
3. **Checksum URLs**:
   - MD5: `https://corretto.aws/downloads/latest_checksum/{package-name}`
   - SHA256: `https://corretto.aws/downloads/latest_sha256/{package-name}`

**Metadata Extraction Process**:

```bash
# From joschi/java-metadata
1. Enumerate known version patterns
2. Check multiple URL patterns for each OS/arch combination
3. Download and verify availability
4. Calculate checksums (Note: Corretto now provides direct checksum URLs)
```

**Comparison with Foojay**:

- ✅ Stable, permanent URLs
- ✅ CDN-backed downloads
- ✅ Direct checksum URLs (MD5 and SHA256)
- ❌ No comprehensive listing API
- ❌ Requires URL pattern knowledge

#### 6. Microsoft JDK

**Collection Method**:

- **Source**: Web scraping from `https://docs.microsoft.com/en-us/java/openjdk/download`
- **Pattern**: Regex matching `microsoft-jdk-*` links
- **Checksum URLs**: Append `.sha256sum.txt` to any download URL
  - Example: `https://aka.ms/download-jdk/microsoft-jdk-21.0.7-linux-x64.tar.gz.sha256sum.txt`

**Metadata Extraction Process**:

```bash
# From joschi/java-metadata
1. Download Microsoft documentation page
2. Extract download links via regex
3. Parse filename for metadata
4. Fetch checksums by appending .sha256sum.txt to download URLs
```

**Comparison with Foojay**:

- ✅ Official Microsoft source
- ✅ Direct SHA256 checksum URLs
- ❌ No API available
- ❌ Requires HTML parsing
- ❌ Subject to page structure changes

#### 7. joschi/java-metadata (Aggregated Metadata)

**Characteristics**:

- **Type**: Pre-collected metadata repository
- **Coverage**: 30+ JDK distributions
- **Update Frequency**: Regular automated updates
- **Access**: Static JSON files via GitHub Pages

**Available Metadata Structure**:

```json
{
  "vendor": "temurin",
  "filename": "OpenJDK21U-jdk_x64_linux_hotspot_21.0.1_12.tar.gz",
  "release_type": "ga", // ga (stable) or ea (early access)
  "version": "21.0.1+12",
  "java_version": "21",
  "jvm_impl": "hotspot", // hotspot, openj9, graalvm
  "os": "linux",
  "architecture": "x64",
  "file_type": "tar.gz",
  "image_type": "jdk", // jdk or jre
  "features": [], // vendor-specific features
  "url": "https://...",
  "md5": "...",
  "sha1": "...",
  "sha256": "...",
  "sha512": "...",
  "size": 195837568
}
```

**Access Methods**:

1. **All Metadata**: `https://joschi.github.io/java-metadata/metadata/all.json`
2. **Checksum Manifests**: `https://joschi.github.io/java-metadata/checksums/{vendor}/{filename}.{hash}`
3. **GitHub Repository**: Direct access to collection scripts

**Comparison with Foojay**:

- ✅ No API rate limits
- ✅ All checksums pre-calculated
- ✅ Consistent format across all vendors
- ✅ Includes vendors not in foojay
- ❌ Not real-time (periodic updates)
- ❌ No query/filtering capabilities
- ❌ Static data requires full download
- ❌ No version discovery API

### Metadata Completeness Comparison

| Field                | Foojay | Adoptium | Azul | Oracle  | GraalVM | Corretto | Microsoft | joschi/java-metadata |
| -------------------- | ------ | -------- | ---- | ------- | ------- | -------- | --------- | -------------------- |
| Distribution Name    | ✅     | ✅       | ✅   | ✅      | ✅      | ✅       | ✅        | ✅                   |
| Version String       | ✅     | ✅       | ✅   | ✅      | ✅      | ✅       | ✅        | ✅                   |
| OS/Platform          | ✅     | ✅       | ✅   | ✅      | ✅      | ✅       | ✅        | ✅                   |
| Architecture         | ✅     | ✅       | ✅   | ✅      | ✅      | ✅       | ✅        | ✅                   |
| Package Type         | ✅     | ✅       | ❌   | ✅      | ✅      | ✅       | ✅        | ✅                   |
| Archive Type         | ✅     | ✅       | ✅   | ✅      | ✅      | ✅       | ✅        | ✅                   |
| Direct Download URL  | ✅     | ✅       | ✅   | ✅      | ✅      | ✅       | ✅        | ✅                   |
| Checksum (All Types) | ✅     | ✅       | ❌   | ✅      | ✅      | ✅       | ✅        | ✅                   |
| LTS/Support Info     | ✅     | ✅       | ❌   | ✅      | ✅      | ✅       | ✅        | ❌                   |
| libc Type            | ✅     | ❌       | ❌   | ❌      | ❌      | ❌       | ❌        | ❌                   |
| Release Status       | ✅     | ✅       | ✅   | ✅      | ✅      | ✅       | ✅        | ✅                   |
| JVM Implementation   | ❌     | ❌       | ❌   | ❌      | ❌      | ❌       | ❌        | ✅                   |
| API Available        | ✅     | ✅       | ✅   | Partial | ❌      | ❌       | ❌        | Static               |
| Real-time Updates    | ✅     | ✅       | ✅   | ✅      | ✅      | ✅       | ✅        | ❌                   |
| Query/Filter Support | ✅     | ✅       | ✅   | ❌      | ❌      | ❌       | ❌        | ❌                   |

## Rationale

### Advantages of Current Foojay Implementation

1. **Unified Interface**: Single API for all distributions
2. **Comprehensive Metadata**: Most complete field coverage
3. **Real-time Updates**: Live data without caching delays
4. **Standardized Format**: Consistent across all vendors
5. **Query Flexibility**: Rich filtering capabilities

### Potential Benefits of Direct Vendor APIs

1. **Authoritative Source**: Direct from vendor
2. **Additional Metadata**: Vendor-specific fields (e.g., download counts)
3. **Reduced Dependency**: Not reliant on third-party aggregator
4. **Faster Updates**: No aggregation delay

### Challenges with Direct Vendor Approach

1. **Inconsistent APIs**: Each vendor has different formats
2. **Missing APIs**: Some vendors (Oracle, Microsoft) lack proper APIs
3. **Maintenance Burden**: Multiple integrations to maintain
4. **Feature Gaps**: Some metadata only available via foojay
5. **Rate Limiting**: GitHub API limits for GraalVM/Corretto

## Consequences

### Positive

- Understanding of alternative metadata sources provides fallback options
- Awareness of vendor-specific metadata availability
- Knowledge of direct vendor URLs enables bypass of aggregators when needed
- Script-based collection methods provide blueprint for custom implementations

### Negative

- Direct vendor integration would significantly increase code complexity
- Maintaining multiple API clients increases maintenance burden
- Some vendors require web scraping, which is fragile
- Loss of unified query interface across distributions

## Recommendations

1. **Primary Strategy**: Continue using foojay.io as the primary metadata source
   - Most comprehensive and unified solution
   - Well-maintained and regularly updated
   - Provides consistent interface across all vendors

2. **Fallback Implementation**: Consider implementing vendor-specific fallbacks for critical distributions:
   - Eclipse Adoptium API for Temurin (has proper API)
   - Azul Metadata API for Zulu (has proper API)
   - GitHub releases for GraalVM and Corretto

3. **Static Metadata Option**: Consider joschi/java-metadata as an emergency fallback:
   - Pre-calculated checksums for all distributions
   - No API rate limits or availability concerns
   - Useful for offline scenarios or when APIs are unavailable
   - Trade-off: Not real-time, requires periodic sync

4. **Metadata Caching**: Enhance local caching to handle foojay.io outages:
   - Store complete metadata snapshots
   - Implement incremental updates
   - Add checksum verification for cached data
   - Consider periodic sync with joschi/java-metadata for offline resilience

5. **Future Considerations**:
   - Monitor vendor API developments
   - Consider contributing to foojay.io for missing features
   - Implement health checks for metadata sources

## References

- Foojay Disco API Documentation: https://api.foojay.io/swagger-ui/
- Eclipse Adoptium API: https://api.adoptium.net/
- Azul Metadata API: https://api.azul.com/metadata/v1/docs/
- joschi/java-metadata: https://github.com/joschi/java-metadata
- joschi/java-metadata All Metadata: https://joschi.github.io/java-metadata/metadata/all.json
- Oracle JDK Downloads: https://www.oracle.com/java/technologies/downloads/
- Oracle Script-Friendly URLs: https://www.oracle.com/java/technologies/jdk-script-friendly-urls/
- GraalVM Releases: https://github.com/graalvm/graalvm-ce-builds/releases
- Corretto Downloads: https://docs.aws.amazon.com/corretto/
- Microsoft JDK: https://docs.microsoft.com/en-us/java/openjdk/download
- Microsoft JDK Script-Friendly URLs: https://learn.microsoft.com/en-us/java/openjdk/download-major-urls
