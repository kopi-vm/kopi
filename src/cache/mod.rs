use chrono::{DateTime, Utc};
use log::warn;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::api::{ApiClient, ApiMetadata};
use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::models::jdk::{ChecksumType, Distribution as JdkDistribution, JdkMetadata};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetadataCache {
    pub version: u32,
    pub last_updated: DateTime<Utc>,
    pub distributions: HashMap<String, DistributionCache>,
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
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                KopiError::ConfigError(format!("Failed to create cache directory: {}", e))
            })?;
        }

        let json = serde_json::to_string_pretty(self).map_err(|_e| KopiError::InvalidMetadata)?;

        // Write to temporary file first for atomic operation
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, json)
            .map_err(|e| KopiError::ConfigError(format!("Failed to write cache file: {}", e)))?;

        // Atomic rename
        fs::rename(temp_path, path)
            .map_err(|e| KopiError::ConfigError(format!("Failed to rename cache file: {}", e)))?;

        Ok(())
    }

    /// Find a JDK package in the cache by its criteria
    pub fn find_package(
        &self,
        distribution: &str,
        version: &str,
        architecture: &str,
        operating_system: &str,
    ) -> Option<&JdkMetadata> {
        self.distributions.get(distribution).and_then(|dist| {
            dist.packages.iter().find(|pkg| {
                pkg.version.to_string() == version
                    && pkg.architecture.to_string() == architecture
                    && pkg.operating_system.to_string() == operating_system
            })
        })
    }
}

// Standalone helper functions for backward compatibility

pub fn load_cache(path: &Path) -> Result<MetadataCache> {
    let contents = fs::read_to_string(path)
        .map_err(|e| KopiError::ConfigError(format!("Failed to read cache file: {}", e)))?;

    let cache: MetadataCache =
        serde_json::from_str(&contents).map_err(|_e| KopiError::InvalidMetadata)?;
    Ok(cache)
}


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
                        return fetch_and_cache_metadata(config);
                    }
                }
                return Ok(loaded_cache);
            }
            Err(e) => {
                // Cache load failed, log warning and fall back to API
                warn!("Failed to load cache: {}. Falling back to API.", e);
            }
        }
    }

    // No cache or cache load failed, fetch from API
    fetch_and_cache_metadata(config)
}

/// Fetch metadata from API and cache it
pub fn fetch_and_cache_metadata(config: &KopiConfig) -> Result<MetadataCache> {
    fetch_and_cache_metadata_with_options(false, config)
}

/// Fetch metadata from API with options and cache it
pub fn fetch_and_cache_metadata_with_options(
    javafx_bundled: bool,
    config: &KopiConfig,
) -> Result<MetadataCache> {
    // Fetch metadata from API
    let api_client = ApiClient::new();
    let metadata = api_client
        .fetch_all_metadata_with_options(javafx_bundled)
        .map_err(|e| {
            KopiError::MetadataFetch(format!("Failed to fetch metadata from API: {}", e))
        })?;

    // Convert API response to cache format
    let new_cache = convert_api_to_cache(metadata)?;

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
    use crate::search::get_current_platform;
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
        .map_err(|e| KopiError::MetadataFetch(format!("Failed to fetch distributions: {}", e)))?;

    let dist_info = distributions
        .iter()
        .find(|d| d.api_parameter == distribution_name)
        .ok_or_else(|| {
            KopiError::InvalidConfig(format!("Unknown distribution: {}", distribution_name))
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
            "Failed to fetch packages for {}: {}",
            distribution_name, e
        ))
    })?;

    // Convert packages to JdkMetadata
    let jdk_packages: Vec<JdkMetadata> = packages
        .into_iter()
        .filter_map(|pkg| convert_package_to_jdk_metadata(pkg).ok())
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
    let package_info = api_client.get_package_by_id(package_id).map_err(|e| {
        KopiError::MetadataFetch(format!("Failed to fetch package checksum: {}", e))
    })?;

    // Parse checksum type
    let checksum_type = match package_info.checksum_type.to_lowercase().as_str() {
        "sha256" => ChecksumType::Sha256,
        "sha512" => ChecksumType::Sha512,
        "md5" => ChecksumType::Md5,
        _ => ChecksumType::Sha256, // Default to SHA256
    };

    Ok((package_info.checksum, checksum_type))
}

