use crate::cache::{MetadataCache, get_cache_path, load_cache};
use crate::error::Result;
use crate::models::jdk::{Distribution, JdkMetadata};
use crate::platform::{get_foojay_libc_type, matches_foojay_libc_type};
use crate::version::parser::{ParsedVersionRequest, VersionParser};

pub struct PackageSearcher<'a> {
    cache: Option<&'a MetadataCache>,
    platform_filter: PlatformFilter,
}

#[derive(Debug, Clone, Default)]
pub struct PlatformFilter {
    pub architecture: Option<String>,
    pub operating_system: Option<String>,
    pub lib_c_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub distribution: String,
    pub display_name: String,
    pub package: JdkMetadata,
}

impl<'a> PackageSearcher<'a> {
    pub fn new(cache: Option<&'a MetadataCache>) -> Self {
        Self {
            cache,
            platform_filter: PlatformFilter::default(),
        }
    }

    pub fn with_platform_filter(mut self, filter: PlatformFilter) -> Self {
        self.platform_filter = filter;
        self
    }

    /// Search for packages matching the version string
    pub fn search(&self, version_string: &str) -> Result<Vec<SearchResult>> {
        let parsed_request = VersionParser::parse(version_string)?;
        self.search_parsed(&parsed_request)
    }

    /// Search for packages matching the parsed version request
    pub fn search_parsed(&self, request: &ParsedVersionRequest) -> Result<Vec<SearchResult>> {
        let cache = match self.cache {
            Some(cache) => cache,
            None => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        for (dist_name, dist_cache) in &cache.distributions {
            // Filter by distribution if specified
            if let Some(ref target_dist) = request.distribution {
                if dist_cache.distribution != *target_dist {
                    continue;
                }
            }

            // Search for matching versions
            for package in &dist_cache.packages {
                if !self.matches_package(package, request) {
                    continue;
                }

                results.push(SearchResult {
                    distribution: dist_name.clone(),
                    display_name: dist_cache.display_name.clone(),
                    package: package.clone(),
                });
            }
        }

        // Sort by distribution and version
        results.sort_by(|a, b| match a.distribution.cmp(&b.distribution) {
            std::cmp::Ordering::Equal => b.package.version.cmp(&a.package.version),
            other => other,
        });

        Ok(results)
    }

    /// Find exact package match for installation
    pub fn find_exact_package(
        &self,
        distribution: &Distribution,
        version: &str,
        architecture: &str,
        operating_system: &str,
    ) -> Option<JdkMetadata> {
        let cache = self.cache?;

        // Look up distribution by its API name
        let dist_cache = cache.distributions.get(distribution.id())?;

        // Find exact match
        dist_cache
            .packages
            .iter()
            .find(|pkg| {
                pkg.version.to_string() == version
                    && pkg.architecture.to_string() == architecture
                    && pkg.operating_system.to_string() == operating_system
            })
            .cloned()
    }

    /// Determine which package would be auto-selected by install command
    pub fn find_auto_selected_package(
        &self,
        distribution: &Distribution,
        version: &str,
        architecture: &str,
        operating_system: &str,
        requested_package_type: Option<crate::models::jdk::PackageType>,
    ) -> Option<JdkMetadata> {
        let cache = self.cache?;
        let _lib_c_type = get_foojay_libc_type();

        // Look up distribution by its API name
        let dist_cache = cache.distributions.get(distribution.id())?;

        // Find packages matching version, arch, and OS
        let matching_packages: Vec<&JdkMetadata> = dist_cache
            .packages
            .iter()
            .filter(|pkg| {
                pkg.version.to_string() == version
                    && pkg.architecture.to_string() == architecture
                    && pkg.operating_system.to_string() == operating_system
            })
            .collect();

        // If only one match, return it
        if matching_packages.len() == 1 {
            return matching_packages.first().cloned().cloned();
        }

        // Multiple matches - apply the same logic as install command
        let packages_to_search = if let Some(requested_type) = requested_package_type {
            // If package type was explicitly requested, filter to that type
            let filtered: Vec<&JdkMetadata> = matching_packages
                .iter()
                .filter(|pkg| pkg.package_type == requested_type)
                .cloned()
                .collect();

            if !filtered.is_empty() {
                filtered
            } else {
                // No packages of requested type, fall back to all packages
                matching_packages
            }
        } else {
            // No specific package type requested, prefer JDK over JRE
            let jdk_packages: Vec<&JdkMetadata> = matching_packages
                .iter()
                .filter(|pkg| pkg.package_type == crate::models::jdk::PackageType::Jdk)
                .cloned()
                .collect();

            if !jdk_packages.is_empty() {
                jdk_packages
            } else {
                matching_packages
            }
        };

        // Then try to find one with matching lib_c_type
        if let Some(pkg) = packages_to_search.iter().find(|pkg| {
            if let Some(ref pkg_lib_c) = pkg.lib_c_type {
                matches_foojay_libc_type(pkg_lib_c)
            } else {
                false
            }
        }) {
            return Some((*pkg).clone());
        }

        // If no exact lib_c_type match, return the first one (mimics install behavior)
        packages_to_search.first().cloned().cloned()
    }

    fn matches_package(&self, package: &JdkMetadata, request: &ParsedVersionRequest) -> bool {
        // Check version match
        if !package
            .version
            .matches_pattern(&request.version.to_string())
        {
            return false;
        }

        // Apply platform filters if set
        if let Some(ref arch) = self.platform_filter.architecture {
            if package.architecture.to_string() != *arch {
                return false;
            }
        }

        if let Some(ref os) = self.platform_filter.operating_system {
            if package.operating_system.to_string() != *os {
                return false;
            }
        }

        if let Some(ref lib_c) = self.platform_filter.lib_c_type {
            if let Some(ref pkg_lib_c) = package.lib_c_type {
                if pkg_lib_c != lib_c {
                    return false;
                }
            } else {
                // Package doesn't specify lib_c_type, skip it if we're filtering
                return false;
            }
        }

        true
    }
}

/// Get current platform information
pub fn get_current_platform() -> (String, String, String) {
    let arch = get_current_architecture();
    let os = get_current_os();
    let lib_c_type = get_foojay_libc_type();
    (arch, os, lib_c_type.to_string())
}

fn get_current_architecture() -> String {
    #[cfg(target_arch = "x86_64")]
    return "x64".to_string();

    #[cfg(target_arch = "x86")]
    return "x86".to_string();

    #[cfg(target_arch = "aarch64")]
    return "aarch64".to_string();

    #[cfg(target_arch = "arm")]
    return "arm32".to_string();

    #[cfg(target_arch = "powerpc64")]
    {
        #[cfg(target_endian = "little")]
        return "ppc64le".to_string();
        #[cfg(target_endian = "big")]
        return "ppc64".to_string();
    }

    #[cfg(target_arch = "s390x")]
    return "s390x".to_string();

    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "x86",
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "powerpc64",
        target_arch = "s390x"
    )))]
    return "unknown".to_string();
}

