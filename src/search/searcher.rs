use crate::cache::{DistributionCache, MetadataCache};
use crate::config::KopiConfig;
use crate::error::Result;
use crate::models::distribution::Distribution;
use crate::models::metadata::JdkMetadata;
use crate::platform::matches_foojay_libc_type;
use crate::version::parser::{ParsedVersionRequest, VersionParser};

use super::models::{PlatformFilter, SearchResult, SearchResultRef, VersionSearchType};

pub struct PackageSearcher<'a> {
    cache: &'a MetadataCache,
    platform_filter: PlatformFilter,
    config: &'a KopiConfig,
}

impl<'a> PackageSearcher<'a> {
    pub fn new(cache: &'a MetadataCache, config: &'a KopiConfig) -> Self {
        Self {
            cache,
            platform_filter: PlatformFilter::default(),
            config,
        }
    }

    pub fn with_platform_filter(mut self, filter: PlatformFilter) -> Self {
        self.platform_filter = filter;
        self
    }

    fn search_common<'b, F, R>(
        &'b self,
        request: &ParsedVersionRequest,
        result_builder: F,
    ) -> Result<Vec<R>>
    where
        'a: 'b,
        F: Fn(&'b str, &'b DistributionCache, &'b JdkMetadata) -> R,
    {
        self.search_common_with_type(request, VersionSearchType::Auto, result_builder)
    }

    fn search_common_with_type<'b, F, R>(
        &'b self,
        request: &ParsedVersionRequest,
        version_type: VersionSearchType,
        result_builder: F,
    ) -> Result<Vec<R>>
    where
        'a: 'b,
        F: Fn(&'b str, &'b DistributionCache, &'b JdkMetadata) -> R,
    {
        let cache = self.cache;

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

        for (dist_name, dist_cache) in &cache.distributions {
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
                    if !self.matches_package_with_version_type(
                        package,
                        request,
                        version_str.as_deref(),
                        actual_version_type,
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
                    results.push(result_builder(dist_name, dist_cache, package));
                }
            } else {
                // Regular search - include all matching versions
                for package in &dist_cache.packages {
                    if !self.matches_package_with_version_type(
                        package,
                        request,
                        version_str.as_deref(),
                        actual_version_type,
                    ) {
                        continue;
                    }

                    results.push(result_builder(dist_name, dist_cache, package));
                }
            }
        }

        // Sort by distribution and version
        Ok(results)
    }

    pub fn search(&self, version_string: &str) -> Result<Vec<SearchResult>> {
        let parser = VersionParser::new(self.config);
        let parsed_request = parser.parse(version_string)?;
        self.search_parsed(&parsed_request)
    }

    pub fn search_parsed(&self, request: &ParsedVersionRequest) -> Result<Vec<SearchResult>> {
        self.search_parsed_with_type(request, VersionSearchType::Auto)
    }

    pub fn search_parsed_with_type(
        &self,
        request: &ParsedVersionRequest,
        version_type: VersionSearchType,
    ) -> Result<Vec<SearchResult>> {
        let mut results = self.search_common_with_type(
            request,
            version_type,
            |dist_name, dist_cache, package| SearchResult {
                distribution: dist_name.to_string(),
                display_name: dist_cache.display_name.clone(),
                package: package.clone(),
            },
        )?;

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

    pub fn search_parsed_refs<'b>(
        &'b self,
        request: &ParsedVersionRequest,
    ) -> Result<Vec<SearchResultRef<'b>>>
    where
        'a: 'b,
    {
        let mut results =
            self.search_common(request, |dist_name, dist_cache, package| SearchResultRef {
                distribution: dist_name,
                display_name: &dist_cache.display_name,
                package,
            })?;

        // Sort by distribution and version
        results.sort_by(|a, b| match a.distribution.cmp(b.distribution) {
            std::cmp::Ordering::Equal => b.package.version.cmp(&a.package.version),
            other => other,
        });

        Ok(results)
    }

    pub fn find_exact_package(
        &self,
        distribution: &Distribution,
        version: &str,
        architecture: &str,
        operating_system: &str,
        package_type: Option<&crate::models::package::PackageType>,
    ) -> Option<JdkMetadata> {
        let cache = self.cache;

        // Look up distribution by its API name, resolving synonyms
        let canonical_name = cache
            .get_canonical_name(distribution.id())
            .unwrap_or(distribution.id());
        let dist_cache = cache.distributions.get(canonical_name)?;

        // Find exact match
        dist_cache
            .packages
            .iter()
            .find(|pkg| {
                pkg.version.matches_pattern(version)
                    && pkg.architecture.to_string() == architecture
                    && pkg.operating_system.to_string() == operating_system
                    && (package_type.is_none() || Some(&pkg.package_type) == package_type)
            })
            .cloned()
    }

    pub fn find_auto_selected_package(
        &self,
        distribution: &Distribution,
        version: &str,
        architecture: &str,
        operating_system: &str,
        requested_package_type: Option<crate::models::package::PackageType>,
    ) -> Option<JdkMetadata> {
        let cache = self.cache;

        // Look up distribution by its API name, resolving synonyms
        let canonical_name = cache
            .get_canonical_name(distribution.id())
            .unwrap_or(distribution.id());
        let dist_cache = cache.distributions.get(canonical_name)?;

        // Find packages matching version, arch, and OS
        let matching_packages: Vec<&JdkMetadata> = dist_cache
            .packages
            .iter()
            .filter(|pkg| {
                pkg.version.matches_pattern(version)
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
                .filter(|pkg| pkg.package_type == crate::models::package::PackageType::Jdk)
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

    fn matches_package_with_version_type(
        &self,
        package: &JdkMetadata,
        request: &ParsedVersionRequest,
        version_str: Option<&str>,
        version_type: VersionSearchType,
    ) -> bool {
        // Check version match if version is specified
        if let Some(version_pattern) = version_str {
            let matches = match version_type {
                VersionSearchType::JavaVersion => package.version.matches_pattern(version_pattern),
                VersionSearchType::DistributionVersion => {
                    // For distribution_version, we do a simple string comparison
                    // supporting partial matching (e.g., "21.0.7.6" matches "21.0.7.6.1")
                    if package.distribution_version.starts_with(version_pattern) {
                        // Ensure we match at component boundaries
                        let dist_ver = &package.distribution_version;
                        dist_ver.len() == version_pattern.len()
                            || dist_ver.chars().nth(version_pattern.len()) == Some('.')
                    } else {
                        false
                    }
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