fn parse_architecture_from_filename(filename: &str) -> Option<crate::models::jdk::Architecture> {
    use crate::models::jdk::Architecture;
    use std::str::FromStr;

    // Common architecture patterns in filenames
    let patterns = [
        ("x64", "x64"),
        ("x86_64", "x64"),
        ("amd64", "x64"),
        ("aarch64", "aarch64"),
        ("arm64", "aarch64"),
        ("x86", "x86"),
        ("i386", "x86"),
        ("i686", "x86"),
        ("arm32", "arm32"),
        ("ppc64le", "ppc64le"),
        ("ppc64", "ppc64"),
        ("s390x", "s390x"),
        ("sparcv9", "sparcv9"),
    ];

    for (pattern, arch_str) in patterns.iter() {
        if filename.contains(pattern) {
            return Architecture::from_str(arch_str).ok();
        }
    }

    None
}

fn convert_package_to_jdk_metadata(api_package: crate::api::Package) -> Result<JdkMetadata> {
    use crate::models::jdk::{
        Architecture, ArchiveType, ChecksumType, OperatingSystem, PackageType, Version,
    };
    use std::str::FromStr;

    // Parse version
    let version = Version::from_str(&api_package.java_version)
        .unwrap_or_else(|_| Version::new(api_package.major_version, 0, 0));

    // Parse architecture from filename
    let architecture =
        parse_architecture_from_filename(&api_package.filename).unwrap_or(Architecture::X64);

    // Parse operating system
    let operating_system =
        OperatingSystem::from_str(&api_package.operating_system).unwrap_or(OperatingSystem::Linux);

    // Parse archive type
    let archive_type =
        ArchiveType::from_str(&api_package.archive_type).unwrap_or(ArchiveType::TarGz);

    let package_type = PackageType::from_str(&api_package.package_type).unwrap_or(PackageType::Jdk);

    let jdk_metadata = JdkMetadata {
        id: api_package.id,
        distribution: api_package.distribution,
        version,
        distribution_version: api_package.distribution_version,
        architecture,
        operating_system,
        package_type,
        archive_type,
        download_url: api_package.links.pkg_download_redirect,
        checksum: None, // TODO: Fetch from API if available
        checksum_type: Some(ChecksumType::Sha256),
        size: api_package.size,
        lib_c_type: api_package.lib_c_type,
        javafx_bundled: api_package.javafx_bundled,
        term_of_support: api_package.term_of_support,
        release_status: api_package.release_status,
        latest_build_available: api_package.latest_build_available,
    };

    Ok(jdk_metadata)
}

