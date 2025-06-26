use chrono::{DateTime, Utc};
use log::warn;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::api::{ApiClient, ApiMetadata};
use crate::error::{KopiError, Result};
use crate::models::jdk::{ChecksumType, Distribution as JdkDistribution, JdkMetadata};
use dirs::home_dir;

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

impl Default for MetadataCache {
    fn default() -> Self {
        Self {
            version: 1,
            last_updated: Utc::now(),
            distributions: HashMap::new(),
        }
    }
}

impl MetadataCache {
    pub fn new() -> Self {
        Self::default()
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
}

pub fn get_cache_path() -> Result<PathBuf> {
    let kopi_home = get_kopi_home()?;
    let cache_dir = kopi_home.join("cache");
    fs::create_dir_all(&cache_dir)
        .map_err(|e| KopiError::ConfigError(format!("Failed to create cache directory: {}", e)))?;
    Ok(cache_dir.join("metadata.json"))
}

fn get_kopi_home() -> Result<PathBuf> {
    // Check KOPI_HOME environment variable first
    if let Ok(kopi_home) = std::env::var("KOPI_HOME") {
        let path = PathBuf::from(kopi_home);
        if path.is_absolute() {
            return Ok(path);
        }
    }

    // Fall back to ~/.kopi
    home_dir()
        .map(|home| home.join(".kopi"))
        .ok_or_else(|| KopiError::ConfigError("Unable to determine home directory".to_string()))
}

pub fn load_cache(path: &Path) -> Result<MetadataCache> {
    if !path.exists() {
        return Ok(MetadataCache::new());
    }

    let contents = fs::read_to_string(path)
        .map_err(|e| KopiError::ConfigError(format!("Failed to read cache file: {}", e)))?;

    serde_json::from_str(&contents).map_err(|_e| KopiError::InvalidMetadata)
}

pub fn save_cache(path: &Path, cache: &MetadataCache) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            KopiError::ConfigError(format!("Failed to create cache directory: {}", e))
        })?;
    }

    let json = serde_json::to_string_pretty(cache).map_err(|_e| KopiError::InvalidMetadata)?;

    // Write to temporary file first for atomic operation
    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, json)
        .map_err(|e| KopiError::ConfigError(format!("Failed to write cache file: {}", e)))?;

    // Atomic rename
    fs::rename(temp_path, path)
        .map_err(|e| KopiError::ConfigError(format!("Failed to rename cache file: {}", e)))?;

    Ok(())
}

pub fn get_metadata(requested_version: Option<&str>) -> Result<MetadataCache> {
    let cache_path = get_cache_path()?;

    // Try to use cache if it exists
    if cache_path.exists() {
        match load_cache(&cache_path) {
            Ok(cache) => {
                // If specific version requested and not in cache, try API
                if let Some(version) = requested_version {
                    if !cache.has_version(version) {
                        return fetch_and_cache_metadata();
                    }
                }
                return Ok(cache);
            }
            Err(e) => {
                // Cache load failed, log warning and fall back to API
                warn!("Failed to load cache: {}. Falling back to API.", e);
            }
        }
    }

    // No cache or cache load failed, fetch from API
    fetch_and_cache_metadata()
}

pub fn fetch_and_cache_metadata() -> Result<MetadataCache> {
    // Fetch metadata from API
    let api_client = ApiClient::new();
    let metadata = api_client.fetch_all_metadata().map_err(|e| {
        KopiError::MetadataFetch(format!("Failed to fetch metadata from API: {}", e))
    })?;

    // Convert API response to cache format
    let cache = convert_api_to_cache(metadata)?;

    // Save to cache
    let cache_path = get_cache_path()?;
    save_cache(&cache_path, &cache)?;

    Ok(cache)
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

/// Find a JDK package in the cache by its criteria
pub fn find_package_in_cache<'a>(
    cache: &'a MetadataCache,
    distribution: &str,
    version: &str,
    architecture: &str,
    operating_system: &str,
) -> Option<&'a JdkMetadata> {
    cache.distributions.get(distribution).and_then(|dist| {
        dist.packages.iter().find(|pkg| {
            pkg.version.to_string() == version
                && pkg.architecture.to_string() == architecture
                && pkg.operating_system.to_string() == operating_system
        })
    })
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

fn convert_api_to_cache(api_metadata: ApiMetadata) -> Result<MetadataCache> {
    use crate::models::jdk::{
        Architecture, ArchiveType, ChecksumType, OperatingSystem, PackageType, Version,
    };
    use std::str::FromStr;

    let mut cache = MetadataCache::new();

    // Convert API format to cache format
    for dist_metadata in api_metadata.distributions {
        let dist_info = dist_metadata.distribution;

        // Parse distribution
        let distribution = JdkDistribution::from_str(&dist_info.api_parameter)
            .unwrap_or(JdkDistribution::Other(dist_info.api_parameter.clone()));

        let mut packages = Vec::new();

        // Convert API packages to JdkMetadata
        for api_package in dist_metadata.packages {
            // Parse version
            let version = Version::from_str(&api_package.java_version)
                .unwrap_or_else(|_| Version::new(api_package.major_version, 0, 0));

            // Parse architecture from filename
            let architecture = parse_architecture_from_filename(&api_package.filename)
                .unwrap_or(Architecture::X64);

            // Parse operating system
            let operating_system = OperatingSystem::from_str(&api_package.operating_system)
                .unwrap_or(OperatingSystem::Linux);

            // Parse archive type
            let archive_type =
                ArchiveType::from_str(&api_package.archive_type).unwrap_or(ArchiveType::TarGz);

            let jdk_metadata = JdkMetadata {
                id: api_package.id,
                distribution: api_package.distribution,
                version,
                distribution_version: api_package.distribution_version,
                architecture,
                operating_system,
                package_type: PackageType::Jdk, // Default to JDK
                archive_type,
                download_url: api_package.links.pkg_download_redirect,
                checksum: None, // TODO: Fetch from API if available
                checksum_type: Some(ChecksumType::Sha256),
                size: api_package.size,
                lib_c_type: api_package.lib_c_type,
            };

            packages.push(jdk_metadata);
        }

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

        let cache = load_cache(&cache_path).unwrap();
        assert_eq!(cache.version, 1);
        assert!(cache.distributions.is_empty());
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

        save_cache(&cache_path, &cache).unwrap();

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
    fn test_find_package_in_cache() {
        use crate::models::jdk::{
            Architecture, ArchiveType, OperatingSystem, PackageType, Version,
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
        };

        let dist = DistributionCache {
            distribution: JdkDistribution::Temurin,
            display_name: "Eclipse Temurin".to_string(),
            packages: vec![jdk_metadata],
        };
        cache.distributions.insert("temurin".to_string(), dist);

        // Should find the package
        let found = find_package_in_cache(&cache, "temurin", "21.0.1", "x64", "linux");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "test-id");

        // Should not find with wrong version
        let not_found = find_package_in_cache(&cache, "temurin", "17.0.1", "x64", "linux");
        assert!(not_found.is_none());
    }
}
