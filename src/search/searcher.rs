//! Core search implementation for finding JDK packages.
//!
//! This module contains the main `PackageSearcher` struct that implements
//! all search strategies described in the parent module documentation.

use crate::cache::{DistributionCache, MetadataCache};
use crate::error::Result;
use crate::models::jdk::{Distribution, JdkMetadata};
use crate::platform::matches_foojay_libc_type;
use crate::version::parser::{ParsedVersionRequest, VersionParser};

use super::models::{PlatformFilter, SearchResult, SearchResultRef};

/// Main search engine for finding JDK packages in cached metadata.
///
/// The searcher operates on a reference to a `MetadataCache` and provides
/// various search methods with different strategies and filters.
///
/// # Lifetime
///
/// The lifetime parameter `'a` is tied to the cache lifetime, ensuring
/// the searcher cannot outlive the cache it references.
///
/// # Example
///
/// ```no_run
/// # use kopi::search::{PackageSearcher, PlatformFilter};
/// # use kopi::cache::MetadataCache;
/// # let cache = MetadataCache::new();
/// // Create a searcher with platform filters
/// let filter = PlatformFilter {
///     architecture: Some("x64".to_string()),
///     operating_system: Some("linux".to_string()),
///     lib_c_type: None,
/// };
/// 
/// let searcher = PackageSearcher::new(Some(&cache))
///     .with_platform_filter(filter);
///
/// // Search for Java 21
/// let results = searcher.search("21").unwrap();
/// ```
pub struct PackageSearcher<'a> {
    cache: Option<&'a MetadataCache>,
    platform_filter: PlatformFilter,
}

impl<'a> PackageSearcher<'a> {
    /// Creates a new searcher with an optional cache reference.
    ///
    /// If `cache` is `None`, all searches will return empty results.
    pub fn new(cache: Option<&'a MetadataCache>) -> Self {
        Self {
            cache,
            platform_filter: PlatformFilter::default(),
        }
    }

    /// Configures platform-specific filters for the searcher.
    ///
    /// This method consumes and returns `self` for method chaining.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use kopi::search::{PackageSearcher, PlatformFilter};
    /// # use kopi::cache::MetadataCache;
    /// # let cache = MetadataCache::new();
    /// let searcher = PackageSearcher::new(Some(&cache))
    ///     .with_platform_filter(PlatformFilter {
    ///         architecture: Some("aarch64".to_string()),
    ///         operating_system: Some("macos".to_string()),
    ///         lib_c_type: None,
    ///     });
    /// ```
    pub fn with_platform_filter(mut self, filter: PlatformFilter) -> Self {
        self.platform_filter = filter;
        self
    }

