mod conversion;
mod metadata_cache;
mod models;
mod searcher;
mod storage;

#[cfg(test)]
mod tests;

use chrono::Utc;
use log::warn;
use std::str::FromStr;

use crate::api::ApiClient;
use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::models::distribution::Distribution as JdkDistribution;
use crate::models::metadata::JdkMetadata;
use crate::models::package::ChecksumType;

// Re-export commonly used types from search functionality
pub use models::{PlatformFilter, SearchResult, SearchResultRef, VersionSearchType};
pub use searcher::PackageSearcher;

// Re-export metadata cache types
pub use metadata_cache::{DistributionCache, MetadataCache};

// Re-export platform functions from the main platform module for convenience
pub use crate::platform::{get_current_architecture, get_current_os, get_current_platform};

// Re-export conversion functions
pub use conversion::{convert_api_to_cache, convert_package_to_jdk_metadata};

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
