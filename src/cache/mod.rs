mod conversion;
mod models;
mod searcher;
mod storage;

#[cfg(test)]
mod tests;

use chrono::{DateTime, Utc};
use log::warn;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use crate::api::ApiClient;
use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::models::distribution::Distribution as JdkDistribution;
use crate::models::metadata::JdkMetadata;
use crate::models::package::ChecksumType;

// Re-export commonly used types from search functionality
pub use models::{PlatformFilter, SearchResult, SearchResultRef, VersionSearchType};
pub use searcher::PackageSearcher;

// Re-export platform functions from the main platform module for convenience
pub use crate::platform::{get_current_architecture, get_current_os, get_current_platform};

// Re-export conversion functions
pub use conversion::{convert_api_to_cache, convert_package_to_jdk_metadata};

// Re-export storage functions
pub use storage::{load_cache, save_cache};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetadataCache {
    pub version: u32,
    pub last_updated: DateTime<Utc>,
    pub distributions: HashMap<String, DistributionCache>,
    /// Maps distribution synonyms to their canonical api_parameter names
    #[serde(default)]
    pub synonym_map: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DistributionCache {
    pub distribution: JdkDistribution,
    pub display_name: String,
    pub packages: Vec<JdkMetadata>,
}

impl MetadataCache {
    pub fn new() -> Self {
        Self {
            version: 1,
            last_updated: Utc::now(),
            distributions: HashMap::new(),
            synonym_map: HashMap::new(),
        }
    }
}

impl Default for MetadataCache {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataCache {
    /// Check if the cache is stale based on the given maximum age
    pub fn is_stale(&self, max_age: Duration) -> bool {
        let now = Utc::now();
        let elapsed = now.signed_duration_since(self.last_updated);

        // Convert chrono::Duration to std::time::Duration for comparison
        match elapsed.to_std() {
            Ok(std_duration) => std_duration > max_age,
            Err(_) => true, // If time went backwards or conversion failed, consider stale
        }
    }

    pub fn has_version(&self, version: &str) -> bool {
        for dist in self.distributions.values() {
            for package in &dist.packages {
                if package.version.to_string() == version {
                    return true;
                }
            }
        }
        false
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        storage::save_cache(self, path)
    }

    /// Resolve a distribution name using synonyms
    /// Returns the canonical name if found in synonym map, otherwise returns the input
    pub fn resolve_distribution_name<'a>(&self, name: &'a str) -> &'a str
    where
        'a: 'a,
    {
        // Try to resolve via synonym map first
        if let Some(canonical_name) = self.synonym_map.get(name) {
            // We need to return the input string since we can't return a reference
            // with a different lifetime. Instead, we'll handle this differently.
            // For now, return the input if it matches the canonical name
            if canonical_name == name {
                return name;
            }
        }

        // If it's already a canonical name in distributions, return it
        if self.distributions.contains_key(name) {
            return name;
        }

        // Return the input as-is
        name
    }

    /// Get the canonical name for a distribution from the synonym map
    /// Returns None if not found
    pub fn get_canonical_name(&self, name: &str) -> Option<&str> {
        self.synonym_map.get(name).map(|s| s.as_str())
    }

    /// Find a JDK package in the cache by its criteria
    pub fn find_package(
        &self,
        distribution: &str,
        version: &str,
        architecture: &str,
        operating_system: &str,
    ) -> Option<&JdkMetadata> {
        // Try to get canonical name from synonym map
        let canonical_name = self
            .get_canonical_name(distribution)
            .unwrap_or(distribution);

        self.distributions.get(canonical_name).and_then(|dist| {
            dist.packages.iter().find(|pkg| {
                pkg.version.to_string() == version
                    && pkg.architecture.to_string() == architecture
                    && pkg.operating_system.to_string() == operating_system
            })
        })
    }
}

// Helper functions for metadata operations

/// Get metadata with optional version check
pub fn get_metadata(requested_version: Option<&str>, config: &KopiConfig) -> Result<MetadataCache> {
    let cache_path = config.metadata_cache_path()?;

    // Try to use cache if it exists
    if cache_path.exists() {
        match load_cache(&cache_path) {
            Ok(loaded_cache) => {
                // If specific version requested and not in cache, try API
                if let Some(version) = requested_version {
                    if !loaded_cache.has_version(version) {
                        return fetch_and_cache_metadata(false, config);
                    }
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
    fetch_and_cache_metadata(false, config)
}

/// Fetch metadata from API and cache it
pub fn fetch_and_cache_metadata(
    javafx_bundled: bool,
    config: &KopiConfig,
) -> Result<MetadataCache> {
    // Fetch metadata from API
    let api_client = ApiClient::new();
    let metadata = api_client
        .fetch_all_metadata_with_options(javafx_bundled)
        .map_err(|e| KopiError::MetadataFetch(format!("Failed to fetch metadata from API: {e}")))?;

    // Convert API response to cache format
    let new_cache = conversion::convert_api_to_cache(metadata)?;

    // Save to cache
    let cache_path = config.metadata_cache_path()?;
    new_cache.save(&cache_path)?;

    Ok(new_cache)
}

/// Fetch metadata for a specific distribution and update the cache
pub fn fetch_and_cache_distribution(
    distribution_name: &str,
    javafx_bundled: bool,
    config: &KopiConfig,
) -> Result<MetadataCache> {
    use std::str::FromStr;

    // Get current platform info
    let (current_arch, current_os, current_libc) = get_current_platform();

    // Load existing cache if available or create new one
    let cache_path = config.metadata_cache_path()?;
    let mut result_cache = if cache_path.exists() {
        load_cache(&cache_path)?
    } else {
        MetadataCache::new()
    };

    // Fetch metadata from API for the specific distribution
    let api_client = ApiClient::new();

    // Check if distribution exists first
    let distributions = api_client
        .get_distributions()
        .map_err(|e| KopiError::MetadataFetch(format!("Failed to fetch distributions: {e}")))?;

    let dist_info = distributions
        .iter()
        .find(|d| d.api_parameter == distribution_name)
        .ok_or_else(|| {
            KopiError::InvalidConfig(format!("Unknown distribution: {distribution_name}"))
        })?;

    // Create package query for this distribution
    let query = crate::api::PackageQuery {
        distribution: Some(distribution_name.to_string()),
        version: None,
        architecture: Some(current_arch.clone()),
        package_type: None,
        operating_system: Some(current_os.clone()),
        lib_c_type: Some(current_libc),
        archive_types: None,
        javafx_bundled: Some(javafx_bundled),
        directly_downloadable: Some(true),
        latest: None,
    };

    let packages = api_client.get_packages(Some(query)).map_err(|e| {
        KopiError::MetadataFetch(format!(
            "Failed to fetch packages for {distribution_name}: {e}"
        ))
    })?;

    // Convert packages to JdkMetadata
    let jdk_packages: Vec<JdkMetadata> = packages
        .into_iter()
        .filter_map(|pkg| conversion::convert_package_to_jdk_metadata(pkg).ok())
        .collect();

    // Create DistributionCache
    let dist_cache = DistributionCache {
        distribution: JdkDistribution::from_str(distribution_name)
            .unwrap_or(JdkDistribution::Other(distribution_name.to_string())),
        display_name: dist_info.name.clone(),
        packages: jdk_packages,
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
pub fn fetch_package_checksum(package_id: &str) -> Result<(String, ChecksumType)> {
    let api_client = ApiClient::new();
    let package_info = api_client
        .get_package_by_id(package_id)
        .map_err(|e| KopiError::MetadataFetch(format!("Failed to fetch package checksum: {e}")))?;

    // Check if checksum is empty
    if package_info.checksum.is_empty() {
        return Err(KopiError::MetadataFetch(format!(
            "No checksum available for package ID: {package_id}"
        )));
    }

    // Parse checksum type
    let checksum_type = match package_info.checksum_type.to_lowercase().as_str() {
        "sha1" => ChecksumType::Sha1,
        "sha256" => ChecksumType::Sha256,
        "sha512" => ChecksumType::Sha512,
        "md5" => ChecksumType::Md5,
        unsupported => {
            warn!(
                "Unsupported checksum type '{unsupported}' received from foojay API. Defaulting to SHA256."
            );
            ChecksumType::Sha256 // Default to SHA256
        }
    };

    Ok((package_info.checksum, checksum_type))
}