fn convert_api_to_cache(api_metadata: ApiMetadata) -> Result<MetadataCache> {
    use std::str::FromStr;

    let mut cache = MetadataCache::new();

    // Convert API format to cache format
    for dist_metadata in api_metadata.distributions {
        let dist_info = dist_metadata.distribution;

        // Parse distribution
        let distribution = JdkDistribution::from_str(&dist_info.api_parameter)
            .unwrap_or(JdkDistribution::Other(dist_info.api_parameter.clone()));

        // Convert API packages to JdkMetadata
        let packages: Vec<JdkMetadata> = dist_metadata
            .packages
            .into_iter()
            .filter_map(|pkg| convert_package_to_jdk_metadata(pkg).ok())
            .collect();

        let dist_cache = DistributionCache {
            distribution,
            display_name: dist_info.name,
            packages,
        };

        cache
            .distributions
            .insert(dist_info.api_parameter, dist_cache);
    }

    Ok(cache)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_new_cache() {
        let cache = MetadataCache::new();
        assert_eq!(cache.version, 1);
        assert!(cache.distributions.is_empty());
    }

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

        cache.save(&cache_path).unwrap();

        let loaded_cache = load_cache(&cache_path).unwrap();
        assert_eq!(loaded_cache.version, cache.version);
        assert_eq!(loaded_cache.distributions.len(), 1);
        assert!(loaded_cache.distributions.contains_key("temurin"));
    }

    #[test]
    fn test_has_version() {
        use crate::models::jdk::{
            Architecture, ArchiveType, ChecksumType, OperatingSystem, PackageType, Version,
        };

        let mut cache = MetadataCache::new();

        let jdk_metadata = JdkMetadata {
            id: "test-id".to_string(),
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            distribution_version: "21.0.1+12".to_string(),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            archive_type: ArchiveType::TarGz,
            download_url: "https://example.com/download".to_string(),
            checksum: None,
            checksum_type: Some(ChecksumType::Sha256),
            size: 100000000,
            lib_c_type: None,
            javafx_bundled: false,
            term_of_support: None,
            release_status: None,
            latest_build_available: None,
        };

        let dist = DistributionCache {
            distribution: JdkDistribution::Temurin,
            display_name: "Eclipse Temurin".to_string(),
            packages: vec![jdk_metadata],
        };
        cache.distributions.insert("temurin".to_string(), dist);

        assert!(cache.has_version("21.0.1"));
        assert!(!cache.has_version("17.0.1"));
    }

    #[test]
    fn test_parse_architecture_from_filename() {
        use crate::models::jdk::Architecture;
        assert_eq!(
            parse_architecture_from_filename("OpenJDK21U-jdk_x64_linux_hotspot_21.0.1_12.tar.gz"),
            Some(Architecture::X64)
        );
        assert_eq!(
            parse_architecture_from_filename(
                "OpenJDK21U-jdk_aarch64_linux_hotspot_21.0.1_12.tar.gz"
            ),
            Some(Architecture::Aarch64)
        );
        assert_eq!(
            parse_architecture_from_filename("amazon-corretto-21.0.1.12.1-linux-x86_64.tar.gz"),
            Some(Architecture::X64)
        );
        assert_eq!(
            parse_architecture_from_filename("some_file_without_arch.tar.gz"),
            None
        );
    }

    #[test]
    fn test_find_package() {
        use crate::models::jdk::{
            Architecture, ArchiveType, ChecksumType, OperatingSystem, PackageType, Version,
        };

        let mut cache = MetadataCache::new();

        let jdk_metadata = JdkMetadata {
            id: "test-id".to_string(),
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            distribution_version: "21.0.1+12".to_string(),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            archive_type: ArchiveType::TarGz,
            download_url: "https://example.com/download".to_string(),
            checksum: None,
            checksum_type: Some(ChecksumType::Sha256),
            size: 100000000,
            lib_c_type: None,
            javafx_bundled: false,
            term_of_support: None,
            release_status: None,
            latest_build_available: None,
        };

        let dist = DistributionCache {
            distribution: JdkDistribution::Temurin,
            display_name: "Eclipse Temurin".to_string(),
            packages: vec![jdk_metadata],
        };
        cache.distributions.insert("temurin".to_string(), dist);

        // Should find the package
        let found = cache.find_package("temurin", "21.0.1", "x64", "linux");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "test-id");

        // Should not find with wrong version
        let not_found = cache.find_package("temurin", "17.0.1", "x64", "linux");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_convert_package_to_jdk_metadata() {
        use crate::api::{Links, Package};

        let api_package = Package {
            id: "test123".to_string(),
            distribution: "temurin".to_string(),
            major_version: 21,
            java_version: "21.0.1".to_string(),
            distribution_version: "21.0.1+12".to_string(),
            jdk_version: 21,
            operating_system: "linux".to_string(),
            package_type: "jdk".to_string(),
            archive_type: "tar.gz".to_string(),
            filename: "OpenJDK21U-jdk_x64_linux_hotspot_21.0.1_12.tar.gz".to_string(),
            directly_downloadable: true,
            links: Links {
                pkg_download_redirect: "https://example.com/download".to_string(),
                pkg_info_uri: None,
            },
            free_use_in_production: true,
            tck_tested: "yes".to_string(),
            size: 195000000,
            lib_c_type: Some("glibc".to_string()),
            javafx_bundled: false,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        };

        let result = convert_package_to_jdk_metadata(api_package);
        assert!(result.is_ok());

        let jdk_metadata = result.unwrap();
        assert_eq!(jdk_metadata.id, "test123");
        assert_eq!(jdk_metadata.distribution, "temurin");
        assert_eq!(jdk_metadata.version.major, 21);
        // Architecture is parsed from filename
        assert_eq!(jdk_metadata.architecture.to_string(), "x64");
    }
}
