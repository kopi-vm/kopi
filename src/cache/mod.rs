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

mod conversion;
mod metadata_cache;
mod models;
mod storage;

#[cfg(test)]
mod tests;

use chrono::Utc;
use log::warn;
use std::str::FromStr;

use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::metadata::provider::MetadataProvider;
use crate::models::distribution::Distribution as JdkDistribution;
use crate::models::metadata::JdkMetadata;
use crate::models::package::ChecksumType;

// Re-export commonly used types from search functionality
pub use models::{PlatformFilter, SearchResult, VersionSearchType};

// Re-export metadata cache types
pub use metadata_cache::{DistributionCache, MetadataCache};

// Re-export platform functions from the main platform module for convenience
pub use crate::platform::{get_current_architecture, get_current_os, get_current_platform};

// Re-export conversion functions
pub use conversion::{
    convert_api_to_cache, convert_package_to_jdk_metadata, parse_architecture_from_filename,
};

// Re-export storage functions
pub use storage::{load_cache, save_cache};

// Helper functions for metadata operations

/// Get metadata with optional version check
pub fn get_metadata(requested_version: Option<&str>, config: &KopiConfig) -> Result<MetadataCache> {
    let cache_path = config.metadata_cache_path()?;

    // Try to use cache if it exists
    if cache_path.exists() {
        match load_cache(&cache_path) {
            Ok(loaded_cache) => {
                // If specific version requested and not in cache, try API
                if let Some(version) = requested_version
                    && !loaded_cache.has_version(version)
                {
                    return fetch_and_cache_metadata(config);
                }
                return Ok(loaded_cache);
            }
            Err(e) => {
                // Cache load failed, log warning and fall back to API
                warn!("Failed to load cache: {e}. Falling back to API.");
            }
        }
    }

    // No cache or cache load failed, fetch from API
    fetch_and_cache_metadata(config)
}

/// Fetch metadata from API and cache it
pub fn fetch_and_cache_metadata(config: &KopiConfig) -> Result<MetadataCache> {
    // Create metadata provider from config
    let provider = MetadataProvider::from_config(config)?;

    // Fetch all metadata (includes both JavaFX and non-JavaFX packages)
    let metadata = provider
        .fetch_all()
        .map_err(|e| KopiError::MetadataFetch(format!("Failed to fetch metadata from API: {e}")))?;

    // Convert metadata to cache format
    let mut new_cache = MetadataCache::new();

    // Group metadata by distribution
    let mut distributions: std::collections::HashMap<String, Vec<JdkMetadata>> =
        std::collections::HashMap::new();
    for jdk in metadata {
        distributions
            .entry(jdk.distribution.clone())
            .or_default()
            .push(jdk);
    }

    // Create distribution caches
    for (dist_name, packages) in distributions {
        let dist_cache = DistributionCache {
            distribution: JdkDistribution::from_str(&dist_name)
                .unwrap_or(JdkDistribution::Other(dist_name.clone())),
            display_name: dist_name.clone(), // For now, use dist name as display name
            packages,
        };
        new_cache.distributions.insert(dist_name, dist_cache);
    }

    new_cache.last_updated = Utc::now();

    // Save to cache
    let cache_path = config.metadata_cache_path()?;
    new_cache.save(&cache_path)?;

    Ok(new_cache)
}

/// Fetch metadata for a specific distribution and update the cache
pub fn fetch_and_cache_distribution(
    distribution_name: &str,
    config: &KopiConfig,
) -> Result<MetadataCache> {
    // Load existing cache if available or create new one
    let cache_path = config.metadata_cache_path()?;
    let mut result_cache = if cache_path.exists() {
        load_cache(&cache_path)?
    } else {
        MetadataCache::new()
    };

    // Create metadata provider from config
    let provider = MetadataProvider::from_config(config)?;

    // Fetch metadata for the specific distribution (includes both JavaFX and non-JavaFX)
    let packages = provider
        .fetch_distribution(distribution_name)
        .map_err(|e| {
            KopiError::MetadataFetch(format!(
                "Failed to fetch packages for {distribution_name}: {e}"
            ))
        })?;

    // Create DistributionCache
    let dist_cache = DistributionCache {
        distribution: JdkDistribution::from_str(distribution_name)
            .unwrap_or(JdkDistribution::Other(distribution_name.to_string())),
        display_name: distribution_name.to_string(), // For now, use dist name as display name
        packages,
    };

    // Update cache with this distribution
    result_cache
        .distributions
        .insert(distribution_name.to_string(), dist_cache);
    result_cache.last_updated = Utc::now();

    // Save updated cache
    result_cache.save(&cache_path)?;

    Ok(result_cache)
}

/// Fetch checksum for a specific JDK package
pub fn fetch_package_checksum(
    package_id: &str,
    config: &KopiConfig,
) -> Result<(String, ChecksumType)> {
    // First, try to find the metadata in the cache
    let cache = get_metadata(None, config)?;

    // Search for the package in all distributions
    let mut found_metadata = None;
    for dist_cache in cache.distributions.values() {
        if let Some(metadata) = dist_cache.packages.iter().find(|pkg| pkg.id == package_id) {
            found_metadata = Some(metadata.clone());
            break;
        }
    }

    // If not found in cache, we can't fetch checksum without full metadata
    let mut metadata = found_metadata.ok_or_else(|| {
        KopiError::MetadataFetch(format!(
            "Package with ID '{package_id}' not found in cache. Cannot fetch checksum."
        ))
    })?;

    // If metadata is incomplete, try to complete it
    if !metadata.is_complete() {
        // Create metadata provider from config
        let provider = MetadataProvider::from_config(config)?;

        // Fetch the complete details
        provider.ensure_complete(&mut metadata).map_err(|e| {
            KopiError::MetadataFetch(format!("Failed to fetch package checksum: {e}"))
        })?;
    }

    // Extract checksum and type
    let checksum = metadata.checksum.ok_or_else(|| {
        KopiError::MetadataFetch(format!(
            "No checksum available for package ID: {package_id}"
        ))
    })?;

    let checksum_type = metadata.checksum_type.unwrap_or_else(|| {
        warn!("No checksum type available for package ID: {package_id}. Defaulting to SHA256.");
        ChecksumType::Sha256
    });

    Ok((checksum, checksum_type))
}