    /// Common search logic that can be used with different result transformations.
    ///
    /// This is the core search algorithm that:
    /// 1. Iterates through all distributions in the cache
    /// 2. Applies distribution and package type filters
    /// 3. Handles "latest" version selection per distribution
    /// 4. Applies platform filters via `matches_package_with_version`
    /// 5. Transforms results using the provided `result_builder`
    ///
    /// The generic design allows the same logic to produce both owned
    /// results (`SearchResult`) and borrowed results (`SearchResultRef`).
    fn search_common<'b, F, R>(
        &'b self,
        request: &ParsedVersionRequest,
        result_builder: F,
    ) -> Result<Vec<R>>
    where
        'a: 'b,
        F: Fn(&'b str, &'b DistributionCache, &'b JdkMetadata) -> R,
    {
        let cache = match self.cache {
            Some(cache) => cache,
            None => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        // Pre-compute version string if needed to avoid repeated conversions
        let version_str = request.version.as_ref().map(|v| v.to_string());

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
                    if !self.matches_package_with_version(package, request, version_str.as_deref())
                    {
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
                    if !self.matches_package_with_version(package, request, version_str.as_deref())
                    {
                        continue;
                    }

                    results.push(result_builder(dist_name, dist_cache, package));
                }
            }
        }

        // Sort by distribution and version
        Ok(results)
    }

    /// Search for packages matching a version string.
    ///
    /// This is the primary search method that parses the version string
    /// and returns owned results suitable for external use.
    ///
    /// # Version String Format
    ///
    /// - `"21"` - Major version only
    /// - `"21.0.1"` - Specific version
    /// - `"temurin@21"` - Distribution and version
    /// - `"latest"` - Latest version across all distributions
    /// - `"temurin@latest"` - Latest version of specific distribution
    /// - `"jdk@21"` or `"jre@21"` - Package type prefix
    ///
    /// # Errors
    ///
    /// Returns an error if the version string format is invalid.
    pub fn search(&self, version_string: &str) -> Result<Vec<SearchResult>> {
        let parsed_request = VersionParser::parse(version_string)?;
        self.search_parsed(&parsed_request)
    }

    /// Search for packages matching a parsed version request.
    ///
    /// This method accepts a pre-parsed request and returns owned results.
    /// Results are sorted by distribution name, then by version (descending).
    pub fn search_parsed(&self, request: &ParsedVersionRequest) -> Result<Vec<SearchResult>> {
        let mut results =
            self.search_common(request, |dist_name, dist_cache, package| SearchResult {
                distribution: dist_name.to_string(),
                display_name: dist_cache.display_name.clone(),
                package: package.clone(),
            })?;

        // Sort by distribution and version
        results.sort_by(|a, b| match a.distribution.cmp(&b.distribution) {
            std::cmp::Ordering::Equal => b.package.version.cmp(&a.package.version),
            other => other,
        });

        Ok(results)
    }

    /// Search for packages and return references to avoid cloning
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

    /// Find an exact package match for installation.
    ///
    /// This method looks for a package that exactly matches all specified
    /// criteria. It's typically used when the user has specified exact
    /// requirements for installation.
    ///
    /// # Parameters
    ///
    /// - `distribution` - The JDK distribution (e.g., Temurin, Zulu)
    /// - `version` - Exact version string (e.g., "21.0.1")
    /// - `architecture` - Target architecture (e.g., "x64", "aarch64")
    /// - `operating_system` - Target OS (e.g., "linux", "windows", "macos")
    ///
    /// # Returns
    ///
    /// Returns `Some(JdkMetadata)` if an exact match is found, `None` otherwise.
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

    /// Determine which package would be auto-selected by the install command.
    ///
    /// This method implements the auto-selection strategy used when multiple
    /// packages match the basic criteria (version, architecture, OS). It's
    /// particularly important for handling cases where both JDK and JRE
    /// packages are available, or when multiple libc variants exist.
    ///
    /// # Selection Priority
    ///
    /// 1. If only one package matches, return it
    /// 2. Apply package type preference:
    ///    - If type explicitly requested, filter to that type
    ///    - Otherwise, prefer JDK over JRE
    /// 3. Try to match system libc type (Linux only)
    /// 4. Return the first remaining match
    ///
    /// # Parameters
    ///
    /// - `distribution` - The JDK distribution
    /// - `version` - Exact version string
    /// - `architecture` - Target architecture
    /// - `operating_system` - Target OS
    /// - `requested_package_type` - Optional explicit package type preference
    ///
    /// # Returns
    ///
    /// Returns the auto-selected package, or `None` if no matches found.
    pub fn find_auto_selected_package(
        &self,
        distribution: &Distribution,
        version: &str,
        architecture: &str,
        operating_system: &str,
        requested_package_type: Option<crate::models::jdk::PackageType>,
    ) -> Option<JdkMetadata> {
        let cache = self.cache?;

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

    /// Optimized version that accepts pre-computed version string
    fn matches_package_with_version(
        &self,
        package: &JdkMetadata,
        request: &ParsedVersionRequest,
        version_str: Option<&str>,
    ) -> bool {
        // Check version match if version is specified
        if let Some(version_pattern) = version_str {
            if !package.version.matches_pattern(version_pattern) {
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
