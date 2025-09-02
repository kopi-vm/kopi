# Foojay Metadata Source

## Overview

The FoojayMetadataSource wraps the existing `ApiClient` to implement the `MetadataSource` trait. This source is primarily used for:
1. Maintaining backward compatibility during migration
2. Generating metadata files via kopi-metadata-gen tool
3. Development and testing purposes

Note: This source is NOT part of the production fallback chain. Production uses HTTP → Local Directory fallback.

## Implementation

```rust
pub struct FoojayMetadataSource {
    client: ApiClient,
    config: FoojayConfig,
}

impl FoojayMetadataSource {
    pub fn new(config: FoojayConfig) -> Self {
        Self {
            client: ApiClient::new(),
            config,
        }
    }
}

impl MetadataSource for FoojayMetadataSource {
    fn id(&self) -> &str {
        "foojay"
    }
    
    fn name(&self) -> &str {
        "Foojay Discovery API"
    }
    
    fn is_available(&self) -> Result<bool> {
        // Try a simple API call to check availability
        match self.client.ping() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    fn fetch_all(&self) -> Result<Vec<JdkMetadata>> {
        // Use existing ApiClient methods
        let packages = self.client.get_packages(None)?;
        
        // Convert to JdkMetadata with is_complete=false
        packages.into_iter()
            .map(|pkg| {
                let mut metadata = convert_package_to_metadata(pkg)?;
                metadata.is_complete = false; // Missing download_url and checksum
                Ok(metadata)
            })
            .collect()
    }
    
    fn fetch_distribution(&self, distribution: &str) -> Result<Vec<JdkMetadata>> {
        let query = PackageQuery {
            distribution: Some(distribution.to_string()),
            ..Default::default()
        };
        
        let packages = self.client.get_packages(Some(query))?;
        
        packages.into_iter()
            .map(|pkg| {
                let mut metadata = convert_package_to_metadata(pkg)?;
                metadata.is_complete = false;
                Ok(metadata)
            })
            .collect()
    }
    
    fn fetch_package_details(&self, package_id: &str) -> Result<PackageDetails> {
        // Fetch complete package info from API
        let package = self.client.get_package_by_id(package_id)?;
        
        Ok(PackageDetails {
            download_url: package.direct_download_uri,
            checksum: package.checksum,
            checksum_type: package.checksum_type.map(parse_checksum_type),
        })
    }
    
    fn last_updated(&self) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
        // Foojay API doesn't provide last update time
        Ok(None)
    }
}
```

## API Characteristics

### Archive Type Filtering

The Foojay API provides packages in various archive formats. Kopi filters API requests to only retrieve supported formats.

#### Investigation Results (2025-07-28)

**1. Tar Format Availability**

Query: `https://api.foojay.io/disco/v3.0/packages?archive_type=tar`  
Result: **0 packages found**

The Foojay API does not provide any packages in uncompressed tar format across all platforms.

**2. Linux x64 Archive Type Distribution**

```
Archive Type | Count  | Percentage
-------------|--------|------------
tar.gz       | 5,044  | 52%
rpm          | 1,886  | 19%
deb          | 1,834  | 19%
apk          | 383    | 4%
zip          | 309    | 3%
bin          | 152    | 2%
tar.xz       | 75     | 1%
-------------|--------|------------
Total        | 9,683  | 100%
```

**3. Kopi Supported Formats Coverage**

Kopi supports the following archive types:
- tar.gz (maps from both "tar.gz" and "tgz")
- zip

Coverage analysis:
- Total packages: 9,683
- Supported formats (tar.gz + zip): 5,353
- Coverage: 55%

The remaining 45% consists primarily of package manager formats (rpm, deb, apk) which are not intended for direct JDK installation via Kopi.

**4. Archive Type Usage by Platform**

Windows:
- zip: 5,442 (primary format)
- msi: 4,230
- tar.gz: 530
- exe: 495

macOS:
- tar.gz: 4,721 (primary format)
- pkg: 2,907
- dmg: 2,719
- zip: 2,620

**5. Code Consistency**

Previously, the code had an inconsistency where FoojayMetadataSource included "tar" in archive_types filter, but:
- The ArchiveType enum did not have a Tar variant
- Archive extraction only supported TarGz and Zip
- No packages exist with archive_type="tar"

This has been resolved by removing "tar" from the archive_types filter in FoojayMetadataSource.

**Future Considerations:**
- tar.xz format is used exclusively by RedHat distribution (75 packages)
- Could be added to expand coverage for enterprise users

### Two-Phase Loading

The Foojay API requires two separate calls:

1. **List Packages** (`/packages`)
   - Returns: Basic metadata (id, version, architecture, etc.)
   - Missing: `download_url`, `checksum`, `checksum_type`
   - Fast: Single API call for many packages

2. **Get Package by ID** (`/packages/{id}`)
   - Returns: Complete package details
   - Includes: Download URL and checksum
   - Slow: One API call per package

### Rate Limiting

The Foojay API has rate limits:
- Requests per minute: ~60
- Burst capacity: Limited
- No authentication required

### Error Handling

Common errors:
- Network timeouts
- Rate limit exceeded (429)
- Server errors (5xx)
- Invalid package ID (404)

## Migration Strategy

1. Keep existing `ApiClient` unchanged
2. Create `FoojayMetadataSource` as a wrapper
3. Update code to use `MetadataProvider` instead of direct `ApiClient`
4. Remove direct `ApiClient` usage once migration is complete

## Configuration

```toml
[metadata.sources.foojay]
enabled = false  # Not enabled by default
base_url = "https://api.foojay.io/disco"
timeout_secs = 30
retry_attempts = 3

# Enable only for:
# - Development/testing
# - Running kopi-metadata-gen tool
# - Debugging metadata issues
```