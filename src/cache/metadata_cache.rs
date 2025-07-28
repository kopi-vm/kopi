use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use crate::error::Result;
use crate::models::distribution::Distribution as JdkDistribution;
use crate::models::metadata::JdkMetadata;
use crate::models::package::PackageType;
use crate::version::parser::ParsedVersionRequest;

use super::models::{PlatformFilter, SearchResult, VersionSearchType};

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
        super::storage::save_cache(self, path)
    }

    /// Get the canonical name for a distribution from the synonym map
    /// Returns None if not found
    pub fn get_canonical_name(&self, name: &str) -> Option<&str> {
        self.synonym_map.get(name).map(|s| s.as_str())
    }

    /// Search for packages matching the given request
    pub fn search(
        &self,
        request: &ParsedVersionRequest,
        version_type: VersionSearchType,
    ) -> Result<Vec<SearchResult>> {
        let platform_filter = PlatformFilter::default();
        let mut results = Vec::new();

        // Pre-compute version string if needed to avoid repeated conversions
        let version_str = request.version.as_ref().map(|v| v.to_string());

        // Determine actual version type to use
        let actual_version_type = match version_type {
            VersionSearchType::Auto => {
                if let Some(ref v_str) = version_str {
                    Self::detect_version_type(v_str)
                } else {
                    VersionSearchType::JavaVersion
                }
            }
            other => other,
        };

        for (dist_name, dist_cache) in &self.distributions {
            // Filter by distribution if specified
            if let Some(ref target_dist) = request.distribution {
                if dist_cache.distribution != *target_dist {
                    continue;
                }
            }

            if request.latest {
                // For "latest" requests, find the highest version per distribution
                let mut latest_package: Option<&JdkMetadata> = None;

                for package in &dist_cache.packages {
                    // Apply package type filter if specified
                    if let Some(ref package_type) = request.package_type {
                        if package.package_type != *package_type {
                            continue;
                        }
                    }

                    // Apply platform filters
                    if !self.matches_package(
                        package,
                        request,
                        version_str.as_deref(),
                        actual_version_type,
                        &platform_filter,
                    ) {
                        continue;
                    }

                    // Track the latest version
                    match latest_package {
                        None => latest_package = Some(package),
                        Some(current_latest) => {
                            if package.version > current_latest.version {
                                latest_package = Some(package);
                            }
                        }
                    }
                }

                if let Some(package) = latest_package {
                    results.push(SearchResult {
                        distribution: dist_name.to_string(),
                        display_name: dist_cache.display_name.clone(),
                        package: package.clone(),
                    });
                }
            } else {
                // Regular search - include all matching versions
                for package in &dist_cache.packages {
                    if !self.matches_package(
                        package,
                        request,
                        version_str.as_deref(),
                        actual_version_type,
                        &platform_filter,
                    ) {
                        continue;
                    }

                    results.push(SearchResult {
                        distribution: dist_name.to_string(),
                        display_name: dist_cache.display_name.clone(),
                        package: package.clone(),
                    });
                }
            }
        }

        // Sort by distribution and version
        results.sort_by(|a, b| match a.distribution.cmp(&b.distribution) {
            std::cmp::Ordering::Equal => b.package.version.cmp(&a.package.version),
            other => other,
        });

        Ok(results)
    }

    /// Auto-detect whether to search by java_version or distribution_version
    pub fn detect_version_type(version_str: &str) -> VersionSearchType {
        // If the version has 4+ components, likely a distribution_version
        let component_count = version_str.split('.').count();
        if component_count >= 4 {
            return VersionSearchType::DistributionVersion;
        }

        // If it contains non-numeric build identifiers after +, likely distribution_version
        if let Some(plus_pos) = version_str.find('+') {
            let build_part = &version_str[plus_pos + 1..];
            // Check if build part contains non-numeric characters or multiple components
            if build_part.contains('.') || build_part.chars().any(|c| !c.is_ascii_digit()) {
                return VersionSearchType::DistributionVersion;
            }
        }

        // Default to java_version for standard formats
        VersionSearchType::JavaVersion
    }

    /// Look up a specific package by distribution, version, and platform
    pub fn lookup(
        &self,
        distribution: &JdkDistribution,
        version: &str,
        architecture: &str,
        operating_system: &str,
        package_type: Option<&PackageType>,
        javafx_bundled: Option<bool>,
    ) -> Option<JdkMetadata> {
        // Look up distribution by its API name, resolving synonyms
        let canonical_name = self
            .get_canonical_name(distribution.id())
            .unwrap_or(distribution.id());
        let dist_cache = self.distributions.get(canonical_name)?;

        // Find exact match
        dist_cache
            .packages
            .iter()
            .find(|pkg| {
                pkg.version.matches_pattern(version)
                    && pkg.architecture.to_string() == architecture
                    && pkg.operating_system.to_string() == operating_system
                    && (package_type.is_none() || Some(&pkg.package_type) == package_type)
                    && (javafx_bundled.is_none() || Some(pkg.javafx_bundled) == javafx_bundled)
                    && self.matches_platform_libc(&pkg.lib_c_type)
            })
            .cloned()
    }

    /// Check if the package's lib_c_type is compatible with the current platform
    fn matches_platform_libc(&self, lib_c_type: &Option<String>) -> bool {
        match lib_c_type {
            None => true, // If no lib_c_type specified, assume it's compatible
            Some(libc) => crate::platform::matches_foojay_libc_type(libc),
        }
    }

    fn matches_package(
        &self,
        package: &JdkMetadata,
        request: &ParsedVersionRequest,
        version_str: Option<&str>,
        version_type: VersionSearchType,
        platform_filter: &PlatformFilter,
    ) -> bool {
        // Check version match if version is specified
        if let Some(version_pattern) = version_str {
            let matches = match version_type {
                VersionSearchType::JavaVersion => package.version.matches_pattern(version_pattern),
                VersionSearchType::DistributionVersion => {
                    // Use Version's matches_pattern method for distribution_version
                    package
                        .distribution_version
                        .matches_pattern(version_pattern)
                }
                VersionSearchType::Auto => {
                    // This shouldn't happen as Auto is resolved earlier, but handle it
                    package.version.matches_pattern(version_pattern)
                }
            };

            if !matches {
                return false;
            }
        }

        // Check package type if specified
        if let Some(ref package_type) = request.package_type {
            if package.package_type != *package_type {
                return false;
            }
        }

        // Apply platform filters if set
        if let Some(ref arch) = platform_filter.architecture {
            if package.architecture.to_string() != *arch {
                return false;
            }
        }

        if let Some(ref os) = platform_filter.operating_system {
            if package.operating_system.to_string() != *os {
                return false;
            }
        }

        if let Some(ref lib_c) = platform_filter.lib_c_type {
            if let Some(ref pkg_lib_c) = package.lib_c_type {
                if pkg_lib_c != lib_c {
                    return false;
                }
            } else {
                // Package doesn't specify lib_c_type, skip it if we're filtering
                return false;
            }
        } else {
            // No explicit lib_c_type filter, but we should still check platform compatibility
            if !self.matches_platform_libc(&package.lib_c_type) {
                return false;
            }
        }

        true
    }
}
