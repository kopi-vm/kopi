# Install Command Design

## Overview

This document describes the design and implementation details of the `kopi install` command, including API integration challenges and solutions.

## Foojay API Integration

### Latest Parameter Behavior

The Foojay API provides a `latest` parameter to control which versions are returned. Through testing and source code analysis, we've identified the following behaviors:

#### `latest=per_version`

- **Purpose**: Returns the latest build for a specific major version
- **Implementation**: Filters packages using `isLatestBuildAvailable()` flag
- **Requirements**: `version` parameter is mandatory
- **Known Issues**: Does not work correctly with version 17

Example results:

```
version=17 + latest=per_version → 0 packages ❌
version=21 + latest=per_version → 21.0.8+9 ✅
```

#### `latest=available`

- **Purpose**: Returns the latest available version(s)
- **Implementation**: Uses version comparison to find the maximum version
- **Requirements**: `version` parameter is optional
- **Behavior**:
  - Without version: Returns latest version for each major version
  - With version: Returns latest version for specified major version

Example results:

```
version=17 + latest=available → 17.0.16+8 ✅
version=21 + latest=available → 21.0.8+9 ✅
```

### Implementation Decision

Based on the analysis, we use `latest=available` instead of `latest=per_version` because:

1. **Consistency**: Works correctly for all versions, including version 17
2. **Reliability**: Does not depend on metadata flags that may be incorrectly set
3. **Flexibility**: Allows queries without version parameter

### Fallback Strategy

To ensure robustness, the implementation includes a fallback mechanism:

```rust
// Primary query with latest=available
let mut packages = self.api_client.get_packages(Some(query))?;

// If no packages found, try without latest parameter
if packages.is_empty() {
    let fallback_query = /* query without latest */;
    packages = self.api_client.get_packages(Some(fallback_query))?;

    // Sort packages by version (descending) to get latest first
    packages.sort_by(|a, b| {
        match (Version::from_str(&a.java_version),
               Version::from_str(&b.java_version)) {
            (Ok(v1), Ok(v2)) => v2.cmp(&v1),
            _ => b.java_version.cmp(&a.java_version),
        }
    });
}
```

### Root Cause Analysis

From the Foojay API source code analysis:

1. **`per_version` filtering**:

   ```java
   .filter(pkg -> pkg.isLatestBuildAvailable())
   ```

   This relies on a metadata flag that appears to be incorrectly set for version 17 packages.

2. **`available` filtering**:
   Uses direct version comparison to find the maximum version, which is more reliable than depending on metadata flags.

### Test Results Summary

| Query                            | Result           |
| -------------------------------- | ---------------- |
| `version=17, latest=per_version` | 0 packages (bug) |
| `version=17, latest=available`   | 17.0.16+8        |
| `version=17, no latest`          | 17+35 (oldest)   |
| `version=21, latest=per_version` | 21.0.8+9         |
| `version=21, latest=available`   | 21.0.8+9         |

### Recommendations

1. Always use `latest=available` for consistent behavior
2. Implement fallback logic for robustness
3. Consider reporting the version 17 issue to the Foojay API maintainers
4. Monitor API behavior changes in future updates

## Version Selection

When a user specifies a simple major version (e.g., `kopi install 17`), the system should:

1. Query for all available versions of that major version
2. Select the latest stable release
3. Avoid selecting outdated versions like `17+35` when newer versions like `17.0.16+8` are available

This ensures users always get the most up-to-date and secure version of their requested JDK.

## Current Implementation

### Package Discovery Flow

The current implementation follows a two-path approach:

1. **Cache Path** (if cache exists):

   ```rust
   // Check if package exists in cache
   let cache_path = self.config.metadata_cache_path()?;
   if cache_path.exists() {
       let cache = crate::cache::load_cache(&cache_path)?;
       let searcher = PackageSearcher::new(&cache, self.config);
       if let Some(jdk_metadata) = searcher.find_exact_package(...) {
           return Ok(self.convert_metadata_to_package(&jdk_metadata));
       }
   }
   ```

