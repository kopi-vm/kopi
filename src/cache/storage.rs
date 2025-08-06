// Copyright 2025 dentsusoken
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::cache::MetadataCache;
use crate::error::{KopiError, Result};
use crate::platform;
use std::fs;
use std::path::Path;

/// Load metadata cache from a file
pub fn load_cache(path: &Path) -> Result<MetadataCache> {
    let contents = fs::read_to_string(path)
        .map_err(|e| KopiError::ConfigError(format!("Failed to read cache file: {e}")))?;

    let cache: MetadataCache =
        serde_json::from_str(&contents).map_err(|_e| KopiError::InvalidMetadata)?;
    Ok(cache)
}

/// Save metadata cache to a file
pub fn save_cache(cache: &MetadataCache, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            KopiError::ConfigError(format!("Failed to create cache directory: {e}"))
        })?;
    }

    let json = serde_json::to_string_pretty(cache).map_err(|_e| KopiError::InvalidMetadata)?;

    // Write to temporary file first for atomic operation
    let temp_path = path.with_extension("tmp");

    // Clean up any leftover temp file from previous failed attempts
    if temp_path.exists() {
        fs::remove_file(&temp_path)
            .map_err(|e| KopiError::ConfigError(format!("Failed to remove old temp file: {e}")))?;
    }

    fs::write(&temp_path, json)
        .map_err(|e| KopiError::ConfigError(format!("Failed to write cache file: {e}")))?;

    // Use platform-specific atomic rename
    platform::file_ops::atomic_rename(&temp_path, path)
        .map_err(|e| KopiError::ConfigError(format!("Failed to rename cache file: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::DistributionCache;
    use crate::models::distribution::Distribution as JdkDistribution;
    use tempfile::TempDir;

    #[test]
    fn test_load_nonexistent_cache() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("cache.json");

        // load_cache should fail for non-existent files
        assert!(load_cache(&cache_path).is_err());
    }

    #[test]
    fn test_save_and_load_cache() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("cache.json");

        let mut cache = MetadataCache::new();
        let dist = DistributionCache {
            distribution: JdkDistribution::Temurin,
            display_name: "Eclipse Temurin".to_string(),
            packages: Vec::new(),
        };
        cache.distributions.insert("temurin".to_string(), dist);

        save_cache(&cache, &cache_path).unwrap();

        let loaded_cache = load_cache(&cache_path).unwrap();
        assert_eq!(loaded_cache.version, cache.version);
        assert_eq!(loaded_cache.distributions.len(), 1);
        assert!(loaded_cache.distributions.contains_key("temurin"));
    }
}