fn get_current_os() -> String {
    #[cfg(target_os = "linux")]
    return "linux".to_string();

    #[cfg(target_os = "windows")]
    return "windows".to_string();

    #[cfg(target_os = "macos")]
    return "macos".to_string();

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    return "unknown".to_string();
}

/// Load cache and create a searcher
pub fn create_searcher_with_cache() -> Result<(MetadataCache, PackageSearcher<'static>)> {
    let cache_path = get_cache_path()?;

    if !cache_path.exists() {
        return Ok((MetadataCache::new(), PackageSearcher::new(None)));
    }

    let cache = load_cache(&cache_path)?;
    // This is a bit tricky - we need to ensure the cache outlives the searcher
    // In practice, the caller will need to manage this lifetime
    Ok((cache, PackageSearcher::new(None)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::{DistributionCache, MetadataCache};
    use crate::models::jdk::{
        Architecture, ArchiveType, ChecksumType, OperatingSystem, PackageType, Version,
    };

    fn create_test_cache() -> MetadataCache {
        let mut cache = MetadataCache::new();

        let mut packages = Vec::new();
        packages.push(JdkMetadata {
            id: "test-21".to_string(),
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            distribution_version: "21.0.1".to_string(),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            archive_type: ArchiveType::TarGz,
            download_url: "https://example.com/jdk21.tar.gz".to_string(),
            checksum: None,
            checksum_type: Some(ChecksumType::Sha256),
            size: 100_000_000,
            lib_c_type: Some("glibc".to_string()),
            javafx_bundled: false,
        });

        packages.push(JdkMetadata {
            id: "test-17".to_string(),
            distribution: "temurin".to_string(),
            version: Version::new(17, 0, 9),
            distribution_version: "17.0.9".to_string(),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            archive_type: ArchiveType::TarGz,
            download_url: "https://example.com/jdk17.tar.gz".to_string(),
            checksum: None,
            checksum_type: Some(ChecksumType::Sha256),
            size: 90_000_000,
            lib_c_type: Some("glibc".to_string()),
            javafx_bundled: false,
        });

        let dist_cache = DistributionCache {
            distribution: Distribution::Temurin,
            display_name: "Eclipse Temurin".to_string(),
            packages,
        };

        cache
            .distributions
            .insert("temurin".to_string(), dist_cache);
        cache
    }

    #[test]
    fn test_search_by_major_version() {
        let cache = create_test_cache();
        let searcher = PackageSearcher::new(Some(&cache));

        let results = searcher.search("21").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].package.version.major, 21);
    }

    #[test]
    fn test_search_with_distribution() {
        let cache = create_test_cache();
        let searcher = PackageSearcher::new(Some(&cache));

        let results = searcher.search("temurin@17").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].package.version.major, 17);
        assert_eq!(results[0].distribution, "temurin");
    }

    #[test]
    fn test_search_with_platform_filter() {
        let cache = create_test_cache();
        let filter = PlatformFilter {
            architecture: Some("x64".to_string()),
            operating_system: Some("linux".to_string()),
            lib_c_type: Some("glibc".to_string()),
        };
        let searcher = PackageSearcher::new(Some(&cache)).with_platform_filter(filter);

        let results = searcher.search("17").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_find_exact_package() {
        let cache = create_test_cache();
        let searcher = PackageSearcher::new(Some(&cache));

        let package = searcher.find_exact_package(&Distribution::Temurin, "21.0.1", "x64", "linux");

        assert!(package.is_some());
        assert_eq!(package.unwrap().version.to_string(), "21.0.1");
    }
}