2. **Direct API Path** (if not in cache):
   ```rust
   // Query API directly with specific parameters
   let query = PackageQuery {
       version: Some(version.to_string()),
       distribution: Some(distribution.id().to_string()),
       latest: Some("available".to_string()),
       // ... other parameters
   };
   let packages = self.api_client.get_packages(Some(query))?;
   ```

### Current Implementation Issues

1. **Cache Freshness**: No automatic refresh mechanism
2. **Duplicate Logic**: Different code paths for cache vs API queries
3. **No Cache Updates**: Downloaded package info not saved to cache
4. **Network Inefficiency**: Repeated API calls for same information

## Proposed Cache Auto-Refresh Design

### Overview

Implement automatic cache refresh to ensure users always have access to the latest JDK versions while maintaining good performance.

### Cache Staleness Detection

```rust
pub struct CacheMetadata {
    pub last_updated: SystemTime,
    pub cache_version: String,
}

impl MetadataCache {
    pub fn is_stale(&self, max_age: Duration) -> bool {
        match self.metadata.last_updated.elapsed() {
            Ok(elapsed) => elapsed > max_age,
            Err(_) => true, // If time went backwards, consider stale
        }
    }
}
```

### Auto-Refresh Configuration

Add to `config.toml`:

```toml
[cache]
# Maximum age before cache is considered stale
max_age_hours = 720  # 30 days

# Whether to auto-refresh stale cache
auto_refresh = true

# Refresh cache if requested package not found
refresh_on_miss = true
```

### Unified Search Path

All package searches will go through the cache:

```rust
impl InstallCommand {
    fn find_package(&self, version_spec: &str) -> Result<Package> {
        // Always ensure fresh cache
        let cache = self.ensure_fresh_cache()?;

        // Always search through cache
        let searcher = PackageSearcher::new(&cache, self.config);
        let package = searcher.find_package(version_spec)?
            .ok_or_else(|| KopiError::VersionNotAvailable(...))?;

        Ok(package)
    }

    fn ensure_fresh_cache(&self) -> Result<MetadataCache> {
        let cache_path = self.config.metadata_cache_path()?;
        let max_age = Duration::from_secs(self.config.cache.max_age_hours * 3600);

        // Load existing cache if available
        let should_refresh = if cache_path.exists() {
            match cache::load_cache(&cache_path) {
                Ok(cache) => cache.is_stale(max_age),
                Err(_) => true,
            }
        } else {
            true
        };

        // Refresh if needed
        if should_refresh && self.config.cache.auto_refresh {
            info!("Refreshing package cache...");
            let metadata = self.api_client.fetch_all_metadata()?;
            let cache = MetadataCache::from_api_metadata(metadata)?;
            cache::save_cache(&cache_path, &cache)?;
            Ok(cache)
        } else {
            cache::load_cache(&cache_path)
        }
    }
}
```

### Benefits

1. **Consistency**: Single code path for all searches
2. **Freshness**: Automatic updates ensure latest versions available
3. **Performance**: Cache prevents unnecessary API calls
4. **Reliability**: Fallback to existing cache if refresh fails
5. **User Control**: Configurable auto-refresh behavior

### Migration Path

1. Implement cache staleness checking
2. Add auto-refresh configuration options
3. Refactor install command to use unified search
4. Remove direct API query code
5. Add telemetry to monitor cache hit/miss rates

### Error Handling

- If cache refresh fails, use existing cache with warning
- If no cache exists and refresh fails, return clear error
- Provide manual refresh option: `kopi cache refresh`

### Future Enhancements

1. **Partial Updates**: Refresh only specific distributions
2. **Background Refresh**: Update cache in background process
3. **Smart Refresh**: Refresh based on release patterns (e.g., more frequent during release windows)
4. **Offline Mode**: Force use of existing cache without refresh
